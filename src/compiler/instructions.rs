use crate::literals::{PrimitiveType, Literal};
use crate::JavaClass;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Label(pub usize);

impl Label {
    pub fn new(id: usize) -> Self {
        Self(id)
    }
}

#[derive(Debug, Clone)]
pub enum Instruction {
    // --- Metadata ---
    Mark(Label),        // Pseudo-instruction: marks a jump target

    // --- Stack Manipulation ---
    Push(Literal),
    Pop,
    Dup,
    Swap,

    // --- Local Variables ---
    Load(PrimitiveType, u8),
    Store(PrimitiveType, u8),

    // --- Arithmetic & Logic ---
    Add(PrimitiveType),
    Sub(PrimitiveType),
    Mul(PrimitiveType),
    Div(PrimitiveType),
    Rem(PrimitiveType),
    Neg(PrimitiveType),
    Shl(PrimitiveType),  // Shift Left
    Shr(PrimitiveType),  // Arithmetic Shift Right
    Ushr(PrimitiveType), // Logical Shift Right
    And(PrimitiveType),
    Or(PrimitiveType),
    Xor(PrimitiveType),
    LogicalNot,          // Simulates !bool (iconst_1, ixor)

    // --- Type Conversions ---
    I2L, I2F, I2D,
    L2I, L2F, L2D,
    F2I, F2L, F2D,
    D2I, D2L, D2F,

    // --- Comparisons (for long/float/double) ---
    // Push -1, 0, or 1 onto the stack; use IfEq/IfLt/etc. afterward
    LCmp,               // long comparison:   -1 if <, 0 if ==, 1 if >
    FCmpl,              // float comparison:  -1 on NaN (use for <, <=)
    FCmpg,              // float comparison:  +1 on NaN (use for >, >=)
    DCmpl,              // double comparison: -1 on NaN (use for <, <=)
    DCmpg,              // double comparison: +1 on NaN (use for >, >=)

    // --- Control Flow ---
    Goto(Label),
    IfEq(Label),         // if x == 0
    IfNe(Label),         // if x != 0
    IfLt(Label),         // if x < 0
    IfGe(Label),         // if x >= 0
    IfGt(Label),         // if x > 0
    IfLe(Label),         // if x <= 0
    IfIcmpEq(Label),     // if i1 == i2
    IfIcmpNe(Label),     // if i1 != i2
    IfIcmpLt(Label),     // if i1 < i2
    IfIcmpGe(Label),     // if i1 >= i2
    IfIcmpGt(Label),     // if i1 > i2
    IfIcmpLe(Label),     // if i1 <= i2
    IfNull(Label),
    IfNonNull(Label),
    Return(PrimitiveType),

    // --- Object / Field / Method ---
    GetStatic { class: String, name: String, desc: String },
    PutStatic { class: String, name: String, desc: String },
    GetField { class: String, name: String, desc: String },
    PutField { class: String, name: String, desc: String },
    InvokeVirtual { class: String, name: String, desc: String },
    InvokeStatic { class: String, name: String, desc: String },
    InvokeSpecial { class: String, name: String, desc: String },
}

impl Instruction {
    /// Two-pass assembler to resolve label offsets.
    pub fn assemble(instructions: &[Instruction], jc: &mut JavaClass) -> Vec<u8> {
        let mut offsets = HashMap::new();
        let mut current_pc = 0;

        // Pass 1: Calculate PC for every instruction and map Labels to their PC
        for inst in instructions {
            if let Instruction::Mark(label) = inst {
                offsets.insert(label.0, current_pc as i16);
            } else {
                current_pc += inst.byte_size(jc);
            }
        }

        // Pass 2: Generate bytecode using calculated offsets
        let mut bytecode = Vec::new();
        let mut pc = 0;
        for inst in instructions {
            if let Instruction::Mark(_) = inst { continue; }
            
            let bytes = inst.emit_with_labels(jc, pc, &offsets);
            pc += bytes.len() as u32;
            bytecode.extend(bytes);
        }
        bytecode
    }

