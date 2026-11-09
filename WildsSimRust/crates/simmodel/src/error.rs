use std::fmt;

#[derive(Debug)]
pub struct SimError(pub String);

impl fmt::Display for SimError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for SimError {}

impl From<std::io::Error> for SimError {
    fn from(value: std::io::Error) -> Self {
        SimError(value.to_string())
    }
}

impl From<csv::Error> for SimError {
    fn from(value: csv::Error) -> Self {
        SimError(value.to_string())
    }
}

pub type SimResult<T> = Result<T, SimError>;

pub fn err<T>(message: impl Into<String>) -> SimResult<T> {
    Err(SimError(message.into()))
}
