use crate::analysis::tokens::{AnnotatedToken, Token, get_word_to_token_map};
use crate::error::ServerError;


/// See if a character can be used to start an identifier
fn is_identifier_start_char(c: char) -> bool {
    c.is_alphabetic() | (c == '_')
}

/// See if a character is valid for an identifier
fn is_identifier_char(c: char) -> bool {
    c.is_alphanumeric() | (c == '_')
}

/// See if a character is valid to directly append to the end of a literal value
fn is_valid_literal_end_char(c: char) -> bool {
    c.is_whitespace() | ";:,]}".contains(c)
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
    pub fn tokenize(&mut self) -> Result<Vec<AnnotatedToken>, ServerError> {
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
        while self.view().is_whitespace() {
            self.advance();
        }
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
        } else if next_char == ':' {
            self.advance();
            Ok(Token::Colon)
        } else if next_char == '{' {
            self.advance();
            Ok(Token::LeftCurlyBracket)
        } else if next_char == '}' {
            self.advance();
            Ok(Token::RightCurlyBracket)
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
            let next_char = self.view();
            if is_valid_literal_end_char(next_char) {
                break;
            }
            if next_char == '.' {
                is_float = true;
            }
            char_vec.push(next_char);
            self.advance();
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
                    return Err(
                        ServerError::TokenizationError("Unterminated string found.".to_string())
                    );
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
        if !is_valid_literal_end_char(self.view()) {
            return Err(
                ServerError::TokenizationError(
                    "Invalid character found at the end of a string.".to_string()
                )
            )
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
            let next_char = self.view();
            if is_valid_literal_end_char(next_char) {
                break;
            } else if is_identifier_char(next_char) {
                self.advance();
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
                            self.token_start_index.. self.current_index
                        ].iter().collect(),
                    }
                )
            )
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::iter::zip;

    #[test]
    fn test_is_identifier_start_char() {
        assert!(is_identifier_start_char('_'));
        assert!(is_identifier_start_char('a'));
        assert!(is_identifier_start_char('A'));
        assert!(!is_identifier_start_char('1'));
        assert!(!is_identifier_start_char('#'));
    }

    #[test]
    fn test_is_identifier_char() {
        assert!(is_identifier_char('_'));
        assert!(is_identifier_char('a'));
        assert!(is_identifier_char('A'));
        assert!(is_identifier_char('1'));
        assert!(!is_identifier_char('#'));
    }

    #[test]
    fn test_is_valid_literal_end_char() {
        assert!(is_valid_literal_end_char(';'));
        assert!(is_valid_literal_end_char(','));
        assert!(is_valid_literal_end_char(']'));
        assert!(is_valid_literal_end_char('}'));
        assert!(is_valid_literal_end_char(':'));
        assert!(is_valid_literal_end_char(' '));
        assert!(is_valid_literal_end_char('\n'));
        assert!(!is_valid_literal_end_char('a'));
        assert!(!is_valid_literal_end_char('2'));
        assert!(!is_valid_literal_end_char('B'));
        assert!(!is_valid_literal_end_char('"'));
        assert!(!is_valid_literal_end_char('!'));
    }

    #[test]
    fn test_tokenizer_simple_query() {
        let mut tokenizer = Tokenizer::new("set x 1");
        let tokens = tokenizer.tokenize().unwrap();
        let expected_tokens = vec![
            AnnotatedToken{token: Token::Set, position: 0, lexeme: "set".to_string()},
            AnnotatedToken{
                token: Token::Identifier(Box::new("x".to_string())),
                position: 4,
                lexeme: "x".to_string()
            },
            AnnotatedToken{token: Token::Integer(1), position: 6, lexeme: "1".to_string()},
        ];
        assert_eq!(3, tokens.len());
        for (expected_token, token) in zip(expected_tokens, tokens) {
            assert_eq!(expected_token, token);
        }
    }

    #[test]
    fn test_tokenizer_query_with_string() {
        let mut tokenizer = Tokenizer::new("set x \"abc\";");
        let tokens = tokenizer.tokenize().unwrap();
        let expected_tokens = vec![
            AnnotatedToken{token: Token::Set, position: 0, lexeme: "set".to_string()},
            AnnotatedToken{
                token: Token::Identifier(Box::new("x".to_string())),
                position: 4,
                lexeme: "x".to_string()
            },
            AnnotatedToken{
                token: Token::StringValue(Box::new("abc".to_string())),
                position: 6,
                lexeme: "\"abc\"".to_string()
            },
            AnnotatedToken{token: Token::Semicolon, position: 11, lexeme: ";".to_string()},
        ];
        assert_eq!(4, tokens.len());
        for (expected_token, token) in zip(expected_tokens, tokens) {
            assert_eq!(expected_token, token);
        }
    }

    
    #[test]
    fn test_tokenizer_query_with_float() {
        let mut tokenizer = Tokenizer::new("set x 1.0;");
        let tokens = tokenizer.tokenize().unwrap();
        let expected_tokens = vec![
            AnnotatedToken{token: Token::Set, position: 0, lexeme: "set".to_string()},
            AnnotatedToken{
                token: Token::Identifier(Box::new("x".to_string())),
                position: 4,
                lexeme: "x".to_string()
            },
            AnnotatedToken{
                token: Token::Float(1.0),
                position: 6,
                lexeme: "1.0".to_string()
            },
            AnnotatedToken{token: Token::Semicolon, position: 9, lexeme: ";".to_string()},
        ];
        assert_eq!(4, tokens.len());
        for (expected_token, token) in zip(expected_tokens, tokens) {
            assert_eq!(expected_token, token);
        }
    }

    #[test]
    fn test_tokenizer_query_with_list() {
        let mut tokenizer = Tokenizer::new("set x [1, 2, 3];");
        let tokens = tokenizer.tokenize().unwrap();
        let expected_tokens = vec![
            AnnotatedToken{token: Token::Set, position: 0, lexeme: "set".to_string()},
            AnnotatedToken{
                token: Token::Identifier(Box::new("x".to_string())),
                position: 4,
                lexeme: "x".to_string()
            },
            AnnotatedToken{token: Token::LeftBracket, position: 6, lexeme: "[".to_string()},
            AnnotatedToken{
                token: Token::Integer(1),
                position: 7,
                lexeme: "1".to_string()
            },
            AnnotatedToken{token: Token::Comma, position: 8, lexeme: ",".to_string()},
            AnnotatedToken{
                token: Token::Integer(2),
                position: 10,
                lexeme: "2".to_string()
            },
            AnnotatedToken{token: Token::Comma, position: 11, lexeme: ",".to_string()},
            AnnotatedToken{
                token: Token::Integer(3),
                position: 13,
                lexeme: "3".to_string()
            },
            AnnotatedToken{token: Token::RightBracket, position: 14, lexeme: "]".to_string()},
            AnnotatedToken{token: Token::Semicolon, position: 15, lexeme: ";".to_string()},
        ];
        assert_eq!(10, tokens.len());
        for (expected_token, token) in zip(expected_tokens, tokens) {
            assert_eq!(expected_token, token);
        }
    }

    #[test]
    fn test_tokenizer_query_with_map() {
        let mut tokenizer = Tokenizer::new("set x int int {1:2 , 3 : 4};");
        let tokens = tokenizer.tokenize().unwrap();
        let expected_tokens = vec![
            AnnotatedToken{token: Token::Set, position: 0, lexeme: "set".to_string()},
            AnnotatedToken{
                token: Token::Identifier(Box::new("x".to_string())),
                position: 4,
                lexeme: "x".to_string(),
            },
            AnnotatedToken{
                token: Token::IntType,
                position: 6,
                lexeme: "int".to_string(),
            },
            AnnotatedToken{
                token: Token::IntType,
                position: 10,
                lexeme: "int".to_string(),
            },
            AnnotatedToken{token: Token::LeftCurlyBracket, position: 14, lexeme: "{".to_string()},
            AnnotatedToken{
                token: Token::Integer(1),
                position: 15,
                lexeme: "1".to_string(),
            },
            AnnotatedToken{token: Token::Colon, position: 16, lexeme: ":".to_string()},
            AnnotatedToken{
                token: Token::Integer(2),
                position: 17,
                lexeme: "2".to_string(),
            },
            AnnotatedToken{token: Token::Comma, position: 19, lexeme: ",".to_string()},
            AnnotatedToken{
                token: Token::Integer(3),
                position: 21,
                lexeme: "3".to_string(),
            },
            AnnotatedToken{token: Token::Colon, position: 23, lexeme: ":".to_string()},
            AnnotatedToken{
                token: Token::Integer(4),
                position: 25,
                lexeme: "4".to_string(),
            },
            AnnotatedToken{token: Token::RightCurlyBracket, position: 26, lexeme: "}".to_string()},
            AnnotatedToken{token: Token::Semicolon, position: 27, lexeme: ";".to_string()},
        ];
        assert_eq!(14, tokens.len());
        for (expected_token, token) in zip(expected_tokens, tokens) {
            assert_eq!(expected_token, token);
        }
    }
}