    fn byte_size(&self, jc: &mut JavaClass) -> u32 {
        match self {
            Instruction::Mark(_) => 0,
            
            // 1-byte opcodes
            Instruction::Pop | Instruction::Dup | Instruction::Swap |
            Instruction::Add(_) | Instruction::Sub(_) | Instruction::Mul(_) | 
            Instruction::Div(_) | Instruction::Rem(_) | Instruction::Neg(_) |
            Instruction::Shl(_) | Instruction::Shr(_) | Instruction::Ushr(_) |
            Instruction::And(_) | Instruction::Or(_) | Instruction::Xor(_) |
            Instruction::I2L | Instruction::I2F | Instruction::I2D |
            Instruction::L2I | Instruction::L2F | Instruction::L2D |
            Instruction::F2I | Instruction::F2L | Instruction::F2D |
            Instruction::D2I | Instruction::D2L | Instruction::D2F |
            Instruction::LCmp | Instruction::FCmpl | Instruction::FCmpg |
            Instruction::DCmpl | Instruction::DCmpg |
            Instruction::Return(_) => 1,

            // 2-byte opcodes
            Instruction::Load(_, _) | Instruction::Store(_, _) => 2,
            Instruction::Push(lit) => match lit {
                Literal::Bool(_) => 1,                   // iconst_0/1
                Literal::Int(v) if *v >= -128 && *v <= 127 => 2, // bipush
                Literal::Int(_) | Literal::String(_) => 2,        // ldc
                Literal::Float(_) | Literal::Double(_) | Literal::Long(_) => 3,
                _ => 2,
            },
            Instruction::LogicalNot => 2, // iconst_1 (1) + ixor (1)

            // 3-byte opcodes (Jumps and Field/Method refs)
            Instruction::Goto(_) | Instruction::IfEq(_) | Instruction::IfNe(_) | 
            Instruction::IfLt(_) | Instruction::IfGe(_) | Instruction::IfGt(_) | 
            Instruction::IfLe(_) | Instruction::IfIcmpEq(_) | Instruction::IfIcmpNe(_) | 
            Instruction::IfIcmpLt(_) | Instruction::IfIcmpGe(_) | Instruction::IfIcmpGt(_) | 
            Instruction::IfIcmpLe(_) | Instruction::IfNull(_) | Instruction::IfNonNull(_) |
            Instruction::GetStatic {..} | Instruction::PutStatic {..} | 
            Instruction::GetField {..} | Instruction::PutField {..} |
            Instruction::InvokeVirtual {..} | Instruction::InvokeStatic {..} | 
            Instruction::InvokeSpecial {..} => 3,
        }
    }

