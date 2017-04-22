
use std::str::*;
use std::iter::*;

#[derive(Debug, PartialEq)]
pub enum InputToken {
    Identifier(String),
    RedirectIn,
    RedirectOut,
    RedirectAppendOut,
    Pipe,
    Background,
    LogicAnd,
    LogicOr,
    Comment,
    NewLine,
}

pub struct InputLexer<'a> {
    input: Peekable<Chars<'a>>,
}

impl<'a> InputLexer<'a> {
    pub fn new(inp: &'a str) -> InputLexer {
        InputLexer { input: inp.chars().peekable() }
    }

    fn peek_char(&mut self) -> char {
        *self.input.peek().unwrap_or(&'\0')
    }

    fn handle_double(&mut self, ch: char, single: Option<InputToken>, double: Option<InputToken>) -> Option<InputToken> {
        if self.peek_char() == ch {
            self.input.next();
            return double;
        }

        return single;
    }
}

impl<'a> Iterator for InputLexer<'a> {
    type Item = InputToken;

    fn next(&mut self) -> Option<InputToken> {
        loop {
            match self.peek_char() {
                ' ' | '\t' => { self.input.next(); },
                '\0' => return None,
                '#' => { self.input.next(); return Some(InputToken::Comment); },
                '<' => { self.input.next(); return Some(InputToken::RedirectIn); },
                '>' => {
                    self.input.next();
                    return self.handle_double('>', Some(InputToken::RedirectOut), Some(InputToken::RedirectAppendOut));
                },
                '&' => {
                    self.input.next();
                    return self.handle_double('&', Some(InputToken::Background), Some(InputToken::LogicAnd));
                },
                '|' => {
                    self.input.next();
                    return self.handle_double('|', Some(InputToken::Pipe), Some(InputToken::LogicOr));
                },
                '\n' => {
                    self.input.next();
                    return Some(InputToken::NewLine);
                },
                '\"' => {
                    self.input.next();
                    return Some(InputToken::Identifier(self.input.by_ref().take_while(|c| *c != '\"').collect()));
                },
                _ => {
                    return Some(InputToken::Identifier(self.input.by_ref()
                                                       .take_while(|c| (*c).is_alphabetic()
                                                                    || (*c).is_digit(10)
                                                                    || *c == '_'
                                                                    || *c == '-'
                                                                    || *c == '/'
                                                                    || *c == '.'
                                                                    ).collect()));
                }
            }
        }
    }
}

