// src/repl/interpreter/evaluator.rs
// This module provides the evaluator for the Borf interpreter

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use crate::repl::interpreter::types::{Env, EvaluatorError, Expr, Param, Pattern, Result, Type, Value};
use crate::repl::interpreter::parser::Parser;
use crate::repl::interpreter::effects::{ResourceManager, ResourceValue, EffectType, 
                                        tag_as_resource, use_resource, consume_resource, borrow_resource};

// Evaluator with resource tracking
pub struct Evaluator {
    pub env: Env,
    pub stack: Vec<Value>,
    pub prelude_path: PathBuf,
    resource_manager: ResourceManager,
}

impl Evaluator {
    pub fn new() -> Self {
        Evaluator {
            env: Env::new(),
            stack: Vec::new(),
            prelude_path: PathBuf::from("src/prelude"),
            resource_manager: ResourceManager::new(),
        }
    }

    pub fn with_prelude_path<P: AsRef<Path>>(prelude_path: P) -> Self {
        Evaluator {
            env: Env::new(),
            stack: Vec::new(),
            prelude_path: prelude_path.as_ref().to_path_buf(),
            resource_manager: ResourceManager::new(),
        }
    }
    
    // Resource management functions
    
    // Create a new resource
    fn create_resource(&mut self, resource_type: &str, value: Value) -> Value {
        tag_as_resource(value, resource_type, &mut self.resource_manager)
    }
    
    // Use a resource (check if it's valid)
    fn use_resource(&self, value: &Value) -> Result<()> {
        use_resource(value, &self.resource_manager)
    }
    
    // Consume a resource
    fn consume_resource(&mut self, value: &Value) -> Result<Value> {
        consume_resource(value, &mut self.resource_manager)
    }
    
    // Start a borrowing region
    fn start_borrowing_region(&mut self) {
        self.resource_manager.start_region();
    }
    
    // End a borrowing region
    fn end_borrowing_region(&mut self) -> Result<()> {
        self.resource_manager.end_region()
    }
    
    // Borrow a resource
    fn borrow_resource(&mut self, value: &Value) -> Result<Value> {
        borrow_resource(value, &mut self.resource_manager)
    }
    
    // Check for resource leaks
    fn check_for_resource_leaks(&self) -> Result<()> {
        self.resource_manager.check_for_leaks()
    }

    // Set up the built-in functions and values
    pub fn initialize(&mut self) -> Result<()> {
        // Add built-in functions
        self.env.set("print", Value::Symbol("print".to_string()));
        self.env.set("add", Value::Symbol("add".to_string()));
        self.env.set("sub", Value::Symbol("sub".to_string()));
        self.env.set("mul", Value::Symbol("mul".to_string()));
        
        // Add core stack operations
        self.env.set("dup", Value::Symbol("dup".to_string()));
        self.env.set("drop", Value::Symbol("drop".to_string()));
        self.env.set("swap", Value::Symbol("swap".to_string()));
        self.env.set("rot", Value::Symbol("rot".to_string()));
        self.env.set("over", Value::Symbol("over".to_string()));
        self.env.set("tuck", Value::Symbol("tuck".to_string()));
        self.env.set("pick", Value::Symbol("pick".to_string()));
        
        // Add data structures and control operations
        self.env.set("list", Value::Symbol("list".to_string()));
        self.env.set("map", Value::Symbol("map".to_string()));
        self.env.set("if", Value::Symbol("if".to_string()));
        self.env.set("eq", Value::Symbol("eq".to_string()));
        
        // Add metaprogramming operations
        self.env.set("eval", Value::Symbol("eval".to_string()));
        self.env.set("quote", Value::Symbol("quote".to_string()));
        self.env.set("unquote", Value::Symbol("unquote".to_string()));
        self.env.set("quasiquote", Value::Symbol("quasiquote".to_string()));
        
        // Add resource management operations
        self.env.set("create_resource", Value::Symbol("create_resource".to_string()));
        self.env.set("consume_resource", Value::Symbol("consume_resource".to_string()));
        self.env.set("borrow", Value::Symbol("borrow".to_string()));
        self.env.set("is_resource", Value::Symbol("is_resource".to_string()));
        self.env.set("resource_type", Value::Symbol("resource_type".to_string()));
        self.env.set("with_borrowed", Value::Symbol("with_borrowed".to_string()));
        
        // Add stack inspection and debugging
        self.env.set(".s", Value::Symbol(".s".to_string()));
        self.env.set("depth", Value::Symbol("depth".to_string()));
        self.env.set(".resources", Value::Symbol(".resources".to_string()));
        
        Ok(())
    }
    
    // Evaluate a Borf program
    pub fn eval(&mut self, input: &str) -> Result<Value> {
        let mut parser = Parser::new(input);
        match parser.parse() {
            Ok(expr) => self.eval_expr(&expr).map(|opt_val| opt_val.unwrap_or(Value::Nil)),
            Err(e) => Err(e),
        }
    }
    
    // Evaluate a Borf file
    pub fn eval_file<P: AsRef<Path>>(&mut self, file_path: P) -> Result<Value> {
        let content = fs::read_to_string(file_path.as_ref())?;
        self.eval(&content)
    }
    