    fn emit_with_labels(&self, jc: &mut JavaClass, current_pc: u32, labels: &HashMap<usize, i16>) -> Vec<u8> {
        let get_offset = |target: &Label| -> Vec<u8> {
            let target_pc = *labels.get(&target.0).expect("Label not found");
            // Offset must be relative to the START of the jump instruction
            let offset = target_pc - (current_pc as i16); 
            offset.to_be_bytes().to_vec()
        };

        match self {
            Instruction::Mark(_) => vec![],

            // --- Stack & Constants ---
            Instruction::Push(lit) => match lit {
                Literal::Int(v) => {
                    let idx = jc.add_integer_constant(*v);
                    if idx <= 255 {
                        vec![0x12, idx as u8] // ldc
                    } else {
                        let mut bytes = vec![0x13]; // ldc_w
                        bytes.extend_from_slice(&(idx as u16).to_be_bytes());
                        bytes
                    }
                }
                Literal::Bool(b) => vec![if *b { 0x04 } else { 0x03 }],
                Literal::String(s) => vec![0x12, jc.add_string_constant(s) as u8],
                Literal::Long(v) => {
                    let idx = jc.add_long_constant(*v);
                    let mut bytes = vec![0x14]; // ldc2_w
                    bytes.extend_from_slice(&idx.to_be_bytes()); // Push 2 bytes for index
                    bytes
                },
                Literal::Double(v) => {
                    let idx = jc.add_double_constant(*v);
                    let mut bytes = vec![0x14]; // ldc2_w
                    bytes.extend_from_slice(&idx.to_be_bytes());
                    bytes
                },
                Literal::Float(v) => vec![0x11, jc.add_float_constant(*v) as u8],
                Literal::Char(c) => vec![0x10, *c as u8],
                _ => panic!("Unsupported literal type: {:?}", lit),
            },
            Instruction::Pop => vec![0x57],
            Instruction::Dup => vec![0x59],
            Instruction::Swap => vec![0x5F],

            // --- Locals ---
            Instruction::Load(t, slot) => vec![match t { 
                PrimitiveType::Long => 0x16, PrimitiveType::Float => 0x17, PrimitiveType::Double => 0x18, 
                _ => 0x15, // Int, Bool, Char
            }, *slot],
            Instruction::Store(t, slot) => vec![match t { 
                PrimitiveType::Long => 0x37, PrimitiveType::Float => 0x38, PrimitiveType::Double => 0x39, 
                _ => 0x36,
            }, *slot],

            // --- Arithmetic ---
            // --- Arithmetic ---
            Instruction::Add(t) => vec![match t { PrimitiveType::Long => 0x61, PrimitiveType::Float => 0x62, PrimitiveType::Double => 0x63, _ => 0x60 }],
            Instruction::Sub(t) => vec![match t { PrimitiveType::Long => 0x65, PrimitiveType::Float => 0x66, PrimitiveType::Double => 0x67, _ => 0x64 }],
            Instruction::Mul(t) => vec![match t { PrimitiveType::Long => 0x69, PrimitiveType::Float => 0x6A, PrimitiveType::Double => 0x6B, _ => 0x68 }],
            Instruction::Div(t) => vec![match t { PrimitiveType::Long => 0x6D, PrimitiveType::Float => 0x6E, PrimitiveType::Double => 0x6F, _ => 0x6C }],
            Instruction::Rem(t) => vec![match t { PrimitiveType::Long => 0x75, PrimitiveType::Float => 0x77, PrimitiveType::Double => 0x79, _ => 0x71 }],
            Instruction::Neg(t) => vec![match t { PrimitiveType::Long => 0x79, PrimitiveType::Float => 0x7B, PrimitiveType::Double => 0x7D, _ => 0x75 }],

            // --- Bitwise ---
            Instruction::Shl(t)  => vec![if matches!(t, PrimitiveType::Long) { 0x79 } else { 0x78 }],
            Instruction::Shr(t)  => vec![if matches!(t, PrimitiveType::Long) { 0x7B } else { 0x7A }],
            Instruction::Ushr(t) => vec![if matches!(t, PrimitiveType::Long) { 0x7D } else { 0x7C }],
            Instruction::And(t)  => vec![if matches!(t, PrimitiveType::Long) { 0x7F } else { 0x7E }],
            Instruction::Or(t)   => vec![if matches!(t, PrimitiveType::Long) { 0x81 } else { 0x80 }],
            Instruction::Xor(t)  => vec![if matches!(t, PrimitiveType::Long) { 0x83 } else { 0x82 }],
            Instruction::LogicalNot => vec![0x04, 0x82], // iconst_1, ixor

            // --- Conversions ---
            Instruction::I2L => vec![0x85], Instruction::I2F => vec![0x86], Instruction::I2D => vec![0x87],
            Instruction::L2I => vec![0x88], Instruction::L2F => vec![0x89], Instruction::L2D => vec![0x8A],
            Instruction::F2I => vec![0x8B], Instruction::F2L => vec![0x8C], Instruction::F2D => vec![0x8D],
            Instruction::D2I => vec![0x8E], Instruction::D2L => vec![0x8F], Instruction::D2F => vec![0x90],

            // --- Comparisons ---
            Instruction::LCmp  => vec![0x94],
            Instruction::FCmpl => vec![0x95],
            Instruction::FCmpg => vec![0x96],
            Instruction::DCmpl => vec![0x97],
            Instruction::DCmpg => vec![0x98],

            // --- Control Flow ---
            Instruction::Goto(l)     => [vec![0xA7], get_offset(l)].concat(),
            Instruction::IfEq(l)     => [vec![0x99], get_offset(l)].concat(),
            Instruction::IfNe(l)     => [vec![0x9A], get_offset(l)].concat(),
            Instruction::IfLt(l)     => [vec![0x9B], get_offset(l)].concat(),
            Instruction::IfGe(l)     => [vec![0x9C], get_offset(l)].concat(),
            Instruction::IfGt(l)     => [vec![0x9D], get_offset(l)].concat(),
            Instruction::IfLe(l)     => [vec![0x9E], get_offset(l)].concat(),
            Instruction::IfIcmpEq(l) => [vec![0x9F], get_offset(l)].concat(),
            Instruction::IfIcmpNe(l) => [vec![0xA0], get_offset(l)].concat(),
            Instruction::IfIcmpLt(l) => [vec![0xA1], get_offset(l)].concat(),
            Instruction::IfIcmpGe(l) => [vec![0xA2], get_offset(l)].concat(),
            Instruction::IfIcmpGt(l) => [vec![0xA3], get_offset(l)].concat(),
            Instruction::IfIcmpLe(l) => [vec![0xA4], get_offset(l)].concat(),
            Instruction::IfNull(l)    => [vec![0xC6], get_offset(l)].concat(),
            Instruction::IfNonNull(l) => [vec![0xC7], get_offset(l)].concat(),
            
            Instruction::Return(t) => vec![match t {
                PrimitiveType::Nil => 0xB1, PrimitiveType::Long => 0xAD, 
                PrimitiveType::Float => 0xAE, PrimitiveType::Double => 0xAF, 
                PrimitiveType::Int | PrimitiveType::Bool | PrimitiveType::Char => 0xAC, 
                _ => 0xB0, // areturn for objects
            }],

            // --- Field & Method Ops ---
            Instruction::GetStatic { class, name, desc } => {
                let idx = jc.add_field_ref(class, name, desc);
                [vec![0xB2], idx.to_be_bytes().to_vec()].concat()
            }
            Instruction::PutStatic { class, name, desc } => {
                let idx = jc.add_field_ref(class, name, desc);
                [vec![0xB3], idx.to_be_bytes().to_vec()].concat()
            }
            Instruction::GetField { class, name, desc } => {
                let idx = jc.add_field_ref(class, name, desc);
                [vec![0xB4], idx.to_be_bytes().to_vec()].concat()
            }
            Instruction::PutField { class, name, desc } => {
                let idx = jc.add_field_ref(class, name, desc);
                [vec![0xB5], idx.to_be_bytes().to_vec()].concat()
            }
            Instruction::InvokeVirtual { class, name, desc } => {
                let idx = jc.add_method_ref(class, name, desc);
                [vec![0xB6], idx.to_be_bytes().to_vec()].concat()
            }
            Instruction::InvokeSpecial { class, name, desc } => {
                let idx = jc.add_method_ref(class, name, desc);
                [vec![0xB7], idx.to_be_bytes().to_vec()].concat()
            }
            Instruction::InvokeStatic { class, name, desc } => {
                let idx = jc.add_method_ref(class, name, desc);
                [vec![0xB8], idx.to_be_bytes().to_vec()].concat()
            }
        }
    }

