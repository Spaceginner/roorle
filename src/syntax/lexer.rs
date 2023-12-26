use std::collections::VecDeque;
use std::fmt;
use crate::take::Take;


#[derive(Clone, Debug)]
pub enum Token {
    SentenceEnd {
        pos: usize,
    },
    Word {
        start: usize,
        value: String,
    },
}


impl Token {
    pub const WORD_SEPARATORS: &'static [char] = &[' '];
    pub const INDEPENDENT_WORDS: &'static [char] = &['@', ':', '/'];
    pub const LINE_SEPARATORS: &'static [char] = &['\n', ';'];
    pub const ESCAPE_SYMBOL: char = '\\';
    pub const ENDLINE_COMMENT: char = '#';
    pub const MULTILINE_COMMENT_START: char = '<';
    pub const MULTILINE_COMMENT_END: char = '>';
}


impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SentenceEnd { pos } => write!(f, "separator (at {pos})"),
            Token::Word { start, value } => write!(f, "'{value}' (at {start})"),
        }
    }
}


#[derive(Debug)]
pub struct TokenStream<C>
    where C: Iterator<Item = char>
{
    char_stream: C,
    pos: usize,
    token_queue: VecDeque<Token>,
    escaping: bool,
    last_was_separator: bool,
    commenting: CommentingMode,
}


#[derive(Debug, Default, Copy, Clone, PartialEq)]
enum CommentingMode {
    #[default]
    Disabled,
    Endline,
    Multiline
}


impl<C> TokenStream<C>
    where C: Iterator<Item = char>
{
    pub fn schedule(&mut self, token: Token) {
        self.token_queue.push_front(token);
        // self.last_was_separator = false;
    }
}


impl<C> From<C> for TokenStream<C>
    where C: Iterator<Item = char>
{
    fn from(chars: C) -> Self {
        Self {
            char_stream: chars,
            pos: 0,
            token_queue: VecDeque::new(),
            escaping: false,
            last_was_separator: true,
            commenting: CommentingMode::Disabled,
        }
    }
}


impl<C> Iterator for TokenStream<C>
    where C: Iterator<Item = char>
{
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(token) = self.token_queue.pop_front() {
            if let Token::SentenceEnd { .. } = token {
                self.last_was_separator = true;
            } else {
                self.last_was_separator = false;
            };

            return Some(token);
        };

        let mut token_value = String::new();
        let mut initial_position;

        loop {
            initial_position = self.pos;

            loop {
                match self.char_stream.next() {
                    None => {
                        if token_value.is_empty() && self.token_queue.is_empty() {
                            return if self.last_was_separator {
                                None
                            } else {
                                self.last_was_separator = true;

                                Some(Token::SentenceEnd { pos: initial_position })
                            };
                        } else {
                            break;
                        };
                    },
                    Some(c) => {
                        self.pos += 1;

                        let escaping = self.escaping.take();

                        if escaping && self.commenting == CommentingMode::Disabled {
                            token_value.push(c);
                            continue;
                        }

                        if c == Token::ESCAPE_SYMBOL {
                            self.escaping = true;
                        } else if c == Token::ENDLINE_COMMENT /* && token_value.is_empty() */ {
                            if self.commenting == CommentingMode::Disabled {
                                self.commenting = CommentingMode::Endline;
                            };
                        } else if c == Token::MULTILINE_COMMENT_START {
                            self.commenting = CommentingMode::Multiline;
                        } else if c == Token::MULTILINE_COMMENT_END {
                            if self.commenting == CommentingMode::Multiline {
                                self.commenting = CommentingMode::Disabled;
                            };
                        } else if Token::LINE_SEPARATORS.contains(&c) {
                            if self.commenting != CommentingMode::Multiline {
                                self.token_queue.push_back(Token::SentenceEnd { pos: self.pos - 1 });
                            };

                            if self.commenting == CommentingMode::Endline && !escaping {
                                self.commenting = CommentingMode::Disabled;
                            };

                            break;
                        } else if self.commenting == CommentingMode::Disabled {
                            if Token::WORD_SEPARATORS.contains(&c) {
                                break;
                            } else if Token::INDEPENDENT_WORDS.contains(&c) {
                                self.token_queue.push_back(Token::Word {
                                    value: String::from(c),
                                    start: self.pos - 1,
                                });

                                break;
                            } else {
                                token_value.push(c);
                            };
                        };
                    }
                };
            };

            if !token_value.is_empty() {
                self.last_was_separator = false;

                return Some(Token::Word {
                    value: token_value,
                    start: initial_position
                })
            } else {
                match self.token_queue.pop_front() {
                    token @ Some(Token::SentenceEnd { .. }) => {
                        if self.last_was_separator {
                            continue;
                        } else {
                            self.last_was_separator = true;

                            return token;
                        }
                    },
                    None => continue,
                    token => {
                        self.last_was_separator = false;

                        return token;
                    },
                };
            };
        };
    }
}