    // Evaluate an expression with type checking
    fn eval_expr(&mut self, expr: &Expr) -> Result<Option<Value>> {
        match expr {
            Expr::Number(n) => Ok(Some(Value::Number(*n))),
            Expr::String(s) => Ok(Some(Value::String(s.clone()))),
            Expr::Symbol(s) => {
                // Look up symbol in environment
                if let Some(value) = self.env.get(s) {
                    Ok(Some(value))
                } else {
                    // Try to execute as operation
                    self.execute_operation(s)?;
                    Ok(None)
                }
            },
            Expr::Quotation(params, body) => {
                // Create a quotation with the current environment
                Ok(Some(Value::Quotation(
                    params.clone(),
                    body.clone(),
                    Some(Box::new(self.env.clone())),
                )))
            },
            Expr::TypedQuotation(params, body, return_type) => {
                // Create a typed quotation with the current environment
                Ok(Some(Value::TypedQuotation(
                    params.clone(),
                    body.clone(),
                    return_type.as_ref().clone(),
                    Some(Box::new(self.env.clone())),
                )))
            },
            Expr::Pipeline(left, right) => {
                // Evaluate left side
                if let Some(left_value) = self.eval_expr(left)? {
                    // Push left value onto the stack
                    self.stack.push(left_value);
                    
                    // Evaluate right side
                    self.eval_expr(right)
                } else {
                    // If left side produced no value, just evaluate right side
                    self.eval_expr(right)
                }
            },
            Expr::Binary(op, left, right) => {
                // Evaluate both sides
                let left_value = self.eval_expr(left)?
                    .ok_or_else(|| EvaluatorError::EvalError("Left operand produced no value".to_string()))?;
                let right_value = self.eval_expr(right)?
                    .ok_or_else(|| EvaluatorError::EvalError("Right operand produced no value".to_string()))?;
                
                // Infer expected types based on operator
                let expected_type = match op.as_str() {
                    "+" | "-" | "*" | "/" => Type::Simple("Num".to_string()),
                    _ => Type::Simple("Any".to_string()),
                };
                
                // Check operand types (for numeric operations)
                if ["add", "sub", "mul", "div"].contains(&op.as_str()) {
                    self.check_type(&left_value, &expected_type)?;
                    self.check_type(&right_value, &expected_type)?;
                }
                
                // Execute the operation
                match op.as_str() {
                    "+" | "add" => match (&left_value, &right_value) {
                        (Value::Number(a), Value::Number(b)) => Ok(Some(Value::Number(a + b))),
                        _ => Err(EvaluatorError::EvalError(format!("Cannot add non-numeric values")))
                    },
                    "-" | "sub" => match (&left_value, &right_value) {
                        (Value::Number(a), Value::Number(b)) => Ok(Some(Value::Number(a - b))),
                        _ => Err(EvaluatorError::EvalError(format!("Cannot subtract non-numeric values")))
                    },
                    "*" | "mul" => match (&left_value, &right_value) {
                        (Value::Number(a), Value::Number(b)) => Ok(Some(Value::Number(a * b))),
                        _ => Err(EvaluatorError::EvalError(format!("Cannot multiply non-numeric values")))
                    },
                    "/" | "div" => match (&left_value, &right_value) {
                        (Value::Number(a), Value::Number(b)) if *b != 0 => Ok(Some(Value::Number(a / b))),
                        (Value::Number(_), Value::Number(b)) if *b == 0 => 
                            Err(EvaluatorError::EvalError(format!("Division by zero"))),
                        _ => Err(EvaluatorError::EvalError(format!("Cannot divide non-numeric values")))
                    },
                    "==" | "eq" => Ok(Some(Value::Number(if left_value == right_value { 1 } else { 0 }))),
                    "!=" => Ok(Some(Value::Number(if left_value != right_value { 1 } else { 0 }))),
                    _ => Err(EvaluatorError::EvalError(format!("Unknown binary operation: {}", op)))
                }
            },
            Expr::Assignment(value_expr, name) => {
                // Evaluate the expression
                let value = self.eval_expr(value_expr)?
                    .ok_or_else(|| EvaluatorError::EvalError(format!("Cannot assign None to {}", name)))?;
                
                // Bind the value in the environment
                self.env.set(name, value.clone());
                
                // Return the value
                Ok(Some(value))
            },
            Expr::Match(expr, patterns) => {
                // Evaluate the expression to match against
                let value = self.eval_expr(expr)?
                    .ok_or_else(|| EvaluatorError::EvalError("Match expression produced no value".to_string()))?;
                
                // Try each pattern
                for (pattern, result_expr) in patterns {
                    // TODO: Implement proper pattern matching
                    // For now, just check if the pattern is a wildcard or equal to the value
                    match pattern {
                        Pattern::Wildcard => {
                            // Wildcard matches everything
                            return self.eval_expr(result_expr);
                        },
                        Pattern::Literal(lit_expr) => {
                            // Evaluate the literal expression
                            if let Some(lit_value) = self.eval_expr(lit_expr)? {
                                if lit_value == value {
                                    return self.eval_expr(result_expr);
                                }
                            }
                        },
                        Pattern::Variable(name) => {
                            // Bind the value to the variable name in a new scope
                            let mut match_env = Env::with_parent(&self.env);
                            match_env.set(name, value.clone());
                            
                            // Evaluate the result expression in this environment
                            // TODO: Implement this by creating a temporary environment
                            return self.eval_expr(result_expr);
                        },
                        // TODO: Implement other pattern types
                        _ => continue,
                    }
                }
                
                // No pattern matched
                Err(EvaluatorError::EvalError("No pattern matched the value".to_string()))
            },
            Expr::Quote(inner) => {
                // Create a quoted value (doesn't evaluate inner)
                Ok(Some(Value::Quoted(Box::new(
                    self.eval_expr(inner)?.unwrap_or(Value::Nil)
                ))))
            },
            Expr::Unquote(inner) => {
                // Evaluate inner expression to get a quoted value
                let inner_value = self.eval_expr(inner)?
                    .ok_or_else(|| EvaluatorError::EvalError("Unquote expression produced no value".to_string()))?;
                
                // Check if it's a quoted value
                match inner_value {
                    Value::Quoted(quoted) => Ok(Some(*quoted)),
                    _ => Err(EvaluatorError::EvalError("Cannot unquote non-quoted value".to_string())),
                }
            },
            Expr::Quasiquote(inner) => {
                // Would process templates with unquote markers
                // TODO: Implement quasiquotation
                Ok(Some(Value::Quasiquoted(Box::new(
                    self.eval_expr(inner)?.unwrap_or(Value::Nil)
                ))))
            },
            Expr::TypeQuote(typ) => {
                // Create a quoted type
                Ok(Some(Value::QuotedType(typ.as_ref().clone())))
            },
            Expr::TypeUnquote(expr) => {
                // Evaluate expression to get a quoted type
                let value = self.eval_expr(expr)?
                    .ok_or_else(|| EvaluatorError::EvalError("Type unquote expression produced no value".to_string()))?;
                
                // Check if it's a quoted type
                match value {
                    Value::QuotedType(typ) => Ok(Some(Value::Type(typ))),
                    _ => Err(EvaluatorError::TypeError("Cannot unquote non-quoted type".to_string())),
                }
            },
            // TODO: Implement other expression types
            _ => Err(EvaluatorError::EvalError(format!("Unsupported expression type: {:?}", expr))),
        }
    }
    
