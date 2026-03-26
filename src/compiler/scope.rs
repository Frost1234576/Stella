use crate::{tokenizer::lexer::{File, Token}};
use std::{hash::Hash, sync::Arc};
use crate::literals::*;
use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;


#[derive(Debug, Clone, PartialEq)]
pub struct Scope {
    // Local mappings: Only visible in this scope
    slots: HashMap<String, u8>,
    variable_types: HashMap<String, PrimitiveType>,
    
    // Shared state: All nested scopes point to the same root bitset
    busy_slots: Rc<RefCell<Vec<bool>>>, 
    
    parent: Option<Box<Scope>>,
}

impl Scope {
    pub fn new(parent: Option<Box<Scope>>) -> Self {
        let busy_slots = match &parent {
            // If there's a parent, clone the pointer to its busy_slots
            Some(p) => Rc::clone(&p.busy_slots),
            // If this is the root, create a brand new bitset
            None => Rc::new(RefCell::new(Vec::new())),
        };

        Self {
            slots: HashMap::new(),
            variable_types: HashMap::new(),
            busy_slots,
            parent,
        }
    }

    pub fn add_var(&mut self, name: String, var_type: PrimitiveType) -> u8 {
        let size = var_type.size() as usize;
        let mut found_idx = None;
        
        // We borrow the shared bitset mutably
        let mut busy = self.busy_slots.borrow_mut();

        // 1. Find a gap in the shared bitset
        for i in 0..=busy.len() {
            let window_end = i + size;
            let is_free = if window_end <= busy.len() {
                busy[i..window_end].iter().all(|&occupied| !occupied)
            } else {
                true 
            };

            if is_free {
                found_idx = Some(i);
                break;
            }
        }

        let slot = found_idx.unwrap() as u8;

        // 2. Resize and mark as occupied in the shared bitset
        if (slot as usize + size) > busy.len() {
            busy.resize(slot as usize + size, false);
        }
        for i in 0..size {
            busy[slot as usize + i] = true;
        }

        // 3. Add to LOCAL hashmaps (the name is only visible here)
        self.slots.insert(name.clone(), slot);
        self.variable_types.insert(name, var_type);
        
        slot
    }

    pub fn free_var(&mut self, name: &str) {
        // Remove from local maps
        if let (Some(slot), Some(ty)) = (self.slots.remove(name), self.variable_types.remove(name)) {
            let mut busy = self.busy_slots.borrow_mut();
            // Clear the bits in the shared bitset
            for i in 0..ty.size() {
                busy[(slot as usize) + i as usize] = false;
            }
        }
    }

    pub fn get_var(&self, name: &str) -> Option<u8> {
        if let Some(slot) = self.slots.get(name) {
            Some(*slot)
        } else if let Some(parent) = &self.parent {
            parent.get_var(name)
        } else {
            None
        }
    }

    pub fn get_type(&self, name: &str) -> Option<&PrimitiveType> {
        if let Some(ty) = self.variable_types.get(name) {
            Some(ty)
        } else if let Some(parent) = &self.parent {
            parent.get_type(name)
        } else {
            None
        }
    }

    pub fn get_max_locals(&self) -> u8 {
        self.busy_slots.borrow().len() as u8
    }

	pub fn dropall(&self) {
        // borrow busy_slots cell mutably
        let mut busy = self.busy_slots.borrow_mut();
        
		// hashmaps cant be cleared because they arent in refcells

        for (name, &slot) in &self.slots {
            if let Some(ty) = self.variable_types.get(name) {
                for i in 0..ty.size() {
                    busy[(slot as usize) + i as usize] = false;
                }
            }
        }
    }
}

