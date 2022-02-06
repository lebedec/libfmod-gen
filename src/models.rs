use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq)]
pub enum Error {
    FileMalformed,
    Pest(String),
    Serde(String),
}

impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Self {
        Self::Serde(error.to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Pointer {
    NormalPointer(String),
    DoublePointer(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Type {
    FundamentalType(String),
    UserType(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Argument {
    pub as_const: Option<String>,
    pub argument_type: Type,
    pub pointer: Option<Pointer>,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Function {
    pub return_type: Type,
    pub name: String,
    pub arguments: Vec<Argument>,
}