    // Execute a built-in operation
    fn execute_operation(&mut self, operation: &str) -> Result<()> {
        match operation {
            "print" => {
                // Pop a value from the stack and print it
                if let Some(value) = self.stack.pop() {
                    println!("{}", value);
                }
            },
            "add" => {
                // Pop two values and add them
                if self.stack.len() >= 2 {
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    
                    match (a, b) {
                        (Value::Number(x), Value::Number(y)) => {
                            self.stack.push(Value::Number(x + y));
                        },
                        _ => return Err(EvaluatorError::EvalError("add requires two numbers".to_string())),
                    }
                } else {
                    return Err(EvaluatorError::EvalError("add requires two values on the stack".to_string()));
                }
            },
            "sub" => {
                // Pop two values and subtract them
                if self.stack.len() >= 2 {
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    
                    match (a, b) {
                        (Value::Number(x), Value::Number(y)) => {
                            self.stack.push(Value::Number(x - y));
                        },
                        _ => return Err(EvaluatorError::EvalError("sub requires two numbers".to_string())),
                    }
                } else {
                    return Err(EvaluatorError::EvalError("sub requires two values on the stack".to_string()));
                }
            },
            "mul" => {
                // Pop two values and multiply them
                if self.stack.len() >= 2 {
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    
                    match (a, b) {
                        (Value::Number(x), Value::Number(y)) => {
                            self.stack.push(Value::Number(x * y));
                        },
                        _ => return Err(EvaluatorError::EvalError("mul requires two numbers".to_string())),
                    }
                } else {
                    return Err(EvaluatorError::EvalError("mul requires two values on the stack".to_string()));
                }
            },
            "type" => {
                // Pop a value and get its type
                if let Some(value) = self.stack.pop() {
                    let typ = self.get_value_type(&value)?;
                    self.stack.push(Value::Type(typ));
                } else {
                    return Err(EvaluatorError::EvalError("type requires a value on the stack".to_string()));
                }
            },
            "type_equals" => {
                // Pop two types and check if they're equal
                if self.stack.len() >= 2 {
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    
                    match (&a, &b) {
                        (Value::Type(t1), Value::Type(t2)) => {
                            let result = self.types_compatible(t1, t2) && self.types_compatible(t2, t1);
                            self.stack.push(Value::Number(if result { 1 } else { 0 }));
                        },
                        _ => return Err(EvaluatorError::TypeError("type_equals requires two types".to_string())),
                    }
                } else {
                    return Err(EvaluatorError::EvalError("type_equals requires two values on the stack".to_string()));
                }
            },
            "type_to_string" => {
                // Pop a type and convert it to a string
                if let Some(value) = self.stack.pop() {
                    match value {
                        Value::Type(typ) => {
                            let type_str = self.type_to_string(&typ)?;
                            self.stack.push(Value::String(type_str));
                        },
                        _ => return Err(EvaluatorError::TypeError("type_to_string requires a type".to_string())),
                    }
                } else {
                    return Err(EvaluatorError::EvalError("type_to_string requires a value on the stack".to_string()));
                }
            },
            "type_quote" => {
                // Pop a type and quote it
                if let Some(value) = self.stack.pop() {
                    match value {
                        Value::Type(typ) => {
                            self.stack.push(Value::QuotedType(typ));
                        },
                        _ => return Err(EvaluatorError::TypeError("type_quote requires a type".to_string())),
                    }
                } else {
                    return Err(EvaluatorError::EvalError("type_quote requires a value on the stack".to_string()));
                }
            },
            "type_unquote" => {
                // Pop a quoted type and unquote it
                if let Some(value) = self.stack.pop() {
                    match value {
                        Value::QuotedType(typ) => {
                            self.stack.push(Value::Type(typ));
                        },
                        _ => return Err(EvaluatorError::TypeError("type_unquote requires a quoted type".to_string())),
                    }
                } else {
                    return Err(EvaluatorError::EvalError("type_unquote requires a value on the stack".to_string()));
                }
            },
            "type_quasiquote" => {
                // Pop a quoted type template and process it
                if let Some(value) = self.stack.pop() {
                    match value {
                        Value::QuotedType(t) => {
                            let processed = self.process_type_quasiquote(&t)?;
                            self.stack.push(Value::Type(processed));
                        },
                        _ => return Err(EvaluatorError::TypeError("type_quasiquote requires a quoted type".to_string())),
                    }
                } else {
                    return Err(EvaluatorError::EvalError("type_quasiquote requires a value on the stack".to_string()));
                }
            },
            // Core stack operations
            "dup" => {
                // Duplicate the top stack item
                if let Some(value) = self.stack.last() {
                    self.stack.push(value.clone());
                } else {
                    return Err(EvaluatorError::EvalError("dup requires a value on the stack".to_string()));
                }
            },
            "drop" => {
                // Remove the top stack item
                if self.stack.pop().is_none() {
                    return Err(EvaluatorError::EvalError("drop requires a value on the stack".to_string()));
                }
            },
            "swap" => {
                // Exchange the top two stack items
                if self.stack.len() >= 2 {
                    let idx = self.stack.len() - 1;
                    self.stack.swap(idx, idx - 1);
                } else {
                    return Err(EvaluatorError::EvalError("swap requires two values on the stack".to_string()));
                }
            },
            "rot" => {
                // Rotate third item to the top
                if self.stack.len() >= 3 {
                    let len = self.stack.len();
                    let third = self.stack.remove(len - 3);
                    self.stack.push(third);
                } else {
                    return Err(EvaluatorError::EvalError("rot requires three values on the stack".to_string()));
                }
            },
            "over" => {
                // Copy the second item to the top
                if self.stack.len() >= 2 {
                    let second = self.stack[self.stack.len() - 2].clone();
                    self.stack.push(second);
                } else {
                    return Err(EvaluatorError::EvalError("over requires two values on the stack".to_string()));
                }
            },
            "tuck" => {
                // Copy the top item to the third position
                if self.stack.len() >= 2 {
                    let top = self.stack.pop().unwrap();
                    let len = self.stack.len();
                    self.stack.insert(len - 1, top.clone());
                    self.stack.push(top);
                } else {
                    return Err(EvaluatorError::EvalError("tuck requires two values on the stack".to_string()));
                }
            },
            "pick" => {
                // Copy the nth item to the top
                if self.stack.len() >= 2 {
                    if let Some(Value::Number(n)) = self.stack.pop() {
                        if n < 0 || (n as usize) >= self.stack.len() {
                            return Err(EvaluatorError::EvalError(format!("Invalid pick depth: {}", n)));
                        }
                        let depth = n as usize;
                        let item = self.stack[self.stack.len() - 1 - depth].clone();
                        self.stack.push(item);
                    } else {
                        return Err(EvaluatorError::EvalError("pick requires a number on the stack".to_string()));
                    }
                } else {
                    return Err(EvaluatorError::EvalError("pick requires a depth and at least one other value".to_string()));
                }
            },
            
            // Resource operations
            "create_resource" => {
                // Pop a value and a resource type and create a resource
                if self.stack.len() >= 2 {
                    let resource_type = match self.stack.pop().unwrap() {
                        Value::String(s) => s,
                        _ => return Err(EvaluatorError::EvalError("create_resource requires a string resource type".to_string())),
                    };
                    let value = self.stack.pop().unwrap();
                    let resource = self.create_resource(&resource_type, value);
                    self.stack.push(resource);
                } else {
                    return Err(EvaluatorError::EvalError("create_resource requires a value and a resource type".to_string()));
                }
            },
            "consume_resource" => {
                // Pop a resource and consume it
                if let Some(value) = self.stack.pop() {
                    match self.consume_resource(&value) {
                        Ok(inner) => {
                            // Push the inner value back on the stack
                            self.stack.push(inner);
                        },
                        Err(e) => return Err(e),
                    }
                } else {
                    return Err(EvaluatorError::EvalError("consume_resource requires a resource on the stack".to_string()));
                }
            },
            "borrow" => {
                // Pop a resource and create a borrowed reference
                if let Some(value) = self.stack.pop() {
                    match self.borrow_resource(&value) {
                        Ok(borrowed) => {
                            self.stack.push(borrowed);
                        },
                        Err(e) => return Err(e),
                    }
                } else {
                    return Err(EvaluatorError::EvalError("borrow requires a resource on the stack".to_string()));
                }
            },
            "is_resource" => {
                // Check if a value is a resource
                if let Some(value) = self.stack.pop() {
                    let is_resource = value.is_resource();
                    self.stack.push(Value::Number(if is_resource { 1 } else { 0 }));
                } else {
                    return Err(EvaluatorError::EvalError("is_resource requires a value on the stack".to_string()));
                }
            },
            "resource_type" => {
                // Get the type of a resource
                if let Some(value) = self.stack.pop() {
                    if let Some(id) = value.get_resource_id() {
                        match self.resource_manager.resource_type(id) {
                            Ok(type_name) => {
                                self.stack.push(Value::String(type_name));
                            },
                            Err(e) => return Err(e),
                        }
                    } else {
                        return Err(EvaluatorError::EvalError("resource_type requires a resource value".to_string()));
                    }
                } else {
                    return Err(EvaluatorError::EvalError("resource_type requires a value on the stack".to_string()));
                }
            },
            "with_borrowed" => {
                // Set up a borrowing region, apply a quotation to a resource, and clean up
                if self.stack.len() >= 2 {
                    // Get the quotation and resource
                    let quotation = self.stack.pop().unwrap();
                    let resource = self.stack.pop().unwrap();
                    
                    // Check that we got a quotation and a resource
                    match quotation {
                        Value::Quotation(params, body, env) => {
                            if params.len() != 1 {
                                return Err(EvaluatorError::EvalError(
                                    "with_borrowed requires a quotation with exactly one parameter".to_string()
                                ));
                            }
                            
                            if !resource.is_resource() {
                                return Err(EvaluatorError::EvalError(
                                    "with_borrowed requires a resource as the second argument".to_string()
                                ));
                            }
                            
                            // Start a borrowing region
                            self.start_borrowing_region();
                            
                            // Create a borrowed resource
                            let borrowed = self.borrow_resource(&resource)?;
                            
                            // Push the borrowed resource
                            self.stack.push(borrowed);
                            
                            // Evaluate the quotation
                            // TODO: Implement proper quotation application
                            
                            // End the borrowing region
                            self.end_borrowing_region()?;
                        },
                        _ => return Err(EvaluatorError::EvalError(
                            "with_borrowed requires a quotation as the first argument".to_string()
                        )),
                    }
                } else {
                    return Err(EvaluatorError::EvalError(
                        "with_borrowed requires a resource and a quotation on the stack".to_string()
                    ));
                }
            },
            
            // Stack inspection
            ".s" => {
                // Print the current stack
                println!("Stack: {} items", self.stack.len());
                for (i, value) in self.stack.iter().enumerate() {
                    println!("{}: {}", i, value);
                }
            },
            "depth" => {
                // Push the current stack depth
                self.stack.push(Value::Number(self.stack.len() as i32));
            },
            ".resources" => {
                // Print information about resources
                println!("{}", self.resource_manager.stats());
            },
            // TODO: Implement other operations
            _ => return Err(EvaluatorError::EvalError(format!("Unknown operation: {}", operation))),
        }
        
        Ok(())
    }
    
