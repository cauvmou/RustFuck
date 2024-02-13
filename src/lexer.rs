use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum RawToken {
    Idp(usize),
    Ddp(usize),
    Inc(usize),
    Dec(usize),
    Out(usize),
    Acc(usize),
    Jfw {
        pos: usize,
        depth: usize
    },
    Jbw {
        pos: usize,
        depth: usize
    },
    Sys(usize),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Token {
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

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum LexingError {
    MissingClosingBracket {
        pos: usize
    },
    MissingOpeningBracket {
        pos: usize
    }
}

impl Display for LexingError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl Error for LexingError {}

pub fn tokenize(program: &str) -> Result<Vec<Token>, LexingError> {
    link_raw_tokens(&(to_raw_tokens(program)))
}

fn to_raw_tokens(program: &str) -> Vec<RawToken> {
    let mut depth = 0usize;
    program
        .chars().enumerate().filter_map(|(index, c)| match c {
        '>' => Some(RawToken::Idp(index)),
        '<' => Some(RawToken::Ddp(index)),
        '+' => Some(RawToken::Inc(index)),
        '-' => Some(RawToken::Dec(index)),
        '.' => Some(RawToken::Out(index)),
        ',' => Some(RawToken::Acc(index)),
        '[' => Some(RawToken::Jfw {
            pos: index,
            depth: {
                let d = depth;
                depth += 1;
                d
            }
        }),
        ']' => Some(RawToken::Jbw {
            pos: index,
            depth: {
                depth -= 1;
                depth
            }
        }),
        '%' => Some(RawToken::Sys(index)),
        _ => None
    }).collect::<Vec<RawToken>>()
}

fn link_raw_tokens(raw_tokens: &[RawToken]) -> Result<Vec<Token>, LexingError> {
    raw_tokens.iter().enumerate().map(|(index, raw_token)| {
        match raw_token {
            RawToken::Idp(_) => {Ok(Token::Idp)}
            RawToken::Ddp(_) => {Ok(Token::Ddp)}
            RawToken::Inc(_) => {Ok(Token::Inc)}
            RawToken::Dec(_) => {Ok(Token::Dec)}
            RawToken::Out(_) => {Ok(Token::Out)}
            RawToken::Acc(_) => {Ok(Token::Acc)}
            RawToken::Jfw { pos, depth } => {
                if let Some(rel_instruction_ref) = raw_tokens[index..].iter().position(|other_token| {
                    match other_token {
                        RawToken::Jbw { pos: _, depth: other_depth } => *other_depth == *depth,
                        _ => false
                    }
                }) {
                    Ok(Token::Jfw {
                        instruction_ref: index + rel_instruction_ref,
                    })
                } else {
                    Err(LexingError::MissingClosingBracket { pos: *pos })
                }
            }
            RawToken::Jbw { pos, depth } => {
                if let Some(rel_instruction_ref) = raw_tokens[..=index].iter().rev().position(|other_token| {
                    match other_token {
                        RawToken::Jfw { pos: _, depth: other_depth } => *other_depth == *depth,
                        _ => false
                    }
                }) {
                    Ok(Token::Jbw {
                        instruction_ref: index - rel_instruction_ref,
                    })
                } else {
                    Err(LexingError::MissingOpeningBracket { pos: *pos })
                }
            }
            RawToken::Sys(_) => {Ok(Token::Sys)}
        }
    }).collect::<Result<Vec<_>, _>>()
}

#[cfg(test)]
mod tests {
    use crate::lexer::{link_raw_tokens, RawToken, to_raw_tokens, Token, tokenize};
    use crate::lexer::LexingError::{MissingClosingBracket, MissingOpeningBracket};

    #[test]
    fn tokenize_raw() {
        assert_eq!(to_raw_tokens("+[-[]]Hi%"), vec![RawToken::Inc(0), RawToken::Jfw { pos: 1, depth: 0 }, RawToken::Dec(2), RawToken::Jfw { pos: 3, depth: 1 }, RawToken::Jbw { pos: 4, depth: 1 }, RawToken::Jbw { pos: 5, depth: 0 }, RawToken::Sys(8)])
    }

    #[test]
    fn linking() {
        assert_eq!(
            link_raw_tokens(&[RawToken::Inc(0), RawToken::Jfw { pos: 1, depth: 0 }, RawToken::Dec(2), RawToken::Jfw { pos: 3, depth: 1 }, RawToken::Jbw { pos: 4, depth: 1 }, RawToken::Jbw { pos: 5, depth: 0 }, RawToken::Sys(8)]),
            Ok(vec![Token::Inc, Token::Jfw { instruction_ref: 5 }, Token::Dec, Token::Jfw { instruction_ref: 4 }, Token::Jbw { instruction_ref: 3 }, Token::Jbw { instruction_ref: 1 }, Token::Sys ])
        );
        assert_eq!(
            link_raw_tokens(&[RawToken::Inc(0), RawToken::Jfw { pos: 1, depth: 0 }, RawToken::Dec(2), RawToken::Jfw { pos: 3, depth: 1 }, RawToken::Jbw { pos: 4, depth: 1 }, RawToken::Sys(8)]),
            Err(MissingClosingBracket { pos: 1 })
        );
        assert_eq!(
            link_raw_tokens(&[RawToken::Inc(0), RawToken::Jbw { pos: 4, depth: 0 }, RawToken::Sys(8)]),
            Err(MissingOpeningBracket { pos: 4 })
        );
    }

    #[test]
    fn tokenize_program() {
        assert_eq!(tokenize("++--<>>[--%[]]"), Ok(vec![Token::Inc, Token::Inc, Token::Dec, Token::Dec, Token::Ddp, Token::Idp, Token::Idp, Token::Jfw { instruction_ref: 13 }, Token::Dec, Token::Dec, Token::Sys, Token::Jfw { instruction_ref: 12 }, Token::Jbw { instruction_ref: 11 }, Token::Jbw { instruction_ref: 7 }]));
        assert_eq!(tokenize("++[][%"), Err(MissingClosingBracket { pos: 4 }));
        assert_eq!(tokenize("++[]]%"), Err(MissingOpeningBracket { pos: 4 }));
    }
}