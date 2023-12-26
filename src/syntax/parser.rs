use std::fmt;
use std::str::FromStr;
use crate::syntax::lexer::{TokenStream, Token as LToken};

mod helper {
    use crate::syntax::lexer::{Token as LToken, Token, TokenStream};
    use super::ParsingError;

    pub fn unwrap_word(lex_token: Option<LToken>, parsing_as: &'static str) -> Result<(String, usize), ParsingError> {
        match lex_token {
            None => Err(ParsingError::StreamTokenDepleted),
            Some(Token::SentenceEnd { pos }) => Err(
                ParsingError::EndOfSentence { parsing_as, pos }
            ),
            Some(Token::Word { value, start }) => Ok((value, start))
        }
    }

    pub fn consume_eos_token<C>(stream: &mut TokenStream<C>)
        where C: Iterator<Item = char>
    {
        if let Some(eos_token) = stream.next() {
            match eos_token {
                LToken::Word { value, start } => stream.schedule(LToken::Word { value, start }),
                LToken::SentenceEnd { .. } => { },
            };
        };
    }
}

#[derive(Debug)]
pub struct Script(Vec<Token>);


impl Script {
    pub fn get_tokens(&self) -> &[Token] {
        &self.0
    }
}


impl TryFrom<&str> for Script {
    type Error = ParsingError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::try_from(&mut TokenStream::from(value.chars()))
    }
}


impl<C> TryFrom<&mut TokenStream<C>> for Script
    where C: Iterator<Item = char>
{
    type Error = ParsingError;

    fn try_from(token_stream: &mut TokenStream<C>) -> Result<Self, Self::Error> {
        let mut tokens = Vec::new();
        loop {
            match Token::try_from(&mut *token_stream) {
                Err(ParsingError::StreamTokenDepleted) => { break; },
                token => tokens.push(token?),
            };
        };

        Ok(Self(tokens))
    }
}


impl fmt::Display for Script {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for token in self.0[..self.0.len() - 1].iter() {
            writeln!(f, "{token}")?;
        };

        write!(f, "{}", self.0.last().unwrap())
    }
}


#[derive(Debug)]
pub enum Token {
    Property {
        name: String,
        value: Value,
    },
    Label {
        name: String,
    },
    Command {
        name: String,
        arguments: Vec<Value>,
    }
}

impl Token {
    const PROPERTY_SEPARATOR: &'static str = ":";
    const LABEL_MARKER: &'static str = "@";
}


impl<C> TryFrom<&mut TokenStream<C>> for Token
    where C: Iterator<Item = char>
{
    type Error = ParsingError;

    fn try_from(stream: &mut TokenStream<C>) -> Result<Self, Self::Error> {
        if let Some(token) = stream.next() {
            match helper::unwrap_word(Some(token), "ptoken")?.0.as_str() {
                Self::LABEL_MARKER => {
                    let label_token = Self::Label { name: helper::unwrap_word(stream.next(), "label")?.0 };

                    helper::consume_eos_token(stream);

                    Ok(label_token)
                },
                name => {
                    let property_sep = stream.next();
                    if let Some(LToken::Word { value, ..}) = property_sep.clone() && value == Self::PROPERTY_SEPARATOR {
                        let property_token = Self::Property { name: String::from(name), value: Value::try_from(&mut *stream)? };

                        helper::consume_eos_token(&mut *stream);

                        Ok(property_token)
                    } else {
                        if let Some(property_sep_token) = property_sep {
                            stream.schedule(property_sep_token);
                        };

                        let mut arguments = Vec::new();
                        loop {
                            let next_token = stream.next();

                            if let Some(LToken::SentenceEnd { .. }) = next_token {
                                break;
                            } else if let Some(token) = next_token {
                                stream.schedule(token);
                            };

                            let value = Value::try_from(&mut *stream);
                            if let Err(ParsingError::EndOfSentence { .. }) = value {
                                break;
                            } else {
                                arguments.push(value?);
                            };
                        };

                        Ok(Self::Command { name: String::from(name), arguments })
                    }
                }
            }
        } else {
            Err(Self::Error::StreamTokenDepleted)
        }
    }
}


impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::Property { name, value } => write!(f, "{name}: {value}"),
            Token::Label { name } => write!(f, "@{name}"),
            Token::Command { name, arguments } => {
                write!(f, "{name}")?;

                for argument in arguments.iter() {
                    write!(f, " {argument}")?;
                };

                Ok(())
            }
        }
    }
}

#[derive(Clone, Debug)]
pub enum Value {
    Whole(u32),
    Fraction {
        numerator: u32,
        denominator: u32,
    },
    String(String),
}


impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Whole(n) => write!(f, "{n}"),
            Self::Fraction {numerator: num, denominator: don} => write!(f, "{num} / {don}"),
            Self::String(s) => write!(f, "{s}"),
        }
    }
}


impl Value {
    const FRACTION_SEPARATOR: &'static str = "/";

    fn parse_num<N: FromStr>(s: &str, pos: usize) -> Result<N, ParsingError>
        where <N as FromStr>::Err: fmt::Display
    {
        s.trim().parse::<N>().map_err(|err| ParsingError::ValueError {
            tried_parsing: Some(String::from(s)),
            parsing_as: "value",
            err_msg: Some(format!("{}", err)),
            pos: Some(pos)
        })
    }

    #[inline]
    fn parse_wrapped_num(num_token: Option<LToken>) -> Result<u32, ParsingError> {
        let word = helper::unwrap_word(num_token, "value")?;

        Self::parse_num(word.0.as_str(), word.1)
    }
}


impl<C> TryFrom<&mut TokenStream<C>> for Value
    where C: Iterator<Item = char>
{
    type Error = ParsingError;

    fn try_from(stream: &mut TokenStream<C>) -> Result<Self, Self::Error> {
        let token_a = helper::unwrap_word(stream.next(), "value")?;

        match Self::parse_num(&token_a.0, token_a.1) {
            Err(_) => match Self::parse_num::<f64>(&token_a.0, token_a.1) {
                Err(_) => Ok(Self::String(token_a.0)),
                Ok(num_a) => {
                    todo!("floats inputted with tenth fractions (float to fraction)")
                }
            },
            Ok(num_a) => {
                let separator = stream.next();
                match separator {
                    None => Err(ParsingError::StreamTokenDepleted),
                    Some(LToken::SentenceEnd { pos }) => {
                        stream.schedule(LToken::SentenceEnd { pos });
                        Ok(Self::Whole(num_a))
                    }
                    Some(LToken::Word { value, start }) => {
                        if value != Self::FRACTION_SEPARATOR {
                            stream.schedule(LToken::Word { value, start });

                            Ok(Self::Whole(num_a))
                        } else {
                            let num_b = Self::parse_wrapped_num(stream.next())?;

                            Ok(Self::Fraction { numerator: num_a, denominator: num_b })
                        }
                    }
                }
            },
        }
    }
}


#[derive(Debug)]
pub enum ParsingError {
    ValueError {
        parsing_as: &'static str,
        tried_parsing: Option<String>,
        err_msg: Option<String>,
        pos: Option<usize>,
    },
    EndOfSentence {
        parsing_as: &'static str,
        pos: usize,
    },
    StreamTokenDepleted,
}
