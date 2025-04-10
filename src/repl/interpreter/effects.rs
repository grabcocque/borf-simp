// src/repl/interpreter/effects.rs
// This module provides the implementation of the linear effect system for Borf

use std::collections::{HashMap, HashSet};
use std::fmt;
use crate::repl::interpreter::types::{EvaluatorError, Result, Value};

// Represent different types of effects
#[derive(Debug, Clone, PartialEq)]
pub enum EffectType {
    Creates(String),      // Creates a resource of the given type
    Consumes(String),     // Consumes a resource of the given type
    Uses(String),         // Uses a resource of the given type
    Transfers(String),    // Transfers ownership of a resource
    Pure,                 // No effects
}

impl fmt::Display for EffectType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EffectType::Creates(resource) => write!(f, "!creates[{}]", resource),
            EffectType::Consumes(resource) => write!(f, "!consumes[{}]", resource),
            EffectType::Uses(resource) => write!(f, "!uses[{}]", resource),
            EffectType::Transfers(resource) => write!(f, "!transfers[{}]", resource),
            EffectType::Pure => write!(f, "!pure"),
        }
    }
}

// Define a resource that can be tracked
#[derive(Debug, Clone)]
pub struct Resource {
    id: usize,            // Unique identifier for the resource
    resource_type: String, // Type of the resource (e.g., "file", "socket")
    consumed: bool,       // Whether the resource has been consumed
}

impl Resource {
    pub fn new(id: usize, resource_type: &str) -> Self {
        Resource {
            id,
            resource_type: resource_type.to_string(),
            consumed: false,
        }
    }
    
    pub fn mark_consumed(&mut self) -> Result<()> {
        if self.consumed {
            return Err(EvaluatorError::EvalError(
                format!("Resource {} (type {}) already consumed", self.id, self.resource_type)
            ));
        }
        self.consumed = true;
        Ok(())
    }
    
    pub fn is_consumed(&self) -> bool {
        self.consumed
    }
    
    pub fn resource_type(&self) -> &str {
        &self.resource_type
    }
}

// Resource manager to track resources
#[derive(Debug, Clone)]
pub struct ResourceManager {
    resources: HashMap<usize, Resource>, // Map from resource ID to Resource
    next_id: usize,                     // Next resource ID to assign
    current_regions: Vec<HashSet<usize>>, // Stack of regions for borrowed resources
}

impl ResourceManager {
    pub fn new() -> Self {
        ResourceManager {
            resources: HashMap::new(),
            next_id: 0,
            current_regions: Vec::new(),
        }
    }
    
    // Create a new resource and return its ID
    pub fn create_resource(&mut self, resource_type: &str) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        
        self.resources.insert(id, Resource::new(id, resource_type));
        id
    }
    
    // Mark a resource as consumed
    pub fn consume_resource(&mut self, id: usize) -> Result<()> {
        if let Some(resource) = self.resources.get_mut(&id) {
            // Check if the resource is borrowed in the current region
            if self.is_borrowed(id) {
                return Err(EvaluatorError::EvalError(
                    format!("Cannot consume borrowed resource {} (type {})", 
                            id, resource.resource_type())
                ));
            }
            
            resource.mark_consumed()?;
            Ok(())
        } else {
            Err(EvaluatorError::EvalError(format!("Resource with ID {} not found", id)))
        }
    }
    
    // Check if a resource exists and is not consumed
    pub fn check_resource(&self, id: usize) -> Result<()> {
        if let Some(resource) = self.resources.get(&id) {
            if resource.is_consumed() {
                Err(EvaluatorError::EvalError(
                    format!("Resource {} (type {}) has been consumed", 
                            id, resource.resource_type())
                ))
            } else {
                Ok(())
            }
        } else {
            Err(EvaluatorError::EvalError(format!("Resource with ID {} not found", id)))
        }
    }
    
    // Get the type of a resource
    pub fn resource_type(&self, id: usize) -> Result<String> {
        if let Some(resource) = self.resources.get(&id) {
            Ok(resource.resource_type().to_string())
        } else {
            Err(EvaluatorError::EvalError(format!("Resource with ID {} not found", id)))
        }
    }
    
    // Start a new borrowing region
    pub fn start_region(&mut self) {
        self.current_regions.push(HashSet::new());
    }
    
    // End the current borrowing region
    pub fn end_region(&mut self) -> Result<()> {
        if let Some(_) = self.current_regions.pop() {
            Ok(())
        } else {
            Err(EvaluatorError::EvalError("No active borrowing region".to_string()))
        }
    }
    
    // Borrow a resource in the current region
    pub fn borrow_resource(&mut self, id: usize) -> Result<()> {
        // Check if the resource exists and is not consumed
        self.check_resource(id)?;
        
        // Add to the current region
        if let Some(region) = self.current_regions.last_mut() {
            region.insert(id);
            Ok(())
        } else {
            Err(EvaluatorError::EvalError("No active borrowing region".to_string()))
        }
    }
    
    // Check if a resource is borrowed in any active region
    pub fn is_borrowed(&self, id: usize) -> bool {
        for region in &self.current_regions {
            if region.contains(&id) {
                return true;
            }
        }
        false
    }
    
    // Check for resource leaks at the end of evaluation
    pub fn check_for_leaks(&self) -> Result<()> {
        let mut leaked = Vec::new();
        
        for (id, resource) in &self.resources {
            if !resource.is_consumed() {
                leaked.push(format!("{} (type {})", id, resource.resource_type()));
            }
        }
        
        if !leaked.is_empty() {
            Err(EvaluatorError::EvalError(
                format!("Resource leak detected: {} resources not consumed: {}", 
                        leaked.len(), leaked.join(", "))
            ))
        } else {
            Ok(())
        }
    }
    
    // Get statistics about current resources
    pub fn stats(&self) -> String {
        let total = self.resources.len();
        let consumed = self.resources.values().filter(|r| r.is_consumed()).count();
        let active = total - consumed;
        
        let mut by_type = HashMap::new();
        for resource in self.resources.values() {
            *by_type.entry(resource.resource_type().to_string())
                   .or_insert(0) += 1;
        }
        
        let mut result = format!("Resources: {} total, {} active, {} consumed\n", 
                               total, active, consumed);
        
        result.push_str("By type:\n");
        for (type_name, count) in by_type {
            result.push_str(&format!("  {}: {}\n", type_name, count));
        }
        
        result
    }
}

