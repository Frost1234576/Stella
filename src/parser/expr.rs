use std::iter::Peekable;
use std::slice::Iter;
use crate::ast::{ASTNode, Expr, Op};
use crate::literals::{Generic, GenericType, Literal, PrimitiveType};
use crate::tokenizer::lexer::{Token, TokenType};
use crate::parser::ParserError;

pub struct ExprParser<'a> {
    tokens: Peekable<Iter<'a, Token>>,
    current_line: usize,
}

impl<'a> ExprParser<'a> {
    pub fn new(tokens: &'a [Token]) -> Self {
        ExprParser {
            tokens: tokens.iter().peekable(),
            current_line: 1,
        }
    }

    // -------------------------------------------------------------------------
    // Type parsing (unchanged from original)
    // -------------------------------------------------------------------------

    pub fn parse_type(tokens: &'a [Token], errors: &mut Vec<ParserError>, allow_generics: bool) -> Option<GenericType> {
        if tokens.is_empty() {
            errors.push(ParserError { line: 0, message: "Expected type".to_string() });
            return None;
        }

        let base = match tokens[0].lexeme.as_str() {
            "bool"   => PrimitiveType::Bool,
            "int"    => PrimitiveType::Int,
            "double" => PrimitiveType::Double,
            "float"  => PrimitiveType::Float,
            "long"   => PrimitiveType::Long,
            "char"   => PrimitiveType::Char,
            "string" => PrimitiveType::String,
            "nil"    => PrimitiveType::Nil,
            _        => PrimitiveType::Reference(tokens[0].lexeme.clone()),
        };

        let mut generic = None;

        if tokens.len() > 1 {
            if !allow_generics {
                errors.push(ParserError {
                    line: tokens[1].line,
                    message: "Generics not allowed here".to_string(),
                });
                return Some(GenericType { base, generic: None });
            }

            if tokens[1].token_type == TokenType::Less {
                let mut idx = 2;
                while idx < tokens.len() {
                    if tokens[idx].token_type == TokenType::Greater { break; }

                    if tokens[idx].token_type != TokenType::Identifier {
                        errors.push(ParserError {
                            line: tokens[idx].line,
                            message: "Expected generic type name".to_string(),
                        });
                        return None;
                    }

                    let mut generic_bounds = Vec::new();

                    if tokens[idx + 1].token_type == TokenType::Colon {
                        while idx < tokens.len() {
                            idx += 1;
                            if tokens[idx].token_type == TokenType::Greater
                                || tokens[idx].token_type == TokenType::Comma
                            {
                                break;
                            }

                            if tokens[idx].token_type != TokenType::Identifier {
                                errors.push(ParserError {
                                    line: tokens[idx].line,
                                    message: "Expected generic bound type".to_string(),
                                });
                                return None;
                            }

                            generic_bounds.push(tokens[idx].lexeme.clone());

                            if tokens.len() <= idx + 1 {
                                errors.push(ParserError {
                                    line: tokens[idx].line,
                                    message: "Unclosed generic bounds; forgetting a '>'?".to_string(),
                                });
                            }

                            idx += 1;
                            if tokens[idx].token_type != TokenType::Plus { break; }
                        }
                    }

                    idx += 1;
                    generic = Some(Generic {
                        constraints: generic_bounds
                            .iter()
                            .map(|s| PrimitiveType::from_string(s))
                            .collect::<Vec<PrimitiveType>>(),
                    });
                }
            }
        }

        Some(GenericType { base, generic })
    }

    // -------------------------------------------------------------------------
    // Assignment
    // -------------------------------------------------------------------------

    pub fn parse_assignment(tokens: &'a [Token], errors: &mut Vec<ParserError>) -> Option<ASTNode> {
        if tokens[0].token_type != TokenType::Identifier {
            errors.push(ParserError {
                line: tokens[0].line,
                message: "Expected identifier for assignment".to_string(),
            });
            return None;
        }

        if tokens[1].token_type != TokenType::Equal {
            errors.push(ParserError {
                line: tokens[1].line,
                message: "Expected '=' for assignment".to_string(),
            });
            return None;
        }

        if tokens.len() < 3 {
            errors.push(ParserError {
                line: tokens[0].line,
                message: "Expected expression for assignment".to_string(),
            });
            return None;
        }

        let var_name = tokens[0].lexeme.clone();
        let expr = ExprParser::parse_expr_from_slice(&tokens[2..], errors)?;

        Some(ASTNode::Assignment(var_name, expr))
    }

