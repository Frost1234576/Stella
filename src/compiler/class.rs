use crate::compiler::instructions::{Instruction};
use crate::Method;

use std::fs::File;
use std::io::{Write, Result, Cursor};

pub mod AccessFlags {
    // Class-specific flags
    pub const ACC_PUBLIC: u16 = 0x0001;
    pub const ACC_FINAL: u16 = 0x0010;
    pub const ACC_SUPER: u16 = 0x0020;
    pub const ACC_INTERFACE: u16 = 0x0200;
    pub const ACC_ABSTRACT: u16 = 0x0400;
    pub const ACC_SYNTHETIC: u16 = 0x1000;
    pub const ACC_ANNOTATION: u16 = 0x2000;
    pub const ACC_ENUM: u16 = 0x4000;

    // Method-specific flags
    pub const ACC_PRIVATE: u16 = 0x0002;
    pub const ACC_PROTECTED: u16 = 0x0004;
    pub const ACC_STATIC: u16 = 0x0008;
    pub const ACC_SYNCHRONIZED: u16 = 0x0020;
    pub const ACC_BRIDGE: u16 = 0x0040;
    pub const ACC_VARARGS: u16 = 0x0080;
    pub const ACC_NATIVE: u16 = 0x0100;
    pub const ACC_STRICT: u16 = 0x0800;

    // Field-specific flags
    pub const ACC_VOLATILE: u16 = 0x0040;
    pub const ACC_TRANSIENT: u16 = 0x0080;
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq)]
enum CpTag {
    Utf8 = 1,
    Integer = 3,
    Float = 4,
    Long = 5,    // Occupies 2 slots
    Double = 6,  // Occupies 2 slots
    Class = 7,
    String = 8,
    Fieldref = 9,
    Methodref = 10,
    NameAndType = 12,
}

/// Renamed from `Method` to avoid collision with `method::Method`.
/// This holds the raw, compiled bytecode indices needed for the .class file.
#[derive(Debug)]
struct CompiledMethod {
    name_idx: u16,
    desc_idx: u16,
    access_flags: u16,
    bytecode: Vec<u8>,
    max_stack: u16,
    max_locals: u16,
}

pub struct JavaClass {
    major_version: u16,
    minor_version: u16,
    constant_pool: Vec<Vec<u8>>,
    cp_logical_size: u16, 
    this_class_idx: u16,
    super_class_idx: u16,
    methods: Vec<CompiledMethod>,
}

impl JavaClass {
    pub fn new(class_name: &str, super_class: Option<&str>) -> Self {
        let mut jc = Self {
            major_version: 49, // Java 5 so that we don't need to use stackmapframes for now
            minor_version: 0,
            constant_pool: Vec::new(),
            cp_logical_size: 1, 
            this_class_idx: 0,
            super_class_idx: 0,
            methods: Vec::new(),
        };

        let this_utf = jc.add_utf8(class_name);
        jc.this_class_idx = jc.add_class(this_utf);

        let super_utf = if let Some(super_name) = super_class {
            jc.add_utf8(super_name)
        } else {
            jc.add_utf8("java/lang/Object")
        };
        jc.super_class_idx = jc.add_class(super_utf);

        jc.add_utf8("Code");

        jc
    }

    fn add_to_cp(&mut self, tag: CpTag, mut data: Vec<u8>) -> u16 {
        let index = self.cp_logical_size;
        let mut entry = vec![tag as u8];
        entry.append(&mut data);
        self.constant_pool.push(entry);
        
        if tag == CpTag::Long || tag == CpTag::Double {
            self.cp_logical_size += 2;
        } else {
            self.cp_logical_size += 1;
        }
        
        index
    }

    pub fn add_utf8(&mut self, s: &str) -> u16 {
        let bytes = s.as_bytes();
        let mut data = (bytes.len() as u16).to_be_bytes().to_vec();
        data.extend(bytes);
        self.add_to_cp(CpTag::Utf8, data)
    }

    pub fn add_float_constant(&mut self, val: f32) -> u16 {
        self.add_to_cp(CpTag::Float, val.to_bits().to_be_bytes().to_vec())
    }

    pub fn add_integer_constant(&mut self, val: i32) -> u16 {
        self.add_to_cp(CpTag::Integer, val.to_be_bytes().to_vec())
    }

    pub fn add_long_constant(&mut self, val: i64) -> u16 {
        self.add_to_cp(CpTag::Long, val.to_be_bytes().to_vec())
    }

    pub fn add_double_constant(&mut self, val: f64) -> u16 {
        self.add_to_cp(CpTag::Double, val.to_bits().to_be_bytes().to_vec())
    }

    pub fn add_class(&mut self, name_idx: u16) -> u16 {
        self.add_to_cp(CpTag::Class, name_idx.to_be_bytes().to_vec())
    }

    pub fn add_string_constant(&mut self, text: &str) -> u16 {
        let utf = self.add_utf8(text);
        self.add_to_cp(CpTag::String, utf.to_be_bytes().to_vec())
    }

    pub fn add_method_ref(&mut self, class: &str, name: &str, desc: &str) -> u16 {
        let c_utf = self.add_utf8(class);
        let c_idx = self.add_class(c_utf);
        let n_utf = self.add_utf8(name);
        let d_utf = self.add_utf8(desc);
        let nat = self.add_to_cp(CpTag::NameAndType, [n_utf.to_be_bytes(), d_utf.to_be_bytes()].concat());
        self.add_to_cp(CpTag::Methodref, [c_idx.to_be_bytes(), nat.to_be_bytes()].concat())
    }