// Parse an effect annotation from a string
pub fn parse_effect(effect_str: &str) -> Result<EffectType> {
    if effect_str.starts_with("!creates[") && effect_str.ends_with("]") {
        let resource = &effect_str[9..effect_str.len() - 1];
        Ok(EffectType::Creates(resource.to_string()))
    } else if effect_str.starts_with("!consumes[") && effect_str.ends_with("]") {
        let resource = &effect_str[10..effect_str.len() - 1];
        Ok(EffectType::Consumes(resource.to_string()))
    } else if effect_str.starts_with("!uses[") && effect_str.ends_with("]") {
        let resource = &effect_str[6..effect_str.len() - 1];
        Ok(EffectType::Uses(resource.to_string()))
    } else if effect_str.starts_with("!transfers[") && effect_str.ends_with("]") {
        let resource = &effect_str[11..effect_str.len() - 1];
        Ok(EffectType::Transfers(resource.to_string()))
    } else if effect_str == "!pure" {
        Ok(EffectType::Pure)
    } else {
        Err(EvaluatorError::ParseError(format!("Invalid effect annotation: {}", effect_str)))
    }
}

// Extension to Value for resource handling
pub trait ResourceValue {
    fn get_resource_id(&self) -> Option<usize>;
    fn with_resource_id(self, id: usize) -> Self;
    fn is_resource(&self) -> bool;
}

impl ResourceValue for Value {
    fn get_resource_id(&self) -> Option<usize> {
        match self {
            Value::Resource(id, _) => Some(*id),
            _ => None,
        }
    }
    
    fn with_resource_id(self, id: usize) -> Self {
        match self {
            Value::Resource(_, value) => Value::Resource(id, value),
            _ => Value::Resource(id, Box::new(self)),
        }
    }
    
    fn is_resource(&self) -> bool {
        matches!(self, Value::Resource(_, _))
    }
}

// Functions for working with resources in the evaluator
pub fn tag_as_resource(value: Value, resource_type: &str, manager: &mut ResourceManager) -> Value {
    let id = manager.create_resource(resource_type);
    value.with_resource_id(id)
}

pub fn use_resource(value: &Value, manager: &ResourceManager) -> Result<()> {
    if let Some(id) = value.get_resource_id() {
        manager.check_resource(id)?;
        Ok(())
    } else {
        Err(EvaluatorError::EvalError("Expected a resource value".to_string()))
    }
}

pub fn consume_resource(value: &Value, manager: &mut ResourceManager) -> Result<Value> {
    if let Some(id) = value.get_resource_id() {
        manager.consume_resource(id)?;
        
        // Return the inner value
        if let Value::Resource(_, inner) = value {
            Ok(*inner.clone())
        } else {
            // This shouldn't happen due to the check above
            Ok(Value::Nil)
        }
    } else {
        Err(EvaluatorError::EvalError("Expected a resource value".to_string()))
    }
}

pub fn borrow_resource(value: &Value, manager: &mut ResourceManager) -> Result<Value> {
    if let Some(id) = value.get_resource_id() {
        manager.borrow_resource(id)?;
        
        // Return a reference to the resource
        if let Value::Resource(_, inner) = value {
            Ok(Value::BorrowedResource(id, inner.clone()))
        } else {
            // This shouldn't happen due to the check above
            Ok(Value::Nil)
        }
    } else {
        Err(EvaluatorError::EvalError("Expected a resource value".to_string()))
    }
}