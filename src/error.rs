use std::fmt;

#[derive(Debug)]
pub enum VaporError {
    AwsSdk(String),
    // Constructed only by feature-gated modules (e.g. `cloudwatch`); unused under the default feature set.
    #[allow(dead_code)]
    InvalidInput(String),
}

impl fmt::Display for VaporError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VaporError::AwsSdk(msg) => write!(f, "AWS SDK error: {msg}"),
            VaporError::InvalidInput(msg) => write!(f, "Invalid input: {msg}"),
        }
    }
}

impl std::error::Error for VaporError {}

