use std::iter::Iterator;

use crate::analysis::{AnnotatedToken, Statement, Token, Tokenizer};
use crate::error::ServerError;
use crate::storage::{StorageKey, StorageValue};


/// Parsing tokens into statements
pub struct Parser {
    /// The tokens to parse
    tokens: Vec<AnnotatedToken>,
    /// The current location at this point in parsing
    current_token: usize,
    /// Has an error been found in parsing
    error_encountered: bool,
}

fn get_name_from_token(token: &AnnotatedToken) -> Result<String, ServerError> {
    let map_name = match &token.token {
        Token::Identifier(identifier) => identifier,
        _ => return Err(
            ServerError::ParseError(
                format!("Expected an identifier. Got {} at {}", token.lexeme, token.position)
            )
        ),
    };
    Ok(*map_name.clone())
}

fn get_key_from_token(token: &AnnotatedToken) -> Result<StorageValue, ServerError> {
    match &token.token {
        Token::Integer(value) => Ok(StorageValue::Int(*value)),
        Token::StringValue(value) => Ok(StorageValue::String(*value.clone())),
        _ => Err(
            ServerError::ParseError(
                format!("Expected a valid map key. Got {} at {}", token.lexeme, token.position)
            )
        )
    }
}

fn get_index_from_token(token: &AnnotatedToken) -> Result<usize, ServerError> {
    match token.token {
        Token::Integer(value) => {
            match value.try_into() {
                Ok(value) => Ok(value),
                Err(_) => Err(
                    ServerError::ParseError(
                        format!(
                            "Expected a valid vector index. Got {} at {}",
                            token.lexeme,
                            token.position,
                        )
                    )
                )
            }
        },
        _ => Err(
            ServerError::ParseError(
                format!("Expected a valid vector index. Got {} at {}", token.lexeme, token.position)
            )
        )
    }
}

impl Parser {
    /// Construct a new parser
    pub fn new(tokens: Vec<AnnotatedToken>) -> Parser {
        Parser { tokens, current_token: 0 , error_encountered: false}
    }

    /// Construct a new parser from a tokenizer
    pub fn from(tokenizer: Tokenizer) -> Result<Parser, ServerError> {
        let mut tokens = vec![];
        for maybe_token in tokenizer {
            match maybe_token {
                Err(err) => return Err(err),
                Ok(token) => tokens.push(token),
            }
        }
        Ok(Parser{ tokens, current_token: 0, error_encountered: false})
    }

    /// Construct a new parser from a tokenizer
    pub fn from_iter(
        token_iter: Box<dyn Iterator<Item=AnnotatedToken>>
    ) -> Result<Parser, ServerError> {
        let tokens = token_iter.collect();
        Ok(Parser{ tokens, current_token: 0, error_encountered: false})
    }

    /// Parse all statements
    pub fn parse(&mut self) -> Result<Vec<Statement>, ServerError> {
        let mut statements = vec![];
        for maybe_statement in self {
            match maybe_statement {
                Ok(statement) => statements.push(statement),
                Err(err) => return Err(err),
            }
        }
        Ok(statements)
    }

    /// Check if we are at the end and need to stop parsing
    fn is_at_end(&self) -> bool {
        self.current_token >= self.tokens.len()
    }

    /// Look at the current token
    fn view(&self) -> &AnnotatedToken {
        &self.tokens[self.current_token]
    }

    /// Consume a token, advance, and return
    fn advance(&mut self) -> &AnnotatedToken {
        self.current_token += 1;
        &self.tokens[self.current_token - 1]
    }

    /// Get the next available statement
    fn get_next_statement(&mut self) -> Result<Option<Statement>, ServerError> {
        self.strip_semicolons();
        if self.is_at_end() {
            return Ok(None);
        }
        let AnnotatedToken{token, position, lexeme,} = self.advance();
        let statement = match token {
            Token::Delete => self.delete(),
            Token::Exists => self.exists(),
            Token::Get => self.get(),
            Token::GetOrNone => self.get_or_none(),
            Token::MapDelete => self.map_delete(),
            Token::MapExists => self.map_exists(),
            Token::MapGet => self.map_get(),
            Token::MapLength => self.map_length(),
            Token::MapSet => self.map_set(),
            Token::Set => self.set(),
            Token::SetIfNotExists => self.set_if_not_exists(),
            Token::SetLifetime => self.set_lifetime(),
            Token::Update => self.update(),
            Token::ValueType => self.value_type(),
            Token::VectorAppend => self.vector_append(),
            Token::VectorGet => self.vector_get(),
            Token::VectorLength => self.vector_length(),
            Token::VectorPop => self.vector_pop(),
            Token::VectorSet => self.vector_set(),
            _ => return Err(
                ServerError::ParseError(
                    format!(
                        "Cannot parse {} at position {}. Expected a command keyword",
                        lexeme,
                        position
                    )
                )
            ),
        };
        match statement {
            Ok(statement) => Ok(Some(statement)),
            Err(err) => Err(err),
        }
    }

