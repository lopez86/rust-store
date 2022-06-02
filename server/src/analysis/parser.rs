use std::iter::Iterator;

use client::error::ServerError;

use crate::analysis::{Statement, Token, Tokenizer};

/// Parsing tokens into statements
pub struct Parser {
    /// The tokens to parse
    tokens: Vec<Token>,
    /// The current location at this point in parsing
    current_token: usize,
    /// Has an error been found in parsing
    error_encountered: bool,
}

impl Parser {
    /// Construct a new parser
    pub fn new(tokens: Vec<Token>) -> Parser {
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
    pub fn from_iter(token_iter: Box<dyn Iterator<Item=Token>>) -> Result<Parser, ServerError> {
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
    fn view(&self) -> &Token {
        &self.tokens[self.current_token]
    }

    /// Consume a token, advance, and return
    fn advance(&mut self) -> &Token {
        self.current_token += 1;
        &self.tokens[self.current_token - 1]
    }

    /// Get the next available statement
    fn get_next_statement(&mut self) -> Result<Option<Statement>, ServerError> {
        self.strip_semicolons();
        if self.is_at_end() {
            return Ok(None);
        }
        let command = match self.find_command_token() {
            Ok(command) => command,
            Err(err) => return Err(err),
        }

        match command {

        }
        
    }
    
    fn find_command_token() -> Result<Statement, ServerError> {
        
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