    // Process a type template with unquote markers
    // This implements type quasiquotation with unquote support
    // Type checking functions

    // Infer type of an expression
    fn infer_type(&self, expr: &Expr) -> Result<Type> {
        match expr {
            Expr::Number(_) => Ok(Type::Simple("Num".to_string())),
            Expr::String(_) => Ok(Type::Simple("String".to_string())),
            Expr::Symbol(name) => {
                // Look up symbol in environment and get its type
                if let Some(value) = self.env.get(name) {
                    self.get_value_type(&value)
                } else {
                    Err(EvaluatorError::TypeError(format!(
                        "Cannot infer type of undefined symbol '{}'", name
                    )))
                }
            },
            Expr::Quotation(params, _) => {
                // Create a function type based on parameter types and inferred return type
                let param_types = params.iter()
                    .map(|p| p.type_annotation.clone().unwrap_or(Type::Simple("Any".to_string())))
                    .collect();
                
                // We can't reliably infer return type without evaluating, so use Any
                Ok(Type::Function(param_types, Box::new(Type::Simple("Any".to_string()))))
            },
            Expr::TypedQuotation(params, _, return_type) => {
                // Use the explicitly provided return type
                let param_types = params.iter()
                    .map(|p| p.type_annotation.clone().unwrap_or(Type::Simple("Any".to_string())))
                    .collect();
                
                Ok(Type::Function(param_types, return_type.clone()))
            },
            Expr::Pipeline(left, right) => {
                // The type of a pipeline is the type of its right side
                self.infer_type(right)
            },
            Expr::Binary(op, left, right) => {
                // Type of a binary operation depends on the operator
                match op.as_str() {
                    "+" | "-" | "*" | "/" => {
                        // Check if both operands are numeric
                        let left_type = self.infer_type(left)?;
                        let right_type = self.infer_type(right)?;
                        
                        if self.is_numeric_type(&left_type) && self.is_numeric_type(&right_type) {
                            Ok(Type::Simple("Num".to_string()))
                        } else {
                            Err(EvaluatorError::TypeError(format!(
                                "Cannot apply numeric operator '{}' to non-numeric types", op
                            )))
                        }
                    },
                    "==" | "!=" | "<" | ">" | "<=" | ">=" => {
                        // Comparison operators return boolean
                        Ok(Type::Simple("Bool".to_string()))
                    },
                    _ => {
                        // For other operators, default to Any
                        Ok(Type::Simple("Any".to_string()))
                    }
                }
            },
            Expr::Match(_, _) => {
                // For match expressions, the type depends on the branches
                // For simplicity, we'll default to Any
                Ok(Type::Simple("Any".to_string()))
            },
            Expr::TypeDef(_, _, _) => {
                // Type definitions don't have a runtime value
                Ok(Type::Simple("Type".to_string()))
            },
            Expr::Quote(inner) => {
                // A quoted expression has the type 'T where T is the inner type
                let inner_type = self.infer_type(inner)?;
                // Return a quoted type (this is a simplified representation)
                Ok(Type::Simple(format!("Quote[{}]", self.type_to_string(&inner_type)?)))
            },
            Expr::TypeQuote(inner_type) => {
                // A type quote has the type #Type
                Ok(Type::Simple("QuotedType".to_string()))
            },
            _ => {
                // Default for other expressions
                Ok(Type::Simple("Any".to_string()))
            }
        }
    }

