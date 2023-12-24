use std::io::{Read, stdin, stdout, Write};

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

fn main() {
    let mut data_pointer = 0usize;
    let mut instruction_pointer = 0usize;
    let mut data = vec![0i8; u32::MAX as usize];
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
                Token::OUT => { let buf = [data[data_pointer] as u8]; stdout().write(&buf).expect("Failed to write to STDOUT!"); }
                Token::ACC => { let mut buf = [0; 1]; stdin().read(&mut buf).expect("Failed to read from STDIN!"); data[data_pointer] = buf[0] as i8; }
                Token::JFW { instruction_ref } => { instruction_pointer = if data[data_pointer] == 0 { *instruction_ref } else { instruction_pointer } }
                Token::JBW { instruction_ref } => { instruction_pointer = if data[data_pointer] != 0 { *instruction_ref } else { instruction_pointer } }
                Token::SYS => { panic!("The '%' syscall instruction, at index {instruction_pointer}, is not supported in interpretation mode!") }
            }
        }
        instruction_pointer += 1;
    }
}
