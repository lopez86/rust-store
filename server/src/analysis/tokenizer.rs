use crate::analysis::tokens::{AnnotatedToken, Token, get_word_to_token_map};
use client::error::ServerError;


fn is_identifier_start_char(c: char) -> bool {
    c.is_alphabetic() | (c == '_')
}

fn is_identifier_char(c: char) -> bool {
    c.is_alphanumeric() | (c == '_')
}

fn is_valid_literal_end_char(c: char) -> bool {
    ";:,".contains(c)
}

/// The basic scanner only implements the most basic operations like get and set.
/// 
/// No complicated logic is implemented.
pub struct Tokenizer {
    command: Vec<char>,
    current_index: usize,
    token_start_index: usize,
    error_detected: bool,
}


impl Tokenizer {
    /// Build a new tokenizer
    pub fn new(command: &str) -> Tokenizer {
        let command = command.to_lowercase();
        let command = Vec::from_iter(command.chars());
        Tokenizer {
            command,
            current_index: 0,
            token_start_index: 0,
            error_detected: false,
        }
    }

    /// Scan the text of a command for characters
    pub fn scan(&mut self) -> Result<Vec<AnnotatedToken>, ServerError> {
        let mut tokens: Vec<AnnotatedToken> = vec![];
        for maybe_token in self {
            match maybe_token {
                Err(err) => return Err(err),
                Ok(token) => {
                    tokens.push(token);
                }
            }
        }
        Ok(tokens)
    }

    /// Retrieve the next token
    fn get_next_token(&mut self) -> Result<Token, ServerError> {
        self.token_start_index = self.current_index;
        let next_char = self.view();
        let next_token = if next_char == ';' {
            self.advance();
            Ok(Token::Semicolon)
        } else if next_char == '[' {
            self.advance();
            Ok(Token::LeftBracket)
        } else if next_char == ']' {
            self.advance();
            Ok(Token::RightBracket)
        } else if next_char == ',' {
            self.advance();
            Ok(Token::Comma)
        } else if next_char.is_numeric() | (next_char == '-') {
            self.get_numeric()
        } else if next_char == '"' {
            self.get_string()
        } else if is_identifier_start_char(next_char) {
            self.get_identifier()
        } else {
            return Err(ServerError::TokenizationError("Cannot build token.".to_string()))
        };
        next_token
    }

    /// Check if we are at the end of the command
    fn is_at_end(&self) -> bool {
        self.current_index >= self.command.len()
    }
    /// Consume a character, move to the next one, and return
    fn view(&mut self) -> char {
        self.command[self.current_index]
    }

    /// Consume a character, move to the next one, and return
    fn advance(&mut self) -> char {
        self.current_index = self.current_index + 1;
        self.command[self.current_index - 1]
    }

    /// Get a numeric token (Float or Int)
    fn get_numeric(&mut self) -> Result<Token, ServerError> {
        let mut char_vec = vec![self.advance()];
        let mut is_float = false;
        loop {
            if self.is_at_end() {
                break;
            }
            let next_char = self.advance();
            if next_char.is_whitespace() {
                break;
            }
            if next_char == '.' {
                is_float = true;
            }
            char_vec.push(next_char);
        }
        let token_string: String = char_vec.into_iter().collect();
        if is_float {
            let value: f32 = match token_string.parse() {
                Ok(val) => val,
                Err(_) => {
                    return Err(
                        ServerError::TokenizationError(
                            format!("Expected float literal, got '{}'", token_string)
                        )
                    );
                }
            };
            Ok(Token::Float(value))
        } else {
            let value: i64 = match token_string.parse() {
                Ok(val) => val,
                Err(_) => {
                    return Err(
                        ServerError::TokenizationError(
                            format!("Expected integer literal, got '{}'", token_string)
                        )
                    );
                }
            };
            Ok(Token::Integer(value))
        }
    }

    /// Get a string literal
    fn get_string(&mut self) -> Result<Token, ServerError> {
        self.advance();
        let mut char_vec = vec![];
        loop {
            if self.is_at_end() {
                return Err(ServerError::TokenizationError("Unterminated string found.".to_string()));
            }
            let next_char = self.advance();
            if next_char == '"' {
                break;
            }
            if next_char == '\\' {
                if self.is_at_end() {
                    return Err(ServerError::TokenizationError("Unterminated string found.".to_string()));
                }
                let escape_char = self.advance();
                match escape_char {
                    '\\' | '"'  => char_vec.push(escape_char),
                    'r' => char_vec.push('\r'),
                    't' => char_vec.push('\t'),
                    'n' => char_vec.push('\n'),
                    other => return Err(
                        ServerError::TokenizationError(
                            format!("Invalid escape character '{}' found", other)
                        )
                    ),
                }
            } else {
                char_vec.push(next_char);
            }
        }
        let token_string: String = char_vec.into_iter().collect();
        Ok(Token::StringValue(Box::new(token_string)))
    }

    /// Get an identifier or keyword
    fn get_identifier(&mut self) -> Result<Token, ServerError> {
        let mut char_vec = vec![self.advance()];
        loop {
            if self.is_at_end() {
                break;
            }
            let next_char = self.advance();
            if next_char.is_whitespace() {
                break;
            } else if is_identifier_char(next_char) {
                char_vec.push(next_char)
            } else {
                return Err(
                    ServerError::TokenizationError(
                        format!("'{}' is an invalid identifier character.", next_char)
                    )
                );
            }

        }
        let token_string: String = char_vec.into_iter().collect();
        let token = match get_word_to_token_map().get(&token_string) {
            Some(keyword_token) => keyword_token.clone(),
            None => Token::Identifier(Box::new(token_string)),
        };
        Ok(token)
    }
}

impl Iterator for Tokenizer {
    type Item = Result<AnnotatedToken, ServerError>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.is_at_end() | self.error_detected {
            return None
        }
        match self.get_next_token() {
            Err(err) => {
                self.error_detected = true;
                Some(Err(err))
            }
            Ok(token) => Some(
                Ok(
                    AnnotatedToken {
                        token,
                        position: self.token_start_index,
                        lexeme: self.command[
                            self.token_start_index..self.current_index
                        ].iter().collect(),
                    }
                )
            )
        }
    }
}
