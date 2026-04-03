/// jar.rs — Reads .class files and nested JARs up to Java 25 (major 69).
use std::collections::HashMap;
use std::io::{self, Cursor, Read};
use serde::{Serialize, Deserialize};

// ─────────────────────────────────────────────
// Public types
// ─────────────────────────────────────────────

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ClassInfo {
    pub name: String,
    pub super_name: Option<String>,
    pub interfaces: Vec<String>,
    pub access_flags: u16,
    pub methods: Vec<MemberInfo>,
    pub fields: Vec<MemberInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberInfo {
    pub access_flags: u16,
    pub name: String,
    pub descriptor: String,
}

impl ClassInfo {
    pub fn method_descriptors(&self, name: &str) -> Vec<&str> {
        self.methods.iter()
            .filter(|m| m.name == name)
            .map(|m| m.descriptor.as_str())
            .collect()
    }

    pub fn field_descriptor(&self, name: &str) -> Option<&str> {
        self.fields.iter()
            .find(|f| f.name == name)
            .map(|f| f.descriptor.as_str())
    }
}

// ─────────────────────────────────────────────
// JAR reader (Recursive for Bundlers)
// ─────────────────────────────────────────────

/// Reads every `.class` and nested `.jar` entry.
pub fn read_jar(jar_bytes: &[u8]) -> io::Result<HashMap<String, ClassInfo>> {
    let mut classes = HashMap::new();
    read_jar_recursive(jar_bytes, &mut classes)?;
    Ok(classes)
}

fn read_jar_recursive(jar_bytes: &[u8], out: &mut HashMap<String, ClassInfo>) -> io::Result<()> {
    let mut archive = zip::ZipArchive::new(Cursor::new(jar_bytes))
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        let name = entry.name().to_string();
        
        // 1. If it's a nested JAR, recurse into it
        if name.ends_with(".jar") {
            let mut inner_bytes = Vec::with_capacity(entry.size() as usize);
            entry.read_to_end(&mut inner_bytes)?;
            // Ignore errors in nested jars to keep going
            let _ = read_jar_recursive(&inner_bytes, out);
            continue;
        }

        // 2. If it's a class file, parse it
        if name.ends_with(".class") {
            let mut bytes = Vec::with_capacity(entry.size() as usize);
            entry.read_to_end(&mut bytes)?;

            match parse_class(&bytes) {
                Ok(info) => { out.insert(info.name.clone(), info); }
                Err(_) => { /* Skip invalid or metadata classes */ }
            }
        }
    }
    Ok(())
}

pub fn read_jar_file(path: &str) -> io::Result<HashMap<String, ClassInfo>> {
    read_jar(&std::fs::read(path)?)
}

// ─────────────────────────────────────────────
// .class parser (Supports Major 69)
// ─────────────────────────────────────────────

pub fn parse_class(bytes: &[u8]) -> io::Result<ClassInfo> {
    let mut r = Reader::new(bytes);

    if r.u32()? != 0xCAFE_BABE { return Err(e("bad magic")); }
    let _minor = r.u16()?;
    let major  = r.u16()?;

    if major > 69 {
        return Err(e(&format!("major_version {} too high", major)));
    }

    let cp_count = r.u16()? as usize;
    let mut pool = vec![Cp::Null; cp_count];

    let mut i = 1;
    while i < cp_count {
        pool[i] = match r.u8()? {
            1  => { let n = r.u16()? as usize; Cp::Utf8(r.str(n)?) }
            3 | 4 => { r.u32()?; Cp::Null }
            5 | 6 => { r.u64()?; i += 1; Cp::Null }
            7  => Cp::Class(r.u16()?),
            8  => { r.u16()?; Cp::Null }
            9 | 10 | 11 | 12 => { r.u32()?; Cp::Null }
            15 => { r.u8()?; r.u16()?; Cp::Null }
            16 => { r.u16()?; Cp::Null }
            17 | 18 => { r.u32()?; Cp::Null }
            19 | 20 => { r.u16()?; Cp::Null }
            _ => { return Err(e("unknown tag")); }
        };
        i += 1;
    }

    let utf8 = |idx: u16| {
        match pool.get(idx as usize) {
            Some(Cp::Utf8(s)) => Ok(s.clone()),
            _ => Err(e("invalid utf8 index")),
        }
    };

    let class_name = |idx: u16| {
        match pool.get(idx as usize) {
            Some(Cp::Class(n)) => utf8(*n),
            _ => Err(e("invalid class index")),
        }
    };

    let access_flags = r.u16()?;
    let name = class_name(r.u16()?)?;
    let super_idx = r.u16()?;
    let super_name = if super_idx == 0 { None } else { Some(class_name(super_idx)?) };

    let iface_count = r.u16()? as usize;
    let mut interfaces = Vec::with_capacity(iface_count);
    for _ in 0..iface_count { interfaces.push(class_name(r.u16()?)?); }

    let fields = read_members(&mut r, &pool)?;
    let methods = read_members(&mut r, &pool)?;

    Ok(ClassInfo { name, super_name, interfaces, access_flags, methods, fields })
}

