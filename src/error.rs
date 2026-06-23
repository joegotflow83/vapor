use std::fmt;

#[derive(Debug)]
pub enum VaporError {
    AwsSdk(String),
    Config(String),
    Query(String),
    InvalidInput(String),
}

impl fmt::Display for VaporError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VaporError::AwsSdk(msg) => write!(f, "AWS SDK error: {msg}"),
            VaporError::Config(msg) => write!(f, "Configuration error: {msg}"),
            VaporError::Query(msg) => write!(f, "Query error: {msg}"),
            VaporError::InvalidInput(msg) => write!(f, "Invalid input: {msg}"),
        }
    }
}

impl std::error::Error for VaporError {}

