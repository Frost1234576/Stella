use crate::binaries::jar::{self, ClassInfo};
use std::collections::HashMap;
use std::io;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

// 1. Ensure ClassInfo derives serde::Serialize and Deserialize in its original file!

const MODULES: &[&str] = &["java.base"];
const CACHE_FILE_NAME: &str = "stdlib_cache.bin";

pub fn read_stdlib() -> io::Result<&'static HashMap<String, ClassInfo>> {
    static CACHE: OnceLock<HashMap<String, ClassInfo>> = OnceLock::new();
    if let Some(map) = CACHE.get() {
        return Ok(map);
    }
    let _ = CACHE.set(load_stdlib()?);
    Ok(CACHE.get().unwrap())
}

fn load_stdlib() -> io::Result<HashMap<String, ClassInfo>> {
    let cache_dir = std::env::temp_dir().join("jvm_compiler_stdlib_cache");
    let serde_cache_path = cache_dir.join(CACHE_FILE_NAME);

    // --- Step 1: Try to load from Serde cache ---
    if serde_cache_path.exists() {
        eprintln!("jimage: loading from serde cache {} ...", serde_cache_path.display());
        let file = File::open(&serde_cache_path)?;
        // Use bincode for performance; replace with serde_json if you want human-readable
        match bincode::deserialize_from(file) {
            Ok(map) => return Ok(map),
            Err(e) => {
                eprintln!("warn: failed to deserialize cache: {}. Re-extracting...", e);
                // Fall through to re-extraction logic
            }
        }
    }

    // --- Step 2: Extract and Parse (The Slow Way) ---
    if !cache_dir.exists() {
        std::fs::create_dir_all(&cache_dir)?;
    }

    eprintln!("jimage: extracting modules and parsing classes...");
    let mut classes = HashMap::new();
    
    // Perform the jimage extract
    extract_with_jimage(&cache_dir)?;

    for module in MODULES {
        let module_dir = cache_dir.join(module);
        if module_dir.exists() {
            read_class_dir(&module_dir, &mut classes)?;
        }
    }

    // --- Step 3: Save to Serde cache for next time ---
    eprintln!("jimage: saving results to serde cache...");
    let file = File::create(&serde_cache_path)?;
    if let Err(e) = bincode::serialize_into(file, &classes) {
        eprintln!("warn: could not save serde cache: {}", e);
    }

    Ok(classes)
}

fn extract_with_jimage(cache_dir: &PathBuf) -> io::Result<()> {
    let includes: Vec<String> = MODULES.iter().map(|m| format!("/{m}/**")).collect();
    let mut cmd = Command::new("jimage");
    cmd.arg("extract").arg("--dir").arg(cache_dir);
    for pat in &includes {
        cmd.arg("--include").arg(pat);
    }
    
    let output = cmd.output()?;
    if !output.status.success() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("jimage extract failed: {}", String::from_utf8_lossy(&output.stderr).trim()),
        ));
    }
    Ok(())
}


fn read_class_dir(dir: &Path, out: &mut HashMap<String, ClassInfo>) -> io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            read_class_dir(&path, out)?;
        } else if path.extension().map_or(false, |e| e == "class") {
            match std::fs::read(&path).and_then(|b| jar::parse_class(&b)) {
                Ok(info) => { out.insert(info.name.clone(), info); }
                Err(err) => eprintln!("warn: skipping {} — {}", path.display(), err),
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_string_from_stdlib() {
        let Ok(classes) = read_stdlib() else { return };
        let s = classes.get("java/lang/String").expect("java/lang/String missing");
        assert!(s.method_descriptors("substring").contains(&"(I)Ljava/lang/String;"));
        assert!(s.method_descriptors("substring").contains(&"(II)Ljava/lang/String;"));
        assert!(s.method_descriptors("length").contains(&"()I"));
    }

    #[test]
    fn loads_integer_from_stdlib() {
        let Ok(classes) = read_stdlib() else { return };
        let int = classes.get("java/lang/Integer").expect("java/lang/Integer missing");
        assert!(int.method_descriptors("parseInt").contains(&"(Ljava/lang/String;)I"));
        assert_eq!(int.field_descriptor("MAX_VALUE"), Some("I"));
		println!("{}", int.method_descriptors("parseInt").join(", "));
    }
}