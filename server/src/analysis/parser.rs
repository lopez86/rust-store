use std::iter::Iterator;

use crate::analysis::{AnnotatedToken, Statement, Token, Tokenizer};
use crate::error::ServerError;
use crate::storage::{CollectionType, KeyType, StorageKey, StorageValue, StorageVector, StorageMap};


/// Parsing tokens into statements
pub struct Parser {
    /// The tokens to parse
    tokens: Vec<AnnotatedToken>,
    /// The current location at this point in parsing
    current_token: usize,
    /// Has an error been found in parsing
    error_encountered: bool,
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
            Token::Shutdown => self.shutdown(),
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
        let map_name = self.get_name_from_next_token()?;
        if self.is_at_end() {
            return Err(ServerError::ParseError("Expected map key after map name.".to_string()));
        }
        let key = self.get_key_from_next_token()?;
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
        let map_name = self.get_name_from_next_token()?;
        let key = self.get_key_from_next_token()?;
        let value = self.get_scalar_value_from_next_token()?;
        Ok(Statement::MapSet(map_name, key, value))
    }

    fn set(&mut self) ->Result<Statement, ServerError> {
        let name = self.get_name_from_next_token()?;
        let value = self.get_value_from_next_token()?;
        let lifetime = self.get_lifetime_from_next_token()?;
        Ok(Statement::Set(name, value, lifetime))
    }

    fn set_if_not_exists(&mut self) -> Result<Statement, ServerError> {
        let name = self.get_name_from_next_token()?;
        let value = self.get_value_from_next_token()?;
        let lifetime = self.get_lifetime_from_next_token()?;
        Ok(Statement::SetIfNotExists(name, value, lifetime))
    }

    fn set_lifetime(&mut self) -> Result<Statement, ServerError> {
        let name = self.get_name_from_next_token()?;
        let lifetime = self.get_lifetime_from_next_token()?;
        Ok(Statement::UpdateLifetime(name, lifetime))
    }

    fn shutdown(&mut self) -> Result<Statement, ServerError> {
        Ok(Statement::Shutdown)
    }


    fn update(&mut self) -> Result<Statement, ServerError> {
        let name = self.get_name_from_next_token()?;
        let value = self.get_value_from_next_token()?;
        let lifetime = self.get_lifetime_from_next_token()?;
        Ok(Statement::Update(name, value, lifetime))
    }

    fn value_type(&mut self) -> Result<Statement, ServerError> {
        self.process_identifier_statement(
            |x| Statement::ValueType(x.clone())
        )
    }

    fn vector_append(&mut self) -> Result<Statement, ServerError> {
        let name = self.get_name_from_next_token()?;
        let value = self.get_scalar_value_from_next_token()?;
        Ok(Statement::VectorAppend(name, value))
    }

    fn vector_get(&mut self) -> Result<Statement, ServerError> {
        let name = self.get_name_from_next_token()?;
        let index = self.get_index_from_next_token()?;
        Ok(Statement::VectorGet(name, index))
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
        let name = self.get_name_from_next_token()?;
        let index = self.get_index_from_next_token()?;
        let value = self.get_scalar_value_from_next_token()?;
        Ok(Statement::VectorSet(name, index, value))
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

    fn get_name_from_next_token(&mut self) -> Result<String, ServerError> {
        if self.is_at_end() {
            return Err(ServerError::ParseError("Expected an identifier instead of the end of the query.".to_string()));
        }
        let token = self.advance();
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
    
    fn get_key_from_next_token(&mut self) -> Result<StorageValue, ServerError> {
        if self.is_at_end() {
            return Err(ServerError::ParseError("Expected an identifier instead of the end of the query.".to_string()));
        }
        let token = self.advance();
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
    
    fn get_index_from_next_token(&mut self) -> Result<usize, ServerError> {
        if self.is_at_end() {
            return Err(ServerError::ParseError("Expected an identifier instead of the end of the query.".to_string()));
        }
        let token = self.advance();
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

    fn get_scalar_value_from_next_token(&mut self) -> Result<StorageValue, ServerError> {
        if self.is_at_end() {
            return Err(ServerError::ParseError("Expected an identifier instead of the end of the query.".to_string()));
        }
        let next_token = self.advance();
        let storage_value = match &next_token.token {
            Token::Bool(value) => {
                StorageValue::Bool(*value)
            },
            Token::Integer(value) => {
                StorageValue::Int(*value)
            },
            Token::Float(value) => {
                StorageValue::Float(*value)
            },
            Token::StringValue(value) => {
                StorageValue::String(*value.clone())
            },
            _ => return Err(ServerError::ParseError("Expected valid scalar value.".to_string())),
        };
        Ok(storage_value)
    }

    fn get_collection_value_from_next_token(&mut self) -> Result<StorageValue, ServerError> {
        let type_token = self.advance().clone();
        if self.is_at_statement_end() {
            let collection_type = get_collection_type(&type_token.token)?;
            return Ok(StorageValue::Vector(StorageVector::new(collection_type)));
        }

        let next_token = self.view().clone();
        let value = if is_collection_or_key_type(&next_token.token) {
            // We have a map
            self.advance();
            let key_type = get_key_type(&type_token.token)?;
            let collection_type = get_collection_type(&next_token.token)?;
            let map = if self.is_at_statement_end() | (self.view().token != Token::LeftCurlyBracket) {
                StorageValue::Map(StorageMap::new(key_type, collection_type))
            } else {
                self.get_map_value(key_type, collection_type)?
            };
            map
        } else if let Token::LeftBracket = next_token.token {
            let collection_type = get_collection_type(&type_token.token)?;
            self.get_vector_value(collection_type)?
        } else {
            return Err(ServerError::ParseError("Could not parse.".to_string()));
        };
        Ok(value)
    }

    fn get_lifetime_from_next_token(&mut self) -> Result<Option<u64>, ServerError> {
        if self.is_at_statement_end() {
            return Ok(None)
        }
        let value = self.advance();
        if let Token::Integer(value) = value.token {
            if value < 0 {
                Err(ServerError::ParseError("Expected a positive integer as a lifetime.".to_string()))
            } else {
                let unsigned_value: u64 = value.try_into().unwrap();
                Ok(Some(unsigned_value))
            }
        } else {
            Err(ServerError::ParseError("Expected an integer value for a lifetime.".to_string()))
        }
    }

    fn get_value_from_next_token(&mut self) -> Result<StorageValue, ServerError> {
        if self.is_at_statement_end() {
            return Ok(StorageValue::Null);
        }
        let value = if is_collection_or_key_type(&self.view().token) {
            self.get_collection_value_from_next_token()?
        } else {
            self.get_scalar_value_from_next_token()?
        };
        Ok(value)
    }

    fn is_at_statement_end(&self) -> bool {
        if self.is_at_end() {
            true
        } else if let Token::Semicolon = self.view().token {
            true
        } else {
            false
        }
    }

    fn get_vector_value(&mut self, collection_type: CollectionType) -> Result<StorageValue, ServerError> {
        let mut value = StorageVector::new(collection_type);
        // We've already checked that the first character is a left bracket
        self.advance(); // [
        if let Token::RightBracket = self.view().token {
            return Ok(StorageValue::Vector(value));
        }

        loop {
            let element = self.get_scalar_value_from_next_token()?;
            value.push(element)?;
            if self.is_at_end() {
                return Err(ServerError::ParseError("Unfinished vector literal found.".to_string()))
            }
            if let Token::RightBracket = self.view().token {
                break;
            } else if let Token::Comma = self.view().token {
                self.advance();
            } else {
                return Err(ServerError::ParseError("Unexpected token after vector element.".to_string()));
            }
        }
        self.advance(); // ]
        Ok(StorageValue::Vector(value))
    }

    fn get_map_value(&mut self, key_type: KeyType, collection_type: CollectionType) -> Result<StorageValue, ServerError> {
        let mut value = StorageMap::new(key_type, collection_type);
        // We've already checked that the first character is a left bracket
        self.advance(); // [
        if let Token::RightCurlyBracket = self.view().token {
            return Ok(StorageValue::Map(value));
        }

        loop {
            let element_key = self.get_scalar_value_from_next_token()?;
            if self.is_at_end() {
                return Err(ServerError::ParseError("Unfinished map literal found.".to_string()));
            }
            let colon = self.advance();
            if colon.token != Token::Colon {
                return Err(ServerError::ParseError("Expected colon after key.".to_string()));
            }
            let element_value = self.get_scalar_value_from_next_token()?;
            if let Err(_) = value.set(element_key, element_value) {
                return Err(ServerError::ParseError("Could not add element to map.".to_string()));
            }
            if let Token::RightCurlyBracket = self.view().token {
                break;
            } else if let Token::Comma = self.view().token {
                self.advance();
            } else {
                return Err(ServerError::ParseError("Unexpected token after key-value pair.".to_string()));
            }
        }
        self.advance(); // ]
        Ok(StorageValue::Map(value))
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

fn get_collection_type(token: &Token) -> Result<CollectionType, ServerError> {
    match token {
        Token::BoolType => Ok(CollectionType::Bool),
        Token::FloatType => Ok(CollectionType::Float),
        Token::IntType => Ok(CollectionType::Int),
        Token::StringType => Ok(CollectionType::String),
        _ => Err(ServerError::ParseError("Expected a valid collection scalar type.".to_string()))
    }
}

fn get_key_type(token: &Token) -> Result<KeyType, ServerError> {
    match token {
        Token::IntType => Ok(KeyType::Int),
        Token::StringType => Ok(KeyType::String),
        _ => Err(ServerError::ParseError("Expected a valid key scalar type.".to_string()))
    }
}

fn is_collection_or_key_type(token: &Token) -> bool {
    match token {
        Token::BoolType | Token::StringType | Token::FloatType | Token::IntType => true,
        _ => false
    }
}
