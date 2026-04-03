use std::sync::Arc;

use crate::tokenizer::lexer::File;

#[derive(Debug, Clone)]
pub struct GlobalContext{
	pub root: Arc<File>,
	pub root_package: String
}