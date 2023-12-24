use std::io::{Read, stdin, stdout, Write};
use std::os::raw::{c_int, c_void};

extern "C" {
    fn syscall(num: c_int, ...) -> c_int;
}

fn generic_syscall(syscall_number: c_int, args: &[usize]) -> isize {
    unsafe {
        match args.len() {
            0 => syscall(syscall_number) as isize,
            1 => syscall(syscall_number, args[0]) as isize,
            2 => syscall(syscall_number, args[0], args[1]) as isize,
            3 => syscall(syscall_number, args[0], args[1], args[2]) as isize,
            4 => syscall(syscall_number, args[0], args[1], args[2], args[3]) as isize,
            5 => syscall(syscall_number, args[0], args[1], args[2], args[3], args[4]) as isize,
            6 => syscall(syscall_number, args[0], args[1], args[2], args[3], args[4], args[5]) as isize,
            7 => syscall(syscall_number, args[0], args[1], args[2], args[3], args[4], args[5], args[6]) as isize,
            // Handle more arguments as needed
            _ => -1,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
enum Token {
    IDP,
    DDP,
    INC,
    DEC,
    OUT,
    ACC,
    JFW {
        instruction_ref: usize
    },
    JBW {
        instruction_ref: usize
    },
    SYS,
}

#[derive(Debug, Copy, Clone, PartialEq)]
enum SyscallArgType {
    Regular,
    Pointer,
    CellPointer,
}

fn main() {
    let mut data_pointer = 0usize;
    let mut instruction_pointer = 0usize;
    let mut data = vec![0u8; u32::MAX as usize];
    // Read in tokens
    let mut depth = 0usize;
    let mut tokens = std::fs::read_to_string(std::env::args().collect::<Vec<String>>().get(1).expect("No program was supplied!")).expect("Failed to read program!")
        .chars().map(|c| match c {
        '>' => Some(Token::IDP),
        '<' => Some(Token::DDP),
        '+' => Some(Token::INC),
        '-' => Some(Token::DEC),
        '.' => Some(Token::OUT),
        ',' => Some(Token::ACC),
        '[' => Some(Token::JFW {
            instruction_ref: {
                let d = depth;
                depth += 1;
                d
            }
        }),
        ']' => Some(Token::JBW {
            instruction_ref: {
                depth -= 1;
                depth
            }
        }),
        '%' => Some(Token::SYS),
        _ => None
    }).filter(|t| *t != None).map(|t| t.unwrap()).collect::<Vec<Token>>();
    // Cross reference jumps
    let tokens_clone = tokens.clone();
    tokens.iter_mut().enumerate().for_each(|(index, t)| match t {
        Token::JFW { instruction_ref } => {
            let r = *instruction_ref;
            *instruction_ref = tokens_clone.iter().enumerate().position(|(i, t)| match t {
                Token::JBW { instruction_ref } => { *instruction_ref == r && i > index }
                _ => false
            }).expect(format!("No closing bracket for '[' at {} (NOTE: Index represents the nth instruction, this may not be the actual character!)", index + 1).as_str());
        }
        _ => {}
    });
    let tokens_clone = tokens.clone();
    tokens.iter_mut().enumerate().for_each(|(index, t)| match t {
        Token::JBW { instruction_ref } => {
            *instruction_ref = tokens_clone.iter().position(|t| match t {
                Token::JFW { instruction_ref } => { *instruction_ref == index }
                _ => false
            }).expect(format!("No opening bracket for ']' at {} (NOTE: Index represents the nth instruction, this may not be the actual character!)", index + 1).as_str());
        }
        _ => {}
    });
    // Interpret it
    while instruction_pointer != tokens.len() {
        if let Some(token) = tokens.get(instruction_pointer) {
            match token {
                Token::IDP => { data_pointer += 1 }
                Token::DDP => { data_pointer -= 1 }
                Token::INC => { data[data_pointer] += 1 }
                Token::DEC => { data[data_pointer] -= 1 }
                Token::OUT => {
                    stdout().write(&data[data_pointer..data_pointer + 1]).expect("Failed to write to STDOUT!");
                }
                Token::ACC => {
                    stdin().read(&mut data[data_pointer..data_pointer + 1]).expect("Failed to read from STDIN!");
                }
                Token::JFW { instruction_ref } => { instruction_pointer = if data[data_pointer] == 0 { *instruction_ref } else { instruction_pointer } }
                Token::JBW { instruction_ref } => { instruction_pointer = if data[data_pointer] != 0 { *instruction_ref } else { instruction_pointer } }
                // TODO: Fix bug, where the webserver cannot read any traffic/bind to port.
                Token::SYS => {
                    let code = data[data_pointer + 0] as usize;
                    let arg_count = data[data_pointer + 1] as usize;
                    let mut arguments: Vec<(SyscallArgType, usize, &[u8])> = Vec::new();
                    let mut local_offset = data_pointer + 2;
                    for i in 0..arg_count {
                        let t = match data[local_offset + 0] {
                            0 => SyscallArgType::Regular,
                            1 => SyscallArgType::Pointer,
                            2 => SyscallArgType::CellPointer,
                            _ => { panic!("INVALID SYSCALL ARG TYPE: {}", data[local_offset]) }
                        };
                        let l = data[local_offset + 1] as usize;
                        let b = &data[local_offset + 2..local_offset + 2 + l];
                        arguments.push((t, l, b));
                        local_offset += 2 + l;
                    }
                    // println!("Performing SYSCALL[{code}] wth args: {arguments:?}");
                    let arguments = arguments.iter().map(|(t, l, b)| match t {
                        SyscallArgType::Regular => {
                            let mut buf = [0; std::mem::size_of::<usize>()];
                            unsafe { std::ptr::copy_nonoverlapping(b.as_ptr(), buf[(std::mem::size_of::<usize>() - *l)..].as_mut_ptr(), *l) };
                            usize::from_be_bytes(buf)
                        }
                        SyscallArgType::Pointer => { (b.as_ptr() as *const c_void) as usize }
                        SyscallArgType::CellPointer => {
                            let index = {
                                let mut buf = [0; std::mem::size_of::<usize>()];
                                unsafe { std::ptr::copy_nonoverlapping(b.as_ptr(), buf[(std::mem::size_of::<usize>() - *l)..].as_mut_ptr(), *l) };
                                usize::from_be_bytes(buf)
                            };
                            (data[index..].as_ptr() as *const c_void) as usize
                        }
                    }).collect::<Vec<usize>>();
                    // println!("------| ENCODED: {arguments:?}");
                    let ret = generic_syscall(code as c_int, arguments.as_slice());
                    for i in 0..std::mem::size_of::<usize>() {
                        data[data_pointer + i] = ((ret >> i) & 0xff) as u8;
                    }
                }
            }
        }
        instruction_pointer += 1;
    }
}
