#![allow(arithmetic_overflow)]

mod lexer;
mod exec;
mod compile;

use clap::Parser;
use crate::exec::interpret_tokens;
use crate::lexer::tokenize;

const DATA_LENGTH: usize = 30_000;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum SyscallArgType {
    Regular,
    Pointer,
    CellPointer,
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "Warn", value_names = ["Trace", "Debug", "Info", "Warn", "Error"], help = "Verbosity of the interpreter.")]
    log_level: log::Level,
    #[arg(short, long, default_value = "false", help = "If the inline flag is set, then the program will be treated as the code to be executed, else it is assumed to be a path to a systemf source code file.")]
    inline: bool,
    #[arg(last = true, help = "A path to a file or systemf code, depending on the inline flag.")]
    program: String,
}

struct Group {
    path: Option<String>,
    program: Option<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    simple_logger::init_with_level(args.log_level)?;
    let tokens = if args.inline {
        tokenize(&args.program)?
    } else {
        tokenize(&std::fs::read_to_string(args.program)?)?
    };
    interpret_tokens(&tokens)
}