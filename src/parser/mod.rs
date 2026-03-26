pub mod method;
pub mod structs;
pub mod expr;

#[derive(Debug, Clone)]
pub struct ParserError {
    pub line: usize,
    pub message: String,
}