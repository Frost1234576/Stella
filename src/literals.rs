#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PrimitiveType {
    Bool,
    Int,
    Double,
    Float,
    Long,
    Char,
    String,
    Nil,
    Reference(String),
    Array(Box<PrimitiveType>), // Added Array (Boxed for recursion)
}

impl PrimitiveType {
    pub fn to_descriptor(&self) -> String {
        match self {
            PrimitiveType::Bool => "Z".to_string(),
            PrimitiveType::Int => "I".to_string(),
            PrimitiveType::Double => "D".to_string(),
            PrimitiveType::Float => "F".to_string(),
            PrimitiveType::Long => "J".to_string(),
            PrimitiveType::Char => "C".to_string(),
            PrimitiveType::String => "Ljava/lang/String;".to_string(),
            PrimitiveType::Nil => "V".to_string(),
            PrimitiveType::Reference(class_name) => format!("L{};", class_name.replace('.', "/")),
            // Arrays in JVM start with [ followed by the inner type's descriptor
            PrimitiveType::Array(inner_type) => format!("[{}", inner_type.to_descriptor()), 
        }
    }

    pub fn from_string(s: &str) -> PrimitiveType {
        match s {
            "bool" => PrimitiveType::Bool,
            "int" => PrimitiveType::Int,
            "double" => PrimitiveType::Double,
            "float" => PrimitiveType::Float,
            "long" => PrimitiveType::Long,
            "char" => PrimitiveType::Char,
            "string" => PrimitiveType::String,
            "nil" => PrimitiveType::Nil,
            "null" => PrimitiveType::Nil,
            "void" => PrimitiveType::Nil,
            _ if s.ends_with("[]") => {
                let inner = &s[..s.len() - 2];
                PrimitiveType::Array(Box::new(PrimitiveType::from_string(inner)))
            },
            _ => PrimitiveType::Reference(s.to_string()),
        }
    }

    // precedence for type promotion in binary operations
    pub fn precedence(&self) -> u8 {
        match self {
            PrimitiveType::Bool => 1,
            PrimitiveType::Int => 2,
            PrimitiveType::Float => 3,
            PrimitiveType::Long => 4,
            PrimitiveType::Double => 5,
            PrimitiveType::Char => 2, // char is treated as int in operations
            PrimitiveType::String => 0, // string concatenation is handled separately
            PrimitiveType::Nil => 0, // nil has no precedence
            PrimitiveType::Reference(_) => 0, // reference types are not promoted
            PrimitiveType::Array(_) => 0, // arrays are not mathematically promoted
        }
    }

    pub fn is_numeric(&self) -> bool {
        matches!(self, PrimitiveType::Int | PrimitiveType::Float | PrimitiveType::Long | PrimitiveType::Double | PrimitiveType::Char)
    }

    pub fn compare_precedence(&self, other: &PrimitiveType) -> PrimitiveType {
        if self.is_numeric() && other.is_numeric() {
            if self.precedence() >= other.precedence() {
                self.clone()
            } else {
                other.clone()
            }
        } else {
            // for non-numeric types, we can define rules as needed
            // for now, we will just return self for simplicity
            self.clone()
        }
    }

    pub fn size(&self) -> u8 {
        match self {
            PrimitiveType::Bool | PrimitiveType::Int | PrimitiveType::Float | PrimitiveType::Char => 1,
            PrimitiveType::Long | PrimitiveType::Double => 2,
            // Arrays are object references on the JVM, so they take 1 slot
            PrimitiveType::String | PrimitiveType::Nil | PrimitiveType::Reference(_) | PrimitiveType::Array(_) => 1, 
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    Bool(bool),
    String(String),
    Char(char),
    Nil,
    Array(Vec<Literal>), // Added Array Literal
}

impl Literal {
    pub fn get_type(&self) -> PrimitiveType {
        match self {
            Literal::Int(_) => PrimitiveType::Int,
            Literal::Long(_) => PrimitiveType::Long,
            Literal::Float(_) => PrimitiveType::Float,
            Literal::Double(_) => PrimitiveType::Double,
            Literal::Bool(_) => PrimitiveType::Bool,
            Literal::String(_) => PrimitiveType::String,
            Literal::Char(_) => PrimitiveType::Char,
            Literal::Nil => PrimitiveType::Nil,
            Literal::Array(elements) => {
                // Peek at the first element to determine the array's type.
                // If it's empty, we default to Nil (or you could default to Object).
                let inner_type = elements.first()
                    .map(|e| e.get_type())
                    .unwrap_or(PrimitiveType::Nil);
                
                PrimitiveType::Array(Box::new(inner_type))
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Generic {
    pub constraints: Vec<PrimitiveType>, // should only be references (i think?)
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Parameter {
    Named{
        name: String,
        param_type: PrimitiveType,
        generic: Option<Generic>,
    },
    Unnamed{
        param_type: PrimitiveType,
        generic: Option<Generic>,
    },
    Signature{
        param_type: PrimitiveType,
    }
}

impl Parameter {
    fn to_signature(&self) -> Parameter {
        match self {
            Parameter::Named { name: _, param_type, generic: _ } => Parameter::Signature { param_type: param_type.clone() },
            Parameter::Unnamed { param_type, generic: _ } => Parameter::Signature { param_type: param_type.clone() },
            Parameter::Signature { param_type } => Parameter::Signature { param_type: param_type.clone() },
        }
    }

    pub fn get_type(&self) -> PrimitiveType {
        match self {
            Parameter::Named { name: _, param_type, generic: _ } => param_type.clone(),
            Parameter::Unnamed { param_type, generic: _ } => param_type.clone(),
            Parameter::Signature { param_type } => param_type.clone(),
        }
    }
}


#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GenericType{
	pub base: PrimitiveType,
	pub generic: Option<Generic>,
}