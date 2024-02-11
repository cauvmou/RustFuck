#![allow(arithmetic_overflow)]

use std::io::{Read, stdin, stdout, Write};
use std::os::raw::{c_int, c_void};
use log::{debug, error, trace};

extern "C" {
    fn syscall(num: c_int, ...) -> c_int;
}

fn preserving_syscall(syscall_number: c_int, args: &[usize], regs: &mut [usize]) -> isize {
    unsafe {
        std::ptr::copy_nonoverlapping(args.as_ptr(), regs.as_mut_ptr(), args.len()); // Preserve registers between syscalls.
        trace!("SYSCALL[{syscall_number}] {regs:?}");
        syscall(syscall_number, regs[0], regs[1], regs[2], regs[3], regs[4], regs[5]) as isize
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
enum Token {
    Idp,
    Ddp,
    Inc,
    Dec,
    Out,
    Acc,
    Jfw {
        instruction_ref: usize
    },
    Jbw {
        instruction_ref: usize
    },
    Sys,
}

#[derive(Debug, Copy, Clone, PartialEq)]
enum SyscallArgType {
    Regular,
    Pointer,
    CellPointer,
}

fn main() -> Result<(), Box<dyn std::error::Error>>{
    simple_logger::init()?;

    // Read args

    // Setup variables
    let mut data_pointer = 0usize;
    let mut instruction_pointer = 0usize;
    let mut data = vec![0u8; 30_000];
    let mut sys_registers = [0usize; 6];
    // Read in tokens
    let mut depth = 0usize;
    let mut tokens = std::fs::read_to_string(std::env::args().collect::<Vec<String>>().last().expect("No program was supplied!")).expect("Failed to read program!")
        .chars().filter_map(|c| match c {
        '>' => Some(Token::Idp),
        '<' => Some(Token::Ddp),
        '+' => Some(Token::Inc),
        '-' => Some(Token::Dec),
        '.' => Some(Token::Out),
        ',' => Some(Token::Acc),
        '[' => Some(Token::Jfw {
            instruction_ref: {
                let d = depth;
                depth += 1;
                d
            }
        }),
        ']' => Some(Token::Jbw {
            instruction_ref: {
                depth -= 1;
                depth
            }
        }),
        '%' => Some(Token::Sys),
        _ => None
    }).collect::<Vec<Token>>();
    // Cross reference jumps
    let tokens_clone = tokens.clone();
    tokens.iter_mut().enumerate().for_each(|(index, t)| if let Token::Jfw { instruction_ref } = t {
        let r = *instruction_ref;
        *instruction_ref = tokens_clone.iter().enumerate().position(|(i, t)| match t {
            Token::Jbw { instruction_ref } => { *instruction_ref == r && i > index }
            _ => false
        }).unwrap_or_else(|| panic!("No closing bracket for '[' at {} (NOTE: Index represents the nth instruction, this may not be the actual character!)", index + 1));
    });
    let tokens_clone = tokens.clone();
    tokens.iter_mut().enumerate().for_each(|(index, t)| if let Token::Jbw { instruction_ref } = t {
        *instruction_ref = tokens_clone.iter().position(|t| match t {
            Token::Jfw { instruction_ref } => { *instruction_ref == index }
            _ => false
        }).unwrap_or_else(|| panic!("No opening bracket for ']' at {} (NOTE: Index represents the nth instruction, this may not be the actual character!)", index + 1));
    });
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
                    let code = data[data_pointer + 0] as usize;
                    let arg_count = data[data_pointer + 1] as usize;
                    let mut arguments: Vec<(SyscallArgType, usize, *mut u8)> = Vec::new();
                    let mut local_offset = data_pointer + 2;
                    for _ in 0..arg_count {
                        let t = match data[local_offset + 0] {
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
                    let arguments = arguments.iter().map(|(t, l, b)| match t {
                        SyscallArgType::Regular => {
                            let mut buf = [0; std::mem::size_of::<usize>()];
                            unsafe { std::ptr::copy_nonoverlapping(*b as *const u8, buf[(std::mem::size_of::<usize>() - *l)..].as_mut_ptr(), *l) };
                            usize::from_be_bytes(buf)
                        }
                        SyscallArgType::Pointer => {
                            let ptr = *b as usize;
                            let v = ptr as *const c_void;
                            let mut array = [0u8; 16];
                            unsafe { std::ptr::copy_nonoverlapping(v as *const u8, array.as_mut_ptr(), 16) };
                            debug!("{array:?}");
                            ptr
                        }
                        SyscallArgType::CellPointer => {
                            let index = {
                                let mut buf = [0; std::mem::size_of::<usize>()];
                                unsafe { std::ptr::copy_nonoverlapping(*b as *const u8, buf[(std::mem::size_of::<usize>() - *l)..].as_mut_ptr(), *l) };
                                usize::from_be_bytes(buf)
                            };
                            (data.as_ptr() as *const c_void) as usize + index
                        }
                    }).collect::<Vec<usize>>();
                    trace!("ENCODED: {arguments:?}");
                    // Call
                    let sys = preserving_syscall(code as c_int, arguments.as_slice(), &mut sys_registers);
                    trace!("RESULT: {sys}");
                    if sys == -1 {
                        error!("{:?}", std::io::Error::last_os_error())
                    }
                    let ret = sys.to_ne_bytes();
                    unsafe { std::ptr::copy_nonoverlapping(ret.as_ptr(), data[data_pointer..].as_mut_ptr(), ret.len()) };
                }
            }
        }
        instruction_pointer += 1;
    }
    Ok(())
}