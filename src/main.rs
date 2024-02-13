#![allow(arithmetic_overflow)]

mod lexer;

use std::io::{Read, stdin, stdout, Write};
use std::os::raw::{c_int, c_void};
use log::{debug, error, trace};
use crate::lexer::{Token, tokenize};

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

const DATA_LENGTH: usize = 30_000;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum SyscallArgType {
    Regular,
    Pointer,
    CellPointer,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    simple_logger::init()?;

    // Read args

    // Setup variables
    let mut data_pointer = 0usize;
    let mut instruction_pointer = 0usize;
    let mut data = vec![0u8; DATA_LENGTH];
    // Read in tokens
    let tokens = tokenize(&std::fs::read_to_string(std::env::args().collect::<Vec<String>>().last().expect("No program was supplied!")).expect("Failed to read program!")).unwrap();
    // Interpret it
    while instruction_pointer != tokens.len() {
        if let Some(token) = tokens.get(instruction_pointer) {
            match token {
                Token::Idp => { data_pointer += 1 }
                Token::Ddp => { data_pointer -= 1 }
                Token::Inc => { data[data_pointer] += 1 }
                Token::Dec => { data[data_pointer] -= 1 }
                Token::Out => {
                    let _ = stdout().write(&data[data_pointer..data_pointer + 1]).expect("Failed to write to STDOUT!");
                }
                Token::Acc => {
                    let _ = stdin().read(&mut data[data_pointer..data_pointer + 1]).expect("Failed to read from STDIN!");
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
                            _ => { panic!("INVALID SYSCALL ARG TYPE: {}", data[local_offset]) }
                        };
                        let l = data[local_offset + 1] as usize;
                        let b = data[local_offset + 2..].as_mut_ptr();
                        arguments.push((t, l, b));
                        local_offset += 2 + l;
                    }
                    trace!("Performing SYSCALL[{code}] wth args: {arguments:?}");
                    // Parse arguments to actual values
                    let arguments = arguments.iter().map(|(syscall_arg_type, length, bytes)| match syscall_arg_type {
                        SyscallArgType::Regular => {
                            let mut buf = [0; std::mem::size_of::<usize>()];
                            unsafe { std::ptr::copy_nonoverlapping(*bytes as *const u8, buf[(std::mem::size_of::<usize>() - *length)..].as_mut_ptr(), *length) };
                            usize::from_be_bytes(buf)
                        }
                        SyscallArgType::Pointer => {
                            *bytes as usize
                        }
                        SyscallArgType::CellPointer => {
                            let index = {
                                let mut buf = [0; std::mem::size_of::<usize>()];
                                unsafe { std::ptr::copy_nonoverlapping(*bytes as *const u8, buf[(std::mem::size_of::<usize>() - *length)..].as_mut_ptr(), *length) };
                                usize::from_be_bytes(buf)
                            };
                            (data.as_ptr() as *const c_void) as usize + index
                        }
                    }).collect::<Vec<usize>>();
                    // Call
                    let sys = dynamic_syscall(code as c_int, arguments.as_slice());
                    if sys == -1 {
                        error!("{:?}", std::io::Error::last_os_error())
                    }
                    trace!("Dumping value {sys:?} to data[{data_pointer}]");
                    data[data_pointer] = sys as u8;
                }
            }
        }
        instruction_pointer += 1;
    }
    Ok(())
}