    fn process_identifier_statement<F>(&mut self, f: F) -> Result<Statement, ServerError>
    where F: Fn(&String) -> Statement
    {
        let next_token = self.advance();
        
        match &next_token.token {
            Token::Identifier(identifier) => {
                Ok(f(&*identifier))
            },
            _ => Err(
                ServerError::ParseError(
                    format!(
                        "Expected an identifier. Got {} at {}",
                        next_token.lexeme,
                        next_token.position
                    )
                )
            ),
        }
    }

    fn process_map_identifier_statement<F>(&mut self, f: F) -> Result<Statement, ServerError>
    where F: Fn(&StorageKey, StorageValue) -> Statement
    {
        let next_token = self.advance();
        let map_name = match get_name_from_token(next_token) {
            Ok(name) => name,
            Err(err) => return Err(err),
        };
        let key = match get_key_from_token(next_token) {
            Ok(key) => key,
            Err(err) => return Err(err),
        };
        Ok(f(&map_name, key))
    }   

    fn delete(&mut self) -> Result<Statement, ServerError> {
        self.process_identifier_statement(|x| Statement::Delete(x.clone()))
    }

    fn exists(&mut self) -> Result<Statement, ServerError> {
        self.process_identifier_statement(|x| Statement::Exists(x.clone()))
    }

    fn get(&mut self) -> Result<Statement, ServerError> {
        self.process_identifier_statement(|x| Statement::Get(x.clone()))   
    }

    fn get_or_none(&mut self) -> Result<Statement, ServerError> {
        self.process_identifier_statement(
            |x| Statement::GetIfExists(x.clone()))   
    }

    fn map_delete(&mut self) -> Result<Statement, ServerError> {
        self.process_map_identifier_statement(
            |x, y| Statement::MapDelete(x.clone(), y)
        )
    }

    fn map_exists(&mut self) -> Result<Statement, ServerError> {
        self.process_map_identifier_statement(
            |x, y| Statement::MapExists(x.clone(), y)
        )
    }

    fn map_get(&mut self) -> Result<Statement, ServerError> {
        self.process_map_identifier_statement(
            |x, y| Statement::MapGet(x.clone(), y)
        )
    }

    fn map_length(&mut self) -> Result<Statement, ServerError> {
        self.process_identifier_statement(
            |x| Statement::MapLength(x.clone())
        )
    }

    fn map_set(&mut self) -> Result<Statement, ServerError> {
        Err(ServerError::ParseError("Feature not implemented.".to_string()))
    }

    fn set(&mut self) ->Result<Statement, ServerError> {
        Err(ServerError::ParseError("Feature not implemented.".to_string()))
    }

    fn set_if_not_exists(&mut self) -> Result<Statement, ServerError> {
        Err(ServerError::ParseError("Feature not implemented.".to_string()))
    }

    fn set_lifetime(&mut self) -> Result<Statement, ServerError> {
        Err(ServerError::ParseError("Feature not implemented.".to_string()))
    }

    fn update(&mut self) -> Result<Statement, ServerError> {
        Err(ServerError::ParseError("Feature not implemented.".to_string()))
    }

    fn value_type(&mut self) -> Result<Statement, ServerError> {
        self.process_identifier_statement(
            |x| Statement::ValueType(x.clone())
        )
    }

    fn vector_append(&mut self) -> Result<Statement, ServerError> {
        Err(ServerError::ParseError("Feature not implemented.".to_string()))
    }

    fn vector_get(&mut self) -> Result<Statement, ServerError> {
        Err(ServerError::ParseError("Feature not implemented.".to_string()))
    }

    fn vector_length(&mut self) -> Result<Statement, ServerError> {
        self.process_identifier_statement(
            |x| Statement::VectorLength(x.clone())
        )    }

    fn vector_pop(&mut self) -> Result<Statement, ServerError> {
        self.process_identifier_statement(
            |x| Statement::VectorPop(x.clone())
        )
    }

    fn vector_set(&mut self) -> Result<Statement, ServerError> {
        Err(ServerError::ParseError("Feature not implemented.".to_string()))
    }

    /// Remove any successive semicolons at the current position
    /// 
    /// Semicolons can optionally appear at the end of a statement or to separate statements
    /// but have no other effect. They should be removed prior to looking for the next complete
    /// statement.
    fn strip_semicolons(&mut self) {
        loop {
            if self.is_at_end() {
                break;
            }
            if let AnnotatedToken{token: Token::Semicolon, ..} = self.view() {
                self.advance();
            } else {
                break;
            }
        }
    }

    
}

impl Iterator for Parser {
    type Item = Result<Statement, ServerError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.is_at_end() | self.error_encountered {
            return None;
        }
        match self.get_next_statement() {
            Ok(None) => None,
            Ok(Some(statement)) => Some(Ok(statement)),
            Err(err) => Some(Err(err)),
        }
    }
}
