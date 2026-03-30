use std::collections::HashMap;

use crate::{ast::MethodSignature, literals::PrimitiveType};

/*
GlobalRegistry: All public classes, methods, fields, etc.
LocalRegistry: File Imports, private classes, methods, fields, etc. (only visible within the file)

*/

struct GlobalRegistry {
	classes: HashMap<String, ClassInfo>,
}

impl GlobalRegistry {
	pub fn new() -> Self {
		Self {
			classes: HashMap::new(),
		}
	}
}

#[derive(Clone)]
struct ClassInfo {
	name: String,
	fields: HashMap<String, PrimitiveType>,
	methods: HashMap<String, MethodSignature>,
}

struct LocalRegistry {
	classes: HashMap<String, ClassInfo>,
}

impl LocalRegistry {
	pub fn new() -> Self {
		Self {
			classes: HashMap::new(),
		}
	}
}


// pub fn register_class(global: &mut GlobalRegistry, local: &mut LocalRegistry, class_info: ClassInfo) {
// 	// Register in global registry
// 	global.classes.insert(class_info.name.clone(), class_info.clone());
	
// 	// Register in local registry
// 	local.classes.insert(class_info.name.clone(), class_info);
// }


