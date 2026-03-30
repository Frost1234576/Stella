pub mod method;
pub mod structs;
pub mod expr;
pub mod registries;

#[derive(Debug, Clone)]
pub struct ParserError {
    pub line: usize,
    pub message: String,
}