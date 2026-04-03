use crate::{tokenizer::lexer::{File, Token}};
use std::{hash::Hash, sync::Arc};
use crate::literals::*;
use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;



#[derive(Debug, Clone, PartialEq)]
pub struct Expr{
	vec: Vec<Op>,
}

impl Expr {
	pub fn new(vec: Vec<Op>) -> Self{
		Self { vec }
	}
    pub fn ops(&self) -> &[Op] {
        &self.vec
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Op {
    Push(Literal),
    LoadIdentifier(String),
    CallMethod(String), // method name - class and method descriptor determined compile time
    CallStaticMethod(String, String),
	GetField(String),
	GetStaticField(String, String), // class, field name - field descriptor determined compile time
    // Arithmetic
    Add, Subtract, Multiply, Divide, Power,
    // Logic
    Not, And, Or, Xor,
    // Comparison
    Equal, NotEqual, Greater, GreaterEqual, Less, LessEqual,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ASTNode{
	Expr(Expr),
    VarDecl(String, Option<PrimitiveType>, Option<Expr>),
    Assignment(String, Expr),
    Return(Option<Expr>),
    Break,
    Continue,

	ScopeEnd(),

    // While(Expr, Vec<ASTNode>, Arc<Scope>), // condition, body
    // For(Box<ASTNode>, Expr, Box<ASTNode>, Vec<ASTNode>, Arc<Scope>), // init, condition, increment, body
    // If(Expr, Option<Arc<Label>>, Option<Arc<Label>>, Arc<Scope>, Option<Arc<Scope>>), // condition, body, else/if block
}


#[derive(Debug, Clone, PartialEq)]
pub enum AST{ // ASTs are only ever functions (technically hooks and events too but hooks and events are functions)
	Function{
		name: String,
		params: Vec<Parameter>,
		return_type: PrimitiveType,
		body: Vec<Box<ASTNode>>,
	},
	Hook{
		name: String,
		params: Vec<Parameter>,
		body: Vec<Box<ASTNode>>, //return type is always the class the hook is attached to or null depending on if its a custom event or not
	},
	Event{
		name: String,
		params: Vec<Parameter>,
		body: Vec<Box<ASTNode>>,

		// event data
		generated_struct_name: String,
		generated_struct_fields: Vec<ASTNode>,
	},
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct ClassSignature{
	name: String,
	generics: Vec<Generic>,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct MethodSignature{
	name: String,
	params: Vec<Parameter>,
	return_type: PrimitiveType,
	bound_class_signature: Option<ClassSignature>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Struct{
	name: String,
	fields: Vec<ASTNode>, // list of ASTNode::VariableDecl
	attributes: HashMap<MethodSignature, AST>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FileContext{
	pub file: Arc<File>,
	pub imports: HashMap<String, String>, // alias -> path
}

impl FileContext{
	pub fn new(file: Arc<File>) -> Self{
		Self{file, imports: HashMap::new()}
	}
}

#[derive(Debug, Clone, PartialEq)]
pub struct CombinedContext{
    pub structs: HashMap<MethodSignature, Struct>,
	pub functions: HashMap<MethodSignature, AST>,
	pub hooks: HashMap<MethodSignature, AST>,
	pub events: HashMap<MethodSignature, AST>,
}

impl CombinedContext{
	pub fn new() -> Self{
		Self{
			structs: HashMap::new(),
			functions: HashMap::new(),
			hooks: HashMap::new(),
			events: HashMap::new(),
		}
	}
}