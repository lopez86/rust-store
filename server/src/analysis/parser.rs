use std::iter::Iterator;

use client::error::ServerError;

use crate::analysis::{AnnotatedToken, Statement, Token, Tokenizer};

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
        let next_token = self.advance();
        let AnnotatedToken{token, position, lexeme,} = self.advance();
        let statement = match token {
            Token::Delete => self.delete(self.view()),
            Token::Exists => self.exists(self.view()),
            Token::Get => self.get(self.view()),
            Token::GetOrNone => self.get_or_none(self.view()),
            Token::MapDelete => self.map_delete(self.view()),
            Token::MapExists => self.map_exists(self.view()),
            Token::MapGet => self.map_get(self.view()),
            Token::MapLength => self.map_length(self.view()),
            Token::MapSet => self.map_set(self.view()),
            Token::Set => self.set(self.view()),
            Token::SetIfNotExists => self.set_if_not_exists(self.view()),
            Token::SetLifetime => self.set_lifetime(self.view()),
            Token::Update => self.update(self.view()),
            Token::ValueType => self.value_type(self.view()),
            Token::VectorAppend => self.vector_append(self.view()),
            Token::VectorGet => self.vector_get(self.view()),
            Token::VectorLength => self.vector_length(self.view()),
            Token::VectorPop => self.vector_pop(self.view()),
            Token::VectorSet => self.vector_set(self.view()),
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
        statement
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

    /// Get the next items
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



