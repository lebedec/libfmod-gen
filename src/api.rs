#[derive(Debug, PartialEq)]
pub enum Error {
    FileMalformed,
    Pest(String),
    Serde(String),
}
