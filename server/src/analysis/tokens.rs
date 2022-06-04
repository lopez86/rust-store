use std::collections::HashMap;


/// Get a map from the expected keyword to tokens
pub fn get_word_to_token_map() -> HashMap<String, Token> {
    HashMap::from([
        ("get".to_string(), Token::Get),
        ("set".to_string(), Token::Set),
        ("del".to_string(), Token::Delete),
        ("ex".to_string(), Token::Exists),
        ("upd".to_string(), Token::Update),
        ("lt".to_string(), Token::Lifetime),
        ("try_get".to_string(), Token::GetOrNone),
        ("try_set".to_string(), Token::SetIfNotExists),
        ("none".to_string(), Token::None),
        ("true".to_string(), Token::Bool(true)),
        ("false".to_string(), Token::Bool(false)),
        ("type".to_string(), Token::ValueType),
        // Vector operations
        ("vset".to_string(), Token::VectorSet),
        ("vget".to_string(), Token::VectorGet),
        ("vpop".to_string(), Token::VectorPop),
        ("vpush".to_string(), Token::VectorAppend),
        ("vlen".to_string(), Token::VectorLength),
        // Map operations
        ("mex".to_string(), Token::MapExists),
        ("mget".to_string(), Token::MapGet),
        ("mset".to_string(), Token::MapSet),
        ("mdel".to_string(), Token::MapDelete),
        ("mlen".to_string(), Token::MapLength),
        // Type keywords
        ("int".to_string(), Token::IntType),
        ("float".to_string(), Token::FloatType),
        ("str".to_string(), Token::StringType),
        ("bool".to_string(), Token::BoolType),
        ("vec".to_string(), Token::VectorType),
        ("map".to_string(), Token::MapType),
    ])
}


/// A token with some extra annotations needed for error handling
#[derive(Clone, Debug, PartialEq)]
pub struct AnnotatedToken {
    /// The token to process
    pub token: Token,
    /// The position in the input
    pub position: usize,
    /// The string of the current value
    pub lexeme: String,
}


/// Basic tokens that a command might include
#[derive(Clone, Debug, PartialEq)]
pub enum Token {
    /// Get a value
    Get,
    /// Set a value
    Set,
    /// Delete a value
    Delete,
    /// Check if a value exists
    Exists,
    /// Update something
    Update,
    /// Get/set lifetimes
    Lifetime,
    /// Get only if it exists
    GetOrNone,
    /// Set only if it doesn't exist
    SetIfNotExists,
    /// Null value
    None,
    /// Beginning of a list
    LeftBracket,
    /// End of a list
    RightBracket,
    /// Beginning of a map
    LeftCurlyBracket,
    /// End of a map
    RightCurlyBracket,
    /// A comma
    Comma,
    /// A colon
    Colon,
    /// A semicolon
    Semicolon,
    /// Set the lifetime
    SetLifetime,
    /// Map element set
    MapSet,
    /// Vector element set
    VectorSet,
    /// Map element get
    MapGet,
    /// Vector element set
    VectorGet,
    /// Vector appent
    VectorAppend,
    /// Map element delete
    MapDelete,
    /// Vector pop
    VectorPop,
    /// Vector length
    VectorLength,
    /// Map length/size
    MapLength,
    /// Check if a key is in a map
    MapExists,
    /// What kind of object something is 
    ValueType,
    /// Integer type
    IntType,
    /// Float type
    FloatType,
    /// String type
    StringType,
    /// Vector type
    VectorType,
    /// Map type
    MapType,
    /// Bool type
    BoolType,
    /// Boolean literal
    Bool(bool),
    /// Integer literal
    Integer(i64),
    /// Float literal
    Float(f32),
    /// String literal
    StringValue(Box<String>),
    /// Identifier literal
    Identifier(Box<String>),
}