    // Check if a value matches a specified type
    fn check_type(&self, value: &Value, expected_type: &Type) -> Result<()> {
        let value_type = self.get_value_type(value)?;
        
        // Check if the types are compatible
        if !self.types_compatible(&value_type, expected_type) {
            return Err(EvaluatorError::TypeError(format!(
                "Type mismatch: expected {}, but got {}",
                self.type_to_string(expected_type)?,
                self.type_to_string(&value_type)?
            )));
        }
        
        // Special handling for linear types
        if let Type::Linear(inner_type) = expected_type {
            // Require linear values for linear types
            match value {
                Value::Linear(_) => {
                    // Linear value for linear type - good
                    // Additional checks on the inner value would go here
                    if let Value::Linear(inner_value) = value {
                        return self.check_type(inner_value, inner_type);
                    }
                },
                _ => {
                    return Err(EvaluatorError::TypeError(format!(
                        "Expected linear value for linear type {}",
                        self.type_to_string(inner_type)?
                    )));
                }
            }
        }
        
        Ok(())
    }

    // Get the type of a runtime value
    fn get_value_type(&self, value: &Value) -> Result<Type> {
        match value {
            Value::Number(_) => Ok(Type::Simple("Num".to_string())),
            Value::String(_) => Ok(Type::Simple("String".to_string())),
            Value::Symbol(_) => Ok(Type::Simple("Symbol".to_string())),
            Value::Quotation(params, _, _) => {
                // Create function type from parameters
                let param_types = params.iter()
                    .map(|p| p.type_annotation.clone().unwrap_or(Type::Simple("Any".to_string())))
                    .collect();
                
                // Return type is unknown without evaluation
                Ok(Type::Function(param_types, Box::new(Type::Simple("Any".to_string()))))
            },
            Value::TypedQuotation(params, _, return_type, _) => {
                // Use the explicitly provided return type
                let param_types = params.iter()
                    .map(|p| p.type_annotation.clone().unwrap_or(Type::Simple("Any".to_string())))
                    .collect();
                
                Ok(Type::Function(param_types, Box::new(return_type.clone())))
            },
            Value::List(items) => {
                // If list is empty, default to List[Any]
                if items.is_empty() {
                    return Ok(Type::Generic("List".to_string(), vec![Type::Simple("Any".to_string())]));
                }
                
                // Try to infer common type from list items
                let mut common_type = self.get_value_type(&items[0])?;
                
                for item in &items[1..] {
                    let item_type = self.get_value_type(item)?;
                    // Find common supertype (simplified)
                    if common_type != item_type {
                        common_type = Type::Simple("Any".to_string());
                        break;
                    }
                }
                
                Ok(Type::Generic("List".to_string(), vec![common_type]))
            },
            Value::Map(_) => {
                // Maps could have heterogeneous keys and values
                // For simplicity, use Map[String, Any]
                Ok(Type::Generic("Map".to_string(), vec![
                    Type::Simple("String".to_string()),
                    Type::Simple("Any".to_string())
                ]))
            },
            Value::Type(t) => Ok(Type::Simple("Type".to_string())),
            Value::QuotedType(t) => Ok(Type::Simple("QuotedType".to_string())),
            Value::Linear(inner) => {
                // Get the inner type and wrap it as linear
                let inner_type = self.get_value_type(inner)?;
                Ok(Type::Linear(Box::new(inner_type)))
            },
            Value::Nothing => Ok(Type::Simple("Nothing".to_string())),
            Value::Nil => Ok(Type::Simple("Nil".to_string())),
            _ => Ok(Type::Simple("Any".to_string()))
        }
    }

