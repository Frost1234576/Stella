pub mod lexer;

use std::sync::Arc;
use lexer::File;



pub fn tokenize_file(path: &str) -> std::io::Result<Vec<lexer::Token>> {
	let mut file = File::new(path);
	file.read()?;
	let lexer = lexer::Lexer::new(file.clone().into());
	Ok(lexer.tokenize())
}