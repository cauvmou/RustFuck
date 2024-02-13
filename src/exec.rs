use std::error::Error;
use std::fmt::{Display, Formatter};
use std::io::{Read, stdin, stdout, Write};
use std::os::raw::{c_int, c_void};
use log::{error, trace};
use crate::{DATA_LENGTH, SyscallArgType};
use crate::lexer::Token;

extern "C" {
    fn syscall(num: c_int, ...) -> c_int;
}

fn dynamic_syscall(syscall_number: c_int, args: &[usize]) -> isize {
    unsafe {
        let mut regs = [0; 6];
        std::ptr::copy_nonoverlapping(args.as_ptr(), regs.as_mut_ptr(), args.len());
        syscall(syscall_number, regs[0], regs[1], regs[2], regs[3], regs[4], regs[5]) as isize
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum RuntimeError {
    InvalidSyscallArgumentType {
        arg_type: u8
    }
}

impl Display for RuntimeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl Error for RuntimeError {}

pub fn interpret_tokens(tokens: &[Token]) -> Result<(), Box<dyn std::error::Error>> {
    let mut data_pointer = 0usize;
    let mut instruction_pointer = 0usize;
    let mut data = vec![0u8; DATA_LENGTH];

    while instruction_pointer != tokens.len() {
        if let Some(token) = tokens.get(instruction_pointer) {
            match token {
                Token::Idp => { data_pointer += 1 }
                Token::Ddp => { data_pointer -= 1 }
                Token::Inc => { data[data_pointer] += 1 }
                Token::Dec => { data[data_pointer] -= 1 }
                Token::Out => {
                    trace!("writing to stdout");
                    let _ = stdout().write(&data[data_pointer..data_pointer + 1])?;
                }
                Token::Acc => {
                    trace!("reading from stdin");
                    let _ = stdin().read(&mut data[data_pointer..data_pointer + 1])?;
                }
                Token::Jfw { instruction_ref } => { instruction_pointer = if data[data_pointer] == 0 { *instruction_ref } else { instruction_pointer } }
                Token::Jbw { instruction_ref } => { instruction_pointer = if data[data_pointer] != 0 { *instruction_ref } else { instruction_pointer } }
                Token::Sys => {
                    // Extract arguments for call
                    let code = data[data_pointer] as usize;
                    let arg_count = data[data_pointer + 1] as usize;
                    let mut arguments: Vec<(SyscallArgType, usize, *mut u8)> = Vec::new();
                    let mut local_offset = data_pointer + 2;
                    for _ in 0..arg_count {
                        let t = match data[local_offset] {
                            0 => SyscallArgType::Regular,
                            1 => SyscallArgType::Pointer,
                            2 => SyscallArgType::CellPointer,
                            arg_type => { return Err(Box::new(RuntimeError::InvalidSyscallArgumentType { arg_type })) }
                        };
                        let l = data[local_offset + 1] as usize;
                        let b = data[local_offset + 2..].as_mut_ptr();
                        arguments.push((t, l, b));
                        local_offset += 2 + l;
                    }
                    trace!("performing syscall[{code}] wth args: {arguments:?}");
                    // Parse arguments to actual values
                    let arguments = arguments.iter().map(|(syscall_arg_type, length, bytes)| match syscall_arg_type {
                        SyscallArgType::Regular => usize_from_cells(*bytes, *length),
                        SyscallArgType::Pointer => {
                            *bytes as usize
                        }
                        SyscallArgType::CellPointer => {
                            let index = usize_from_cells(*bytes, *length);
                            (data.as_ptr() as *const c_void) as usize + index
                        }
                    }).collect::<Vec<usize>>();
                    // Call
                    let sys = dynamic_syscall(code as c_int, arguments.as_slice());
                    if sys == -1 {
                        error!("{:?}", std::io::Error::last_os_error())
                    }
                    trace!("dumping value {sys:?} to data[{data_pointer}]");
                    data[data_pointer] = sys as u8;
                }
            }
        }
        instruction_pointer += 1;
    }
    Ok(())
}

fn usize_from_cells(bytes: *mut u8, length: usize) -> usize {
    let mut buf = [0; std::mem::size_of::<usize>()];
    unsafe { std::ptr::copy_nonoverlapping(bytes as *const u8, buf[(std::mem::size_of::<usize>() - length)..].as_mut_ptr(), length) };
    usize::from_be_bytes(buf)
}