    // Check if a value type is compatible with an expected type
    fn types_compatible(&self, actual: &Type, expected: &Type) -> bool {
        match (actual, expected) {
            // Any type is compatible with itself
            _ if actual == expected => true,
            
            // Any expected type accepts any actual type
            (_, Type::Simple(name)) if name == "Any" => true,
            
            // Linear types
            (Type::Linear(a), Type::Linear(b)) => self.types_compatible(a, b),
            
            // Optional types
            (Type::Optional(a), Type::Optional(b)) => self.types_compatible(a, b),
            
            // Function types
            (Type::Function(actual_params, actual_return), 
             Type::Function(expected_params, expected_return)) => {
                // Check parameter count
                if actual_params.len() != expected_params.len() {
                    return false;
                }
                
                // Check each parameter is compatible
                for (a, e) in actual_params.iter().zip(expected_params.iter()) {
                    if !self.types_compatible(a, e) {
                        return false;
                    }
                }
                
                // Check return type
                self.types_compatible(actual_return, expected_return)
            },
            
            // Generic types (like List[T])
            (Type::Generic(actual_name, actual_params),
             Type::Generic(expected_name, expected_params)) => {
                if actual_name != expected_name || actual_params.len() != expected_params.len() {
                    return false;
                }
                
                // Check type parameters
                for (a, e) in actual_params.iter().zip(expected_params.iter()) {
                    if !self.types_compatible(a, e) {
                        return false;
                    }
                }
                
                true
            },
            
            // Record types
            (Type::Record(actual_fields), Type::Record(expected_fields)) => {
                // Check if actual has at least all the expected fields
                for (field, expected_type) in expected_fields {
                    match actual_fields.get(field) {
                        Some(actual_type) => {
                            if !self.types_compatible(actual_type, expected_type) {
                                return false;
                            }
                        },
                        None => return false, // Missing required field
                    }
                }
                
                true
            },
            
            // Union types
            (actual, Type::Union(expected_types)) => {
                // Check if actual matches any type in the union
                expected_types.iter().any(|expected_type| 
                    self.types_compatible(actual, expected_type)
                )
            },
            
            // Default case: types are not compatible
            _ => false,
        }
    }