fn read_members(r: &mut Reader, pool: &[Cp]) -> io::Result<Vec<MemberInfo>> {
    let count = r.u16()? as usize;
    let mut members = Vec::with_capacity(count);
    for _ in 0..count {
        let access_flags = r.u16()?;
        let name_idx = r.u16()?;
        let desc_idx = r.u16()?;

        let name = match pool.get(name_idx as usize) {
            Some(Cp::Utf8(s)) => s.clone(),
            _ => return Err(e("member name error")),
        };
        let descriptor = match pool.get(desc_idx as usize) {
            Some(Cp::Utf8(s)) => s.clone(),
            _ => return Err(e("member desc error")),
        };

        let attr_count = r.u16()? as usize;
        for _ in 0..attr_count {
            r.u16()?;
            let len = r.u32()? as usize;
            r.skip(len)?;
        }
        members.push(MemberInfo { access_flags, name, descriptor });
    }
    Ok(members)
}

#[derive(Clone)] enum Cp { Null, Utf8(String), Class(u16) }

struct Reader<'a> { data: &'a [u8], pos: usize }
impl<'a> Reader<'a> {
    fn new(data: &'a [u8]) -> Self { Self { data, pos: 0 } }
    fn u8(&mut self) -> io::Result<u8> {
        self.data.get(self.pos).copied().map(|b| { self.pos += 1; b }).ok_or_else(|| e("EOF"))
    }
    fn u16(&mut self) -> io::Result<u16> { Ok(((self.u8()? as u16) << 8) | self.u8()? as u16) }
    fn u32(&mut self) -> io::Result<u32> { Ok(((self.u16()? as u32) << 16) | self.u16()? as u32) }
    fn u64(&mut self) -> io::Result<u64> { Ok(((self.u32()? as u64) << 32) | self.u32()? as u64) }
    fn str(&mut self, n: usize) -> io::Result<String> {
        let end = self.pos + n;
        let slice = self.data.get(self.pos..end).ok_or_else(|| e("EOF"))?;
        self.pos = end;
        Ok(String::from_utf8_lossy(slice).into_owned())
    }
    fn skip(&mut self, n: usize) -> io::Result<()> {
        self.pos += n;
        if self.pos > self.data.len() { return Err(e("EOF")); }
        Ok(())
    }
}

fn e(msg: &str) -> io::Error { io::Error::new(io::ErrorKind::InvalidData, msg) }



// ─────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_string_methods() {
        // Point this at any rt.jar or modern JAR on your system.
        let path = match [
            "/usr/lib/jvm/java-8-openjdk-amd64/jre/lib/rt.jar",
            "/usr/lib/jvm/java-8-openjdk/jre/lib/rt.jar",
        ].iter().find(|p| std::path::Path::new(p).exists()) {
            Some(p) => *p,
            None    => { eprintln!("skipping: no jar found"); return; }
        };

        let classes = read_jar_file(path).unwrap();
        let s = classes.get("java/lang/String").expect("String not found");

        let subs = s.method_descriptors("substring");
        assert!(subs.contains(&"(I)Ljava/lang/String;"));
        assert!(subs.contains(&"(II)Ljava/lang/String;"));
    }

	#[test]
	fn generate_jar_stellab(){
		// let path = "resources/jars/paper-1.21.11-127.jar";
		let path = "resources/jars/server.jar";
		let classes = read_jar_file(path).unwrap();
		println!("Loaded {} classes from {}", classes.len(), path);
		println!("Classes: {:?}", classes.keys().take(100).collect::<Vec<_>>());
		// let s = classes.get("org/bukkit/World").expect("World not found");
		// assert!(s.method_descriptors("getBlockAt").contains(&"(III)Lorg/bukkit/block/Block;"));
	}
}