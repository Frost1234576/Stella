//     let mut script = Vec::new();

//     script.push(Instruction::Push(Literal::Long(5000000000)));
//     script.push(Instruction::Push(Literal::Long(1)));
//     script.push(Instruction::Add(Type::Long));
//     script.push(Instruction::Store(Type::Long, 1)); 

//     script.push(Instruction::GetStatic {
//         class: "java/lang/System".to_string(),
//         name: "out".to_string(),
//         desc: "Ljava/io/PrintStream;".to_string(),
//     });

//     script.push(Instruction::Load(Type::Long, 1));
//     script.push(Instruction::InvokeVirtual {
//         class: "java/io/PrintStream".to_string(),
//         name: "println".to_string(),
//         desc: "(J)V".to_string(),
//     });

//     script.push(Instruction::Return(Type::Nil));

//     // Use constants for method access
//     let method_flags = AccessFlags::ACC_PUBLIC | AccessFlags::ACC_STATIC;

//     class.add_method_from_instructions(
//         "main", 
//         "([Ljava/lang/String;)V", 
//         method_flags, 
//         script, 
//         4, 
//         4  
//     );

use std::sync::Arc;

use crate::ast::{AST, ASTNode, Expr, Op};
use crate::compiler::scope::Scope;
use crate::literals::{PrimitiveType, Literal, Parameter, Generic};
use crate::compiler::instructions::{Instruction, Label};
use std::borrow::BorrowMut;
use std::cell::RefCell;
use std::rc::Rc;

pub struct Method{
	pub name: String,
	pub descriptor: String,
	pub access_flags: u16,
	pub code: Vec<Instruction>,
	pub max_stack: u16,
	pub max_locals: u16,

	pub nodes: Vec<ASTNode>,
	pub params: Vec<Parameter>,
	pub return_type: PrimitiveType,

	label_counter: usize,
}

impl Method {
	pub fn new(name: &str, access_flags: u16, nodes: Vec<ASTNode>, params: Vec<Parameter>, return_type: PrimitiveType) -> Self {
		Method {
			name: name.to_string(),
			descriptor: "(".to_string()+&params.iter().map(|p| p.get_type().to_descriptor()).collect::<Vec<String>>().join("")+")"+&return_type.to_descriptor(),
			access_flags: access_flags,
			code: Vec::new(),
			max_stack: 0,
			max_locals: 0,

			nodes: nodes,
			params: params,
			return_type: return_type,

			label_counter: 0
		}
	}

	fn next_label(&mut self) -> Label {
		let l = Label::new(self.label_counter);
		self.label_counter += 1;
		l
	}

	pub fn add_instruction(&mut self, instr: Instruction) {
		self.code.push(instr);
	}

	// pub fn assignment()

	pub fn stack_length(stack: &Vec<PrimitiveType>) -> usize {
		let mut sum = 0;
		for t in stack {
			sum += t.size();
		}
		sum as usize
	}