    // Convert a type to a string representation
    fn type_to_string(&self, typ: &Type) -> Result<String> {
        match typ {
            Type::Simple(name) => Ok(name.to_string()),
            Type::Linear(inner) => Ok(format!("!{}", self.type_to_string(inner)?)),
            Type::Optional(inner) => Ok(format!("?{}", self.type_to_string(inner)?)),
            Type::Generic(name, params) => {
                let param_strings: Result<Vec<String>> = params.iter()
                    .map(|p| self.type_to_string(p))
                    .collect();
                Ok(format!("{}[{}]", name, param_strings?.join(", ")))
            },
            Type::Function(params, return_type) => {
                let param_strings: Result<Vec<String>> = params.iter()
                    .map(|p| self.type_to_string(p))
                    .collect();
                Ok(format!("({}) => {}", 
                          param_strings?.join(", "), 
                          self.type_to_string(return_type)?))
            },
            Type::Record(fields) => {
                let mut field_strings = Vec::new();
                for (name, field_type) in fields {
                    field_strings.push(format!("{}: {}", 
                                              name, 
                                              self.type_to_string(field_type)?));
                }
                Ok(format!("{{ {} }}", field_strings.join(", ")))
            },
            Type::Union(types) => {
                let type_strings: Result<Vec<String>> = types.iter()
                    .map(|t| self.type_to_string(t))
                    .collect();
                Ok(type_strings?.join(" | "))
            },
            Type::Variant(variants) => {
                let mut variant_strings = Vec::new();
                for (name, variant_types) in variants {
                    let type_strings: Result<Vec<String>> = variant_types.iter()
                        .map(|t| self.type_to_string(t))
                        .collect();
                    variant_strings.push(format!("{}: {}", 
                                               name, 
                                               type_strings?.join(", ")));
                }
                Ok(format!("{{ {} }}", variant_strings.join(" | ")))
            },
        }
    }
    
    // Check if a type is numeric
    fn is_numeric_type(&self, typ: &Type) -> bool {
        match typ {
            Type::Simple(name) => name == "Num" || name == "Int" || name == "Float",
            _ => false,
        }
    }