    pub fn add_field_ref(&mut self, class: &str, name: &str, desc: &str) -> u16 {
        let c_utf = self.add_utf8(class);
        let c_idx = self.add_class(c_utf);
        let n_utf = self.add_utf8(name);
        let d_utf = self.add_utf8(desc);
        let nat = self.add_to_cp(CpTag::NameAndType, [n_utf.to_be_bytes(), d_utf.to_be_bytes()].concat());
        self.add_to_cp(CpTag::Fieldref, [c_idx.to_be_bytes(), nat.to_be_bytes()].concat())
    }

    /// Takes the high-level Method struct from method.rs, assembles the instructions
    /// into bytecodes, and stores the compiled version.
    pub fn add_method(&mut self, method: Method) {
        let name_idx = self.add_utf8(&method.name);
        let desc_idx = self.add_utf8(&method.descriptor);
        
        // Assemble the jump-aware instruction stream
        let bytecode = Instruction::assemble(&method.code, self);
        
        self.methods.push(CompiledMethod {
            name_idx,
            desc_idx,
            access_flags: method.access_flags,
            bytecode,
            max_stack: method.max_stack,
            max_locals: method.max_locals,
        });
        println!("{:?}", &self.methods[self.methods.len()-1]);
    }




    pub fn generate_bytes(&self) -> Vec<u8> {
        let mut buffer = Vec::new();

        // Magic number and versions
        buffer.write_all(&0xCAFEBABE_u32.to_be_bytes()).unwrap();
        buffer.write_all(&self.minor_version.to_be_bytes()).unwrap();
        buffer.write_all(&self.major_version.to_be_bytes()).unwrap();

        // Constant Pool
        buffer.write_all(&self.cp_logical_size.to_be_bytes()).unwrap();
        for entry in &self.constant_pool {
            buffer.write_all(entry).unwrap();
        }

        // Class access and indices
        let class_flags = AccessFlags::ACC_PUBLIC | AccessFlags::ACC_SUPER;
        buffer.write_all(&class_flags.to_be_bytes()).unwrap();
        buffer.write_all(&self.this_class_idx.to_be_bytes()).unwrap();
        buffer.write_all(&self.super_class_idx.to_be_bytes()).unwrap();

        // Interfaces (0) and Fields (0)
        buffer.write_all(&0_u16.to_be_bytes()).unwrap(); 
        buffer.write_all(&0_u16.to_be_bytes()).unwrap(); 

        // Methods
        buffer.write_all(&(self.methods.len() as u16).to_be_bytes()).unwrap();
        let code_attr_utf = self.get_utf8_index("Code").expect("Code attribute missing");
        
        for m in &self.methods {
            buffer.write_all(&m.access_flags.to_be_bytes()).unwrap();
            buffer.write_all(&m.name_idx.to_be_bytes()).unwrap();
            buffer.write_all(&m.desc_idx.to_be_bytes()).unwrap();
            buffer.write_all(&1_u16.to_be_bytes()).unwrap(); // Attributes count (Code)

            // Code Attribute
            buffer.write_all(&code_attr_utf.to_be_bytes()).unwrap();
            let attr_len = (8 + m.bytecode.len() + 2 + 2) as u32;
            buffer.write_all(&attr_len.to_be_bytes()).unwrap();
            buffer.write_all(&m.max_stack.to_be_bytes()).unwrap();
            buffer.write_all(&m.max_locals.to_be_bytes()).unwrap();
            buffer.write_all(&(m.bytecode.len() as u32).to_be_bytes()).unwrap();
            buffer.write_all(&m.bytecode).unwrap();
            buffer.write_all(&0_u16.to_be_bytes()).unwrap(); // Exception table length
            buffer.write_all(&0_u16.to_be_bytes()).unwrap(); // Attributes count for Code
        }

        // Class Attributes (0)
        buffer.write_all(&0_u16.to_be_bytes()).unwrap(); 
        
        buffer
    }

    /// Wrapper that uses generate_bytes to write to a file
    pub fn write_file(&self, filename: &str) -> Result<()> {
        let bytes = self.generate_bytes();
        let mut f = File::create(filename)?;
        f.write_all(&bytes)?;
        Ok(())
    }

    fn get_utf8_index(&self, s: &str) -> Option<u16> {
        let mut current_idx = 1;
        for entry in &self.constant_pool {
            let tag = entry[0];
            if tag == CpTag::Utf8 as u8 {
                let len = u16::from_be_bytes([entry[1], entry[2]]) as usize;
                if &entry[3..3 + len] == s.as_bytes() {
                    return Some(current_idx);
                }
            }
            if tag == CpTag::Long as u8 || tag == CpTag::Double as u8 {
                current_idx += 2;
            } else {
                current_idx += 1;
            }
        }
        None
    }
}


use zip::write::FileOptions;
use zip::ZipWriter;

pub fn create_jar(jar_path: &str, main_class: &str, class_files: Vec<(&str, Vec<u8>)>) -> std::io::Result<()> {
    let file = File::create(jar_path)?;
    let mut zip = ZipWriter::new(file);

    let options = FileOptions::default()
        .compression_method(zip::CompressionMethod::Stored); // JARs often use 'Stored' for speed

    // 1. Create META-INF/MANIFEST.MF
    zip.add_directory("META-INF/", options)?;
    zip.start_file("META-INF/MANIFEST.MF", options)?;
    let manifest = format!("Manifest-Version: 1.0\nMain-Class: {}\n\n", main_class);
    zip.write_all(manifest.as_bytes())?;

    // 2. Add your .class files
    for (name, bytes) in class_files {
        zip.start_file(name, options)?;
        zip.write_all(&bytes)?;
    }

    zip.finish()?;
    Ok(())
}