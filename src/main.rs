#![allow(arithmetic_overflow)]

mod lexer;
mod exec;
mod compile;

use crate::exec::interpret_tokens;
use crate::lexer::tokenize;

const DATA_LENGTH: usize = 30_000;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum SyscallArgType {
    Regular,
    Pointer,
    CellPointer,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    simple_logger::init()?;
    let tokens = tokenize(&std::fs::read_to_string(std::env::args().collect::<Vec<String>>().last().expect("No program was supplied!")).expect("Failed to read program!")).unwrap();
    interpret_tokens(&tokens)
}