    // Process a type template with unquote markers
    fn process_type_quasiquote(&self, template: &Type) -> Result<Type> {
        match template {
            Type::Simple(name) => {
                // Handle unquote markers in type names (e.g., $TypeName)
                if name.starts_with('$') {
                    let var_name = &name[1..]; // Remove the $ prefix
                    if let Some(value) = self.env.get(var_name) {
                        match value {
                            Value::Type(typ) => Ok(typ),
                            Value::QuotedType(typ) => Ok(typ),
                            _ => Err(EvaluatorError::TypeError(format!(
                                "Unquote variable '{}' is not a type", var_name
                            ))),
                        }
                    } else {
                        Err(EvaluatorError::EvalError(format!(
                            "Unquote variable '{}' not found", var_name
                        )))
                    }
                } else {
                    // Regular simple type, no processing needed
                    Ok(Type::Simple(name.clone()))
                }
            },
            Type::Linear(inner) => {
                // Process the inner type recursively
                let processed_inner = self.process_type_quasiquote(inner)?;
                Ok(Type::Linear(Box::new(processed_inner)))
            },
            Type::Optional(inner) => {
                // Process the inner type recursively
                let processed_inner = self.process_type_quasiquote(inner)?;
                Ok(Type::Optional(Box::new(processed_inner)))
            },
            Type::Generic(name, type_args) => {
                // Handle unquote markers in generic type names
                let processed_name = if name.starts_with('$') {
                    let var_name = &name[1..]; // Remove the $ prefix
                    if let Some(value) = self.env.get(var_name) {
                        match value {
                            Value::String(s) => s,
                            _ => return Err(EvaluatorError::TypeError(format!(
                                "Unquote variable '{}' is not a string for generic type name", var_name
                            ))),
                        }
                    } else {
                        return Err(EvaluatorError::EvalError(format!(
                            "Unquote variable '{}' not found", var_name
                        )));
                    }
                } else {
                    name.clone()
                };
                
                // Process each type argument recursively
                let mut processed_args = Vec::new();
                for arg in type_args {
                    processed_args.push(self.process_type_quasiquote(arg)?);
                }
                
                Ok(Type::Generic(processed_name, processed_args))
            },
            Type::Union(types) => {
                // Process each union member recursively
                let mut processed_types = Vec::new();
                for typ in types {
                    processed_types.push(self.process_type_quasiquote(typ)?);
                }
                
                Ok(Type::Union(processed_types))
            },
            Type::Record(fields) => {
                // Process each field type recursively
                let mut processed_fields = HashMap::new();
                
                for (field_name, field_type) in fields {
                    // Handle unquote markers in field names
                    let processed_name = if field_name.starts_with('$') {
                        let var_name = &field_name[1..]; // Remove the $ prefix
                        if let Some(value) = self.env.get(var_name) {
                            match value {
                                Value::String(s) => s,
                                _ => return Err(EvaluatorError::TypeError(format!(
                                    "Unquote variable '{}' is not a string for field name", var_name
                                ))),
                            }
                        } else {
                            return Err(EvaluatorError::EvalError(format!(
                                "Unquote variable '{}' not found", var_name
                            )));
                        }
                    } else if field_name.ends_with("...") {
                        // Handle record field spreading
                        let var_name = &field_name[..field_name.len() - 3]; // Remove the ... suffix
                        if let Some(value) = self.env.get(var_name) {
                            match value {
                                Value::Type(Type::Record(spread_fields)) => {
                                    // Add all the fields from the record to our processed fields
                                    for (k, v) in spread_fields {
                                        processed_fields.insert(k.clone(), self.process_type_quasiquote(&v)?);
                                    }
                                    continue; // Skip the normal field insertion
                                },
                                _ => return Err(EvaluatorError::TypeError(format!(
                                    "Spread variable '{}' is not a record type", var_name
                                ))),
                            }
                        } else {
                            return Err(EvaluatorError::EvalError(format!(
                                "Spread variable '{}' not found", var_name
                            )));
                        }
                    } else {
                        field_name.clone()
                    };
                    
                    // Process the field type
                    let processed_type = self.process_type_quasiquote(field_type)?;
                    processed_fields.insert(processed_name, processed_type);
                }
                
                Ok(Type::Record(processed_fields))
            },
            Type::Variant(variants) => {
                // Process each variant recursively
                let mut processed_variants = HashMap::new();
                
                for (variant_name, variant_types) in variants {
                    // Handle unquote markers in variant names
                    let processed_name = if variant_name.starts_with('$') {
                        let var_name = &variant_name[1..]; // Remove the $ prefix
                        if let Some(value) = self.env.get(var_name) {
                            match value {
                                Value::String(s) => s,
                                _ => return Err(EvaluatorError::TypeError(format!(
                                    "Unquote variable '{}' is not a string for variant name", var_name
                                ))),
                            }
                        } else {
                            return Err(EvaluatorError::EvalError(format!(
                                "Unquote variable '{}' not found", var_name
                            )));
                        }
                    } else {
                        variant_name.clone()
                    };
                    
                    // Process the variant types
                    let mut processed_types = Vec::new();
                    for typ in variant_types {
                        processed_types.push(self.process_type_quasiquote(typ)?);
                    }
                    
                    processed_variants.insert(processed_name, processed_types);
                }
                
                Ok(Type::Variant(processed_variants))
            },
            Type::Function(param_types, return_type) => {
                // Process each parameter type recursively
                let mut processed_params = Vec::new();
                for param in param_types {
                    processed_params.push(self.process_type_quasiquote(param)?);
                }
                
                // Process the return type
                let processed_return = self.process_type_quasiquote(return_type)?;
                
                Ok(Type::Function(processed_params, Box::new(processed_return)))
            },
        }
    }
}