    pub fn cast(from: &PrimitiveType, to: &PrimitiveType) -> Instruction {
        match (from, to) {
            (PrimitiveType::Int, PrimitiveType::Long) => Instruction::I2L,
            (PrimitiveType::Int, PrimitiveType::Float) => Instruction::I2F,
            (PrimitiveType::Int, PrimitiveType::Double) => Instruction::I2D,
            (PrimitiveType::Long, PrimitiveType::Int) => Instruction::L2I,
            (PrimitiveType::Long, PrimitiveType::Float) => Instruction::L2F,
            (PrimitiveType::Long, PrimitiveType::Double) => Instruction::L2D,
            (PrimitiveType::Float, PrimitiveType::Int) => Instruction::F2I,
            (PrimitiveType::Float, PrimitiveType::Long) => Instruction::F2L,
            (PrimitiveType::Float, PrimitiveType::Double) => Instruction::F2D,
            (PrimitiveType::Double, PrimitiveType::Int) => Instruction::D2I,
            (PrimitiveType::Double, PrimitiveType::Long) => Instruction::D2L,
            (PrimitiveType::Double, PrimitiveType::Float) => Instruction::D2F,
            _ => panic!("Unsupported cast from {:?} to {:?}", from.to_descriptor(), to.to_descriptor()),
        }
    }

    /// Returns the compare instruction to use before an IfEq/IfLt/etc. for non-int types.
    /// For < and <= use the "l" variant (NaN → -1, safely falls to false branch).
    /// For > and >= use the "g" variant (NaN → +1, safely falls to false branch).
    pub fn cmp_for_type(t: &PrimitiveType, nan_is_greater: bool) -> Option<Instruction> {
        match t {
            PrimitiveType::Long => Some(Instruction::LCmp),
            PrimitiveType::Float => Some(if nan_is_greater { Instruction::FCmpg } else { Instruction::FCmpl }),
            PrimitiveType::Double => Some(if nan_is_greater { Instruction::DCmpg } else { Instruction::DCmpl }),
            _ => None, // Int/Bool/Char use IfIcmp* directly
        }
    }
}