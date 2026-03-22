mod compiler;
mod method;
mod ast;
mod tokenizer;
mod literals;

use compiler::class::{JavaClass, create_jar};
use method::Method;
use ast::ASTNode;
use literals::Literal;
use tokenizer::File;
use compiler::instructions::Instruction;

use crate::{ast::{Expr, Op}, compiler::class::AccessFlags, literals::{Parameter, PrimitiveType}};

fn main() -> std::io::Result<()> {
    let mut class = JavaClass::new("CompilerTest", Some("java/lang/Object"));
    let mut nodes = Vec::new();

    let math_ops = vec![
        Op::GetStaticField("java/lang/System".into(), "out".into(), "Ljava/io/PrintStream;".into()),
        Op::Push(Literal::Long(5_000_000_000)),
        Op::Push(Literal::Long(1)),
        Op::Add,      // LADD
        Op::Push(Literal::Long(2)),
        Op::Multiply, // LMUL
        Op::CallMethod("java/io/PrintStream".into(), "println".into(), "(J)V".into()),
    ];
    nodes.push(ASTNode::Expr(Expr::new(math_ops)));

    // (10 > 5) == true
    let bool_ops = vec![
        Op::GetStaticField("java/lang/System".into(), "out".into(), "Ljava/io/PrintStream;".into()),
        Op::Push(Literal::Long(5_000_000_000)),
        Op::Push(Literal::Long(5)),
        Op::Greater,  // Should emit IF_ICMPLE and push 1 or 0
        Op::CallMethod("java/io/PrintStream".into(), "println".into(), "(Z)V".into()),
    ];
    nodes.push(ASTNode::Expr(Expr::new(bool_ops)));

    // --- Case 3: The "Empty Return" Edge Case ---
    nodes.push(ASTNode::Return(None));

    let mut main_method = Method::new(
        "main",
        AccessFlags::ACC_PUBLIC | AccessFlags::ACC_STATIC,
        nodes,
        vec![Parameter::Named {
            name: "args".into(),
            param_type: PrimitiveType::Array(Box::new(PrimitiveType::String)),
            generic: None,
        }],
        PrimitiveType::Nil
    );

    main_method.compile(); // This will calculate max_stack and max_locals
    class.add_method(main_method);

    let class_bytes = class.generate_bytes();
    let files = vec![("CompilerTest.class", class_bytes)];
    create_jar("CompilerTest.jar", "CompilerTest", files)?;

    println!("Successfully compiled EdgeCaseDemo.jar");
    Ok(())
}