	pub fn compile(&mut self) {
		let mut scopes = Vec::new();
		let mut stack = Vec::new();
		let mut max_stack = 0;
		let mut instructions = Vec::new();
		
		scopes.push(Scope::new(None));

		// add parameters to scope (they should all be Named)
		for param in &self.params {
			if let Parameter::Named { name, param_type, .. } = param {
				scopes[0].add_var(name.clone(), param_type.clone());
			} else {
				unimplemented!("Unnamed parameters are not supported");
			}
		}

		for node in &self.nodes {
			match node{
				ASTNode::Expr(expr) => {
					let (expr_instructions, expr_max_stack) = Method::compile_expr(&mut self.label_counter, expr, &scopes[0], &mut stack);
					if expr_max_stack > max_stack {
						max_stack = expr_max_stack;
					}
					instructions.extend(expr_instructions);
				},
				ASTNode::VarDecl(name, type_, expr) => {
					let slot = scopes[0].add_var(name.clone(), type_.clone().expect("VarDecl must have a type"));
					if(expr.is_none()) {
						instructions.push(Instruction::Push(Literal::Nil));
					}else{
						let (expr_instructions, expr_max_stack) = Method::compile_expr(&mut self.label_counter, expr.as_ref().unwrap(), &scopes[0], &mut stack);
						if expr_max_stack > max_stack {
							max_stack = expr_max_stack;
						}
						instructions.extend(expr_instructions);
					}
					
					instructions.push(Instruction::Store(type_.clone().expect("VarDecl must have a type"), slot));
				},
				ASTNode::Assignment(name, expr) => {
					let slot = scopes[0].get_var(name);
					let (expr_instructions, expr_max_stack) = Method::compile_expr(&mut self.label_counter, expr, &scopes[0], &mut stack);
					if expr_max_stack > max_stack {
						max_stack = expr_max_stack;
					}
					instructions.extend(expr_instructions);
					instructions.push(Instruction::Store(scopes[0].get_type(name).unwrap().clone(), slot.unwrap()));
				},
				ASTNode::Return(expr) => {
					if let Some(e) = expr {
						let (expr_instructions, expr_max_stack) = Method::compile_expr(&mut self.label_counter, e, &scopes[0], &mut stack);
						if expr_max_stack > max_stack {
							max_stack = expr_max_stack;
						}
						instructions.extend(expr_instructions);
						instructions.push(Instruction::Return(self.return_type.clone()));
					} else {
						instructions.push(Instruction::Return(PrimitiveType::Nil));
					}
				},
				ASTNode::ScopeEnd() => {

				},
				_ => {}
			}
			if Self::stack_length(&stack) > max_stack {
				max_stack = Self::stack_length(&stack);
			}
		}

		self.max_stack = max_stack as u16;
		self.max_locals = scopes[0].get_max_locals() as u16;

		self.code = instructions;

		for instruction in &self.code {
			println!("{:?}", instruction);
		}
	}

	pub fn check_stack_types(stack: Vec<PrimitiveType>) -> (Vec<Instruction>, PrimitiveType) {
		if stack.is_empty(){
			return (Vec::new(), PrimitiveType::Nil);
		}
		if stack.len() == 1{
			return (Vec::new(), stack[0].clone());
		}
		let mut instructions = Vec::new();
		let top = stack[stack.len() - 1].clone();
		let second = stack[stack.len() - 2].clone();

		if top != second {
			let promoted_type = top.compare_precedence(&second);
			if promoted_type != top {
				instructions.push(Instruction::cast(&top, &promoted_type));
			}
			if promoted_type != second {
				instructions.push(Instruction::Swap);
				instructions.push(Instruction::cast(&second, &promoted_type));
				instructions.push(Instruction::Swap); // restore order
			}
			return (instructions, promoted_type);
		}
		(instructions, top)
	}