    // -------------------------------------------------------------------------
    // Public entry points
    // -------------------------------------------------------------------------

    pub fn parse_expr_from_slice(tokens: &'a [Token], errors: &mut Vec<ParserError>) -> Option<Expr> {
        let mut parser = ExprParser::new(tokens);
        parser.parse_expr(errors)
    }

    // -------------------------------------------------------------------------
    // Iterator helpers
    // -------------------------------------------------------------------------

    fn peek(&mut self) -> Option<&Token> {
        self.tokens.peek().copied()
    }

    fn advance(&mut self) -> Option<&'a Token> {
        let token = self.tokens.next();
        if let Some(t) = token {
            self.current_line = t.line;
        }
        token
    }

    fn check(&mut self, token_type: TokenType) -> bool {
        matches!(self.peek(), Some(t) if t.token_type == token_type)
    }

    fn match_types(&mut self, types: &[TokenType]) -> Option<&'a Token> {
        for t in types {
            if self.check(*t) {
                return self.advance();
            }
        }
        None
    }

    // -------------------------------------------------------------------------
    // Expression parsing — each level returns Option<Vec<Op>> (postfix order).
    // The public parse_expr wraps the result in Expr::new().
    // -------------------------------------------------------------------------

    pub fn parse_expr(&mut self, errors: &mut Vec<ParserError>) -> Option<Expr> {
        let ops = self.expr_ops(errors)?;
        Some(Expr::new(ops))
    }

    // Logical OR  (lowest precedence)
    fn expr_ops(&mut self, errors: &mut Vec<ParserError>) -> Option<Vec<Op>> {
        self.parse_logical_or(errors)
    }

    fn parse_logical_or(&mut self, errors: &mut Vec<ParserError>) -> Option<Vec<Op>> {
        let mut ops = self.parse_logical_and(errors)?;

        while self.match_types(&[TokenType::DoubleBar]).is_some() {
            let right = self.parse_logical_and(errors)?;
            ops.extend(right);
            ops.push(Op::Or);
        }

        Some(ops)
    }

    // Logical AND
    fn parse_logical_and(&mut self, errors: &mut Vec<ParserError>) -> Option<Vec<Op>> {
        let mut ops = self.parse_equality(errors)?;

        while self.match_types(&[TokenType::DoubleAmpersand]).is_some() {
            let right = self.parse_equality(errors)?;
            ops.extend(right);
            ops.push(Op::And);
        }

        Some(ops)
    }

    // Equality (==, !=)
    fn parse_equality(&mut self, errors: &mut Vec<ParserError>) -> Option<Vec<Op>> {
        let mut ops = self.parse_comparison(errors)?;

        while let Some(op_tok) = self.match_types(&[TokenType::EqualEqual, TokenType::BangEqual]) {
            let op = match op_tok.token_type {
                TokenType::EqualEqual => Op::Equal,
                TokenType::BangEqual  => Op::NotEqual,
                _ => unreachable!(),
            };
            let right = self.parse_comparison(errors)?;
            ops.extend(right);
            ops.push(op);
        }

        Some(ops)
    }

    // Comparison (<, >, <=, >=)
    fn parse_comparison(&mut self, errors: &mut Vec<ParserError>) -> Option<Vec<Op>> {
        let mut ops = self.parse_term(errors)?;

        while let Some(op_tok) = self.match_types(&[
            TokenType::Greater,
            TokenType::GreaterEqual,
            TokenType::Less,
            TokenType::LessEqual,
        ]) {
            let op = match op_tok.token_type {
                TokenType::Greater      => Op::Greater,
                TokenType::GreaterEqual => Op::GreaterEqual,
                TokenType::Less         => Op::Less,
                TokenType::LessEqual    => Op::LessEqual,
                _ => unreachable!(),
            };
            let right = self.parse_term(errors)?;
            ops.extend(right);
            ops.push(op);
        }

        Some(ops)
    }

    // Addition / subtraction
    fn parse_term(&mut self, errors: &mut Vec<ParserError>) -> Option<Vec<Op>> {
        let mut ops = self.parse_factor(errors)?;

        while let Some(op_tok) = self.match_types(&[TokenType::Plus, TokenType::Minus]) {
            let op = match op_tok.token_type {
                TokenType::Plus  => Op::Add,
                TokenType::Minus => Op::Subtract,
                _ => unreachable!(),
            };
            let right = self.parse_factor(errors)?;
            ops.extend(right);
            ops.push(op);
        }

        Some(ops)
    }

    // Multiplication / division
    fn parse_factor(&mut self, errors: &mut Vec<ParserError>) -> Option<Vec<Op>> {
        let mut ops = self.parse_unary(errors)?;

        while let Some(op_tok) = self.match_types(&[TokenType::Star, TokenType::Slash]) {
            let op = match op_tok.token_type {
                TokenType::Star  => Op::Multiply,
                TokenType::Slash => Op::Divide,
                _ => unreachable!(),
            };
            let right = self.parse_unary(errors)?;
            ops.extend(right);
            ops.push(op);
        }

        Some(ops)
    }

    // Unary (!, -, +)
    fn parse_unary(&mut self, errors: &mut Vec<ParserError>) -> Option<Vec<Op>> {
        if let Some(op_tok) = self.match_types(&[TokenType::Bang, TokenType::Minus, TokenType::Plus]) {
            let op = match op_tok.token_type {
                TokenType::Bang  => Op::Not,
                // Unary minus / plus — emit a zero and then Subtract/Add so the
                // stack machine always sees binary ops.  Alternatively you could
                // add Op::Negate; adjust here if you add that variant.
                TokenType::Minus => {
                    let mut ops = vec![Op::Push(Literal::Int(0))];
                    ops.extend(self.parse_unary(errors)?);
                    ops.push(Op::Subtract);
                    return Some(ops);
                }
                TokenType::Plus => {
                    // Unary plus is a no-op; just return the inner expression.
                    return self.parse_unary(errors);
                }
                _ => unreachable!(),
            };

            let mut ops = self.parse_unary(errors)?;
            ops.push(op);
            return Some(ops);
        }

        self.parse_postfix(errors)
    }

    // Postfix: dot-access and method calls
    // Emits the object ops first, then GetField / CallMethod on top.
    fn parse_postfix(&mut self, errors: &mut Vec<ParserError>) -> Option<Vec<Op>> {
        let mut ops = self.parse_primary(errors)?;

        loop {
            if self.match_types(&[TokenType::Dot]).is_none() {
                break;
            }

            let name_tok = match self.peek() {
                Some(t) if t.token_type == TokenType::Identifier => {
                    let name = t.lexeme.clone();
                    self.advance();
                    name
                }
                _ => {
                    errors.push(ParserError {
                        line: self.current_line,
                        message: "Expected identifier after '.'".to_string(),
                    });
                    return None;
                }
            };

            if self.check(TokenType::LeftParen) {
                self.advance(); // consume '('
                let arg_ops = self.parse_arguments(errors)?;

                // Object ops are already in `ops`; arguments follow on the stack.
                ops.extend(arg_ops);

                let class_desc  = Self::get_class_descriptor(&name_tok);
                let method_desc = Self::get_method_descriptor(&name_tok);
                ops.push(Op::CallMethod(class_desc, name_tok, method_desc));
            } else {
                ops.push(Op::GetField(name_tok));
            }
        }

        Some(ops)
    }

    // Primary: literals, identifiers, function calls, grouped expressions
    fn parse_primary(&mut self, errors: &mut Vec<ParserError>) -> Option<Vec<Op>> {
        let token = match self.peek() {
            Some(t) => t.clone(),
            None => {
                errors.push(ParserError {
                    line: self.current_line,
                    message: "Expected expression".to_string(),
                });
                return None;
            }
        };

        match token.token_type {
            // --- Boolean / nil literals ---
            TokenType::True => {
                self.advance();
                return Some(vec![Op::Push(Literal::Bool(true))]);
            }
            TokenType::False => {
                self.advance();
                return Some(vec![Op::Push(Literal::Bool(false))]);
            }
            TokenType::Nil => {
                self.advance();
                return Some(vec![Op::Push(Literal::Nil)]);
            }

            // --- Numeric literals ---
            TokenType::Float => {
                let raw = token.literal.clone().unwrap_or_else(|| token.lexeme.clone());
                self.advance();

                if let Ok(v) = raw.parse::<f32>() {
                    return Some(vec![Op::Push(Literal::Float(v))]);
                }

                errors.push(ParserError {
                    line: self.current_line,
                    message: format!("Invalid float literal: {}", raw),
                });
                return None;
            }
            TokenType::Double => {
                let raw = token.literal.clone().unwrap_or_else(|| token.lexeme.clone());
                self.advance();

                if let Ok(v) = raw.parse::<f64>() {
                    return Some(vec![Op::Push(Literal::Double(v))]);
                }

                errors.push(ParserError {
                    line: self.current_line,
                    message: format!("Invalid double literal: {}", raw),
                });
                return None;
            }
            TokenType::Int => {
                let raw = token.literal.clone().unwrap_or_else(|| token.lexeme.clone());
                self.advance();
                if let Ok(v) = raw.parse::<i32>() {
                    return Some(vec![Op::Push(Literal::Int(v))]);
                }
                errors.push(ParserError {
                    line: self.current_line,
                    message: format!("Invalid int literal: {}", raw),
                });
                return None;
            }
            TokenType::Long => {
                let raw = token.literal.clone().unwrap_or_else(|| token.lexeme.clone());
                self.advance();
                if let Ok(v) = raw.parse::<i64>() {
                    return Some(vec![Op::Push(Literal::Long(v))]);
                }
                errors.push(ParserError {
                    line: self.current_line,
                    message: format!("Invalid long literal: {}", raw),
                });
                return None;
            }
            TokenType::Float => {
                let raw = token.literal.clone().unwrap_or_else(|| token.lexeme.clone());
                self.advance();
                if let Ok(v) = raw.parse::<f32>() {
                    return Some(vec![Op::Push(Literal::Float(v))]);
                }
                errors.push(ParserError {
                    line: self.current_line,
                    message: format!("Invalid float literal: {}", raw),
                });
                return None;
            }
            TokenType::Double => {
                let raw = token.literal.clone().unwrap_or_else(|| token.lexeme.clone());
                self.advance();
                if let Ok(v) = raw.parse::<f64>() {
                    return Some(vec![Op::Push(Literal::Double(v))]);
                }
                errors.push(ParserError {
                    line: self.current_line,
                    message: format!("Invalid double literal: {}", raw),
                });
                return None;
            }
            TokenType::String => {
                let value = token.literal.clone().unwrap_or_else(|| token.lexeme.clone());
                self.advance();
                return Some(vec![Op::Push(Literal::String(value))]);
            }

            // --- Identifier: variable reference or standalone function call ---
            TokenType::Identifier => {
                let name = token.lexeme.clone();
                self.advance();

                if self.check(TokenType::LeftParen) {
                    self.advance(); // consume '('
                    let arg_ops = self.parse_arguments(errors)?;

                    // A bare function call is emitted as CallMethod with no
                    // implicit receiver pushed; the class/descriptor helpers
                    // are responsible for resolving the target.
                    let class_desc  = Self::get_class_descriptor(&name);
                    let method_desc = Self::get_method_descriptor(&name);
                    let mut ops = arg_ops;
                    ops.push(Op::CallMethod(class_desc, name, method_desc));
                    return Some(ops);
                }

                return Some(vec![Op::LoadIdentifier(name)]);
            }

            // --- Grouped expression ---
            TokenType::LeftParen => {
                self.advance(); // consume '('
                let ops = self.expr_ops(errors)?;
                if self.match_types(&[TokenType::RightParen]).is_none() {
                    errors.push(ParserError {
                        line: self.current_line,
                        message: "Expected ')' after expression".to_string(),
                    });
                    return None;
                }
                // No extra op needed — grouping is purely syntactic.
                return Some(ops);
            }

            _ => {}
        }

        errors.push(ParserError {
            line: self.current_line,
            message: "Expected expression".to_string(),
        });
        None
    }

    // Parse comma-separated argument expressions, returning all their ops
    // concatenated in left-to-right order (each argument is fully evaluated
    // before the next, matching call-stack conventions).
    fn parse_arguments(&mut self, errors: &mut Vec<ParserError>) -> Option<Vec<Op>> {
        let mut all_ops: Vec<Op> = Vec::new();

        if !self.check(TokenType::RightParen) {
            loop {
                let arg_ops = self.expr_ops(errors)?;
                all_ops.extend(arg_ops);

                if self.match_types(&[TokenType::Comma]).is_none() {
                    break;
                }
            }
        }

        if self.match_types(&[TokenType::RightParen]).is_none() {
            errors.push(ParserError {
                line: self.current_line,
                message: "Expected ')' after arguments".to_string(),
            });
            return None;
        }

        Some(all_ops)
    }

    // Stubs — assumed to be implemented elsewhere per the user's note.
    fn get_class_descriptor(name: &str) -> String {
        todo!("get_class_descriptor({name})")
    }

    fn get_method_descriptor(name: &str) -> String {
        todo!("get_method_descriptor({name})")
    }
}