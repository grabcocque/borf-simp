// src/repl/interpreter/env.rs
// This module provides the environment implementation for the Borf interpreter

use std::collections::HashMap;
use crate::repl::interpreter::types::Value;

// Re-export the Env struct
pub use crate::repl::interpreter::types::Env;

impl Env {
    pub fn new() -> Self {
        Env {
            bindings: HashMap::new(),
            parent: None,
        }
    }

    pub fn with_parent(parent: &Env) -> Self {
        Env {
            bindings: HashMap::new(),
            parent: Some(Box::new(parent.clone())),
        }
    }

    pub fn get(&self, name: &str) -> Option<Value> {
        if let Some(value) = self.bindings.get(name) {
            Some(value.clone())
        } else if let Some(parent) = &self.parent {
            parent.get(name)
        } else {
            None
        }
    }

    pub fn set(&mut self, name: &str, value: Value) {
        self.bindings.insert(name.to_string(), value);
    }
}