	pub fn compile_expr(label_counter: &mut usize, expr: &Expr, scope: &Scope, stack: &mut Vec<PrimitiveType>) -> (Vec<Instruction>, usize) {
		let mut instructions = Vec::new();
		let mut max_stack = 0;

		for op in expr.ops() {
			match op {
				Op::Push(lit) => {
					instructions.push(Instruction::Push(lit.clone()));
					stack.push(lit.get_type());
				},
				Op::LoadIdentifier(name) => {
					let index = scope.get_var(name);
					instructions.push(Instruction::Load(scope.get_type(name).unwrap().clone(), index.unwrap()));
					stack.push(scope.get_type(name).unwrap().clone());
				},
				Op::Add => {
					let (cast_instructions, promoted) = Method::check_stack_types(stack.clone());
					instructions.extend(cast_instructions);
					instructions.push(Instruction::Add(promoted.clone()));
					stack.pop();
					stack.pop();
					stack.push(promoted);
				},
				Op::Subtract => {
					let (cast_instructions, promoted) = Method::check_stack_types(stack.clone());
					instructions.extend(cast_instructions);
					instructions.push(Instruction::Sub(promoted.clone()));
					stack.pop();
					stack.pop();
					stack.push(promoted);
				},
				Op::Multiply => {
					let (cast_instructions, promoted) = Method::check_stack_types(stack.clone());
					instructions.extend(cast_instructions);
					instructions.push(Instruction::Mul(promoted.clone()));
					stack.pop();
					stack.pop();
					stack.push(promoted);
				},
				Op::Divide => {
					let (cast_instructions, promoted) = Method::check_stack_types(stack.clone());
					instructions.extend(cast_instructions);
					instructions.push(Instruction::Div(promoted.clone()));
					stack.pop();
					stack.pop();
					stack.push(promoted);
				},

				Op::Not => {
					instructions.push(Instruction::LogicalNot);
				},

				Op::And => {
					let (cast_instructions, promoted) = Method::check_stack_types(stack.clone());
					instructions.extend(cast_instructions);
					instructions.push(Instruction::And(promoted.clone()));
					stack.pop();
					stack.pop();
					stack.push(promoted);
				},

				Op::Or => {
					let (cast_instructions, promoted) = Method::check_stack_types(stack.clone());
					instructions.extend(cast_instructions);
					instructions.push(Instruction::Or(promoted.clone()));
					stack.pop();
					stack.pop();
					stack.push(promoted);
				},

				Op::Xor => {
					let (cast_instructions, promoted) = Method::check_stack_types(stack.clone());
					instructions.extend(cast_instructions);
					instructions.push(Instruction::Xor(promoted.clone()));
					stack.pop();
					stack.pop();
					stack.push(promoted);
				},

				Op::Equal => {
					let t = stack.pop().unwrap(); stack.pop().unwrap();
					let (false_label, end_label) = Method::emit_cmp(&mut instructions, &t,
						Instruction::IfIcmpNe, Instruction::IfNe, false, label_counter);
					instructions.push(Instruction::Push(Literal::Bool(true)));
					instructions.push(Instruction::Goto(end_label));
					instructions.push(Instruction::Mark(false_label));
					instructions.push(Instruction::Push(Literal::Bool(false)));
					instructions.push(Instruction::Mark(end_label));
					stack.push(PrimitiveType::Bool);
				},

				Op::NotEqual => {
					let t = stack.pop().unwrap(); stack.pop().unwrap();
					let (false_label, end_label) = Method::emit_cmp(&mut instructions, &t,
						Instruction::IfIcmpEq, Instruction::IfEq, false, label_counter);
					instructions.push(Instruction::Push(Literal::Bool(true)));
					instructions.push(Instruction::Goto(end_label));
					instructions.push(Instruction::Mark(false_label));
					instructions.push(Instruction::Push(Literal::Bool(false)));
					instructions.push(Instruction::Mark(end_label));
					stack.push(PrimitiveType::Bool);
				},

				Op::Greater => {
					let t = stack.pop().unwrap(); stack.pop().unwrap();
					// nan_is_greater=true → FCmpg/DCmpg, NaN produces +1 → IfLe branches false (correct)
					let (false_label, end_label) = Method::emit_cmp(&mut instructions, &t,
						Instruction::IfIcmpLe, Instruction::IfLe, true, label_counter);
					instructions.push(Instruction::Push(Literal::Bool(true)));
					instructions.push(Instruction::Goto(end_label));
					instructions.push(Instruction::Mark(false_label));
					instructions.push(Instruction::Push(Literal::Bool(false)));
					instructions.push(Instruction::Mark(end_label));
					stack.push(PrimitiveType::Bool);
				},

				Op::GreaterEqual => {
					let t = stack.pop().unwrap(); stack.pop().unwrap();
					let (false_label, end_label) = Method::emit_cmp(&mut instructions, &t,
						Instruction::IfIcmpLt, Instruction::IfLt, true, label_counter);
					instructions.push(Instruction::Push(Literal::Bool(true)));
					instructions.push(Instruction::Goto(end_label));
					instructions.push(Instruction::Mark(false_label));
					instructions.push(Instruction::Push(Literal::Bool(false)));
					instructions.push(Instruction::Mark(end_label));
					stack.push(PrimitiveType::Bool);
				},

				Op::Less => {
					let t = stack.pop().unwrap(); stack.pop().unwrap();
					// nan_is_greater=false → FCmpl/DCmpl, NaN produces -1 → IfGe branches false (correct)
					let (false_label, end_label) = Method::emit_cmp(&mut instructions, &t,
						Instruction::IfIcmpGe, Instruction::IfGe, false, label_counter);
					instructions.push(Instruction::Push(Literal::Bool(true)));
					instructions.push(Instruction::Goto(end_label));
					instructions.push(Instruction::Mark(false_label));
					instructions.push(Instruction::Push(Literal::Bool(false)));
					instructions.push(Instruction::Mark(end_label));
					stack.push(PrimitiveType::Bool);
				},

				Op::LessEqual => {
					let t = stack.pop().unwrap(); stack.pop().unwrap();
					let (false_label, end_label) = Method::emit_cmp(&mut instructions, &t,
						Instruction::IfIcmpGt, Instruction::IfGt, false, label_counter);
					instructions.push(Instruction::Push(Literal::Bool(true)));
					instructions.push(Instruction::Goto(end_label));
					instructions.push(Instruction::Mark(false_label));
					instructions.push(Instruction::Push(Literal::Bool(false)));
					instructions.push(Instruction::Mark(end_label));
					stack.push(PrimitiveType::Bool);
				},

				Op::GetStaticField(class, name, desc) => {
					instructions.push(Instruction::GetStatic {
						class: class.clone(),
						name: name.clone(),
						desc: desc.clone(),
					});
					// Record that a reference is now on the stack
					stack.push(PrimitiveType::Reference(desc.clone()));
				},

				Op::CallMethod(class, name, desc) => {
					let arg_count = Self::count_parameters(&desc);
					
					// 1. Pop arguments from our tracking stack
					for _ in 0..arg_count {
						stack.pop();
					}
					
					// 2. Pop the object reference (the 'receiver')
					stack.pop();

					instructions.push(Instruction::InvokeVirtual {
						class: class.clone(),
						name: name.clone(),
						desc: desc.clone(),
					});

					// 3. Push return type back to stack if not void (V)
					if !desc.ends_with('V') {
						// stack.push(parsed_return_type_from_desc);
					}
				},
								
				_ => {}
			}
			if Self::stack_length(&stack) > max_stack {
				max_stack = Self::stack_length(&stack);
			}
		}
		
		(instructions, max_stack)
	}

	fn emit_cmp(
		instructions: &mut Vec<Instruction>,
		t: &PrimitiveType,
		// The IfIcmp* variant for ints, and the If* variant after lcmp/fcmp/dcmp
		int_branch: fn(Label) -> Instruction,
		val_branch: fn(Label) -> Instruction,
		nan_is_greater: bool,
		label_counter: &mut usize,
	) -> (Label, Label) {
		let false_label = Label::new(*label_counter); *label_counter += 1;
		let end_label   = Label::new(*label_counter); *label_counter += 1;

		if let Some(cmp) = Instruction::cmp_for_type(t, nan_is_greater) {
			instructions.push(cmp);
			instructions.push(val_branch(false_label));
		} else {
			instructions.push(int_branch(false_label));
		}
		(false_label, end_label)
	}

	fn count_parameters(descriptor: &str) -> usize {
		let params_part = descriptor.split('(').nth(1).unwrap_or("").split(')').next().unwrap_or("");
		let mut count = 0;
		let mut chars = params_part.chars().peekable();

		while let Some(c) = chars.next() {
			match c {
				'L' => { // Object type: L...;
					while chars.next() != Some(';') {}
					count += 1;
				}
				'[' => { // Array type: skip the [ and handle the base type
					while let Some(&'[') = chars.peek() { chars.next(); }
					if chars.peek() == Some(&'L') {
						chars.next();
						while chars.next() != Some(';') {}
					} else {
						chars.next();
					}
					count += 1;
				}
				'J' | 'D' => { // Long and Double take 2 stack slots in JVM, but 1 "parameter"
					count += 1; 
				}
				_ => count += 1, // Primitives (I, Z, B, etc.)
			}
		}
		count
	}
}