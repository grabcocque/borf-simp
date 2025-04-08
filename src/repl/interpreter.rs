// src/repl/interpreter.rs
// This module provides the Borf interpreter code used by the REPL

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum EvaluatorError {
    #[error("File error: {0}")]
    FileError(#[from] std::io::Error),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Evaluation error: {0}")]
    EvalError(String),

    #[error("Type error: {0}")]
    TypeError(String),
}

pub type Result<T> = std::result::Result<T, EvaluatorError>;

// Re-export the core Borf structures from main.rs
// Note: In a production implementation, these would be properly structured
// Here we're just creating a thin wrapper around the existing functionality

// AST representation
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Number(i32),
    String(String),
    Symbol(String),
    Quotation(Vec<Param>, Vec<Expr>), // Includes parameter list
    TypedQuotation(Vec<Param>, Vec<Expr>, Box<Type>), // Unified function with params, body, and return type
    Pipeline(Box<Expr>, Box<Expr>),
    Match(Box<Expr>, Vec<(Pattern, Expr)>),
    Binary(String, Box<Expr>, Box<Expr>), // Binary operations
    Assignment(Box<Expr>, String),        // Variable assignment: expr -> name
    Module(String, Vec<Expr>, Vec<Expr>), // Module with name, imports, and definitions
    Import(String),                       // Import another module
    TypeDef(String, Vec<TypeParam>, Box<Type>), // Type definition
    Quote(Box<Expr>),                     // Quoted expression 'expr
    Unquote(Box<Expr>),                   // Unquoted expression $expr
    Quasiquote(Box<Expr>),                // Quasiquoted expression `expr` (template)
    TypeQuote(Box<Type>),                 // Quoted type #Type
    TypeUnquote(Box<Expr>),               // Unquoted type expression $T
    FunctionType(Vec<Type>, Box<Type>),   // Function type declaration
}

// Parameter for quotations
#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub name: String,
    pub type_annotation: Option<Type>,
}

// Type parameter for generic types
#[derive(Debug, Clone, PartialEq)]
pub struct TypeParam {
    pub name: String,
    pub is_linear: bool,
}

// Type representation
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Simple(String),                      // Simple types like Num, String, etc.
    Linear(Box<Type>),                   // Linear types marked with !
    Optional(Box<Type>),                 // Optional types marked with ?
    Generic(String, Vec<Type>),          // Generic types like List[T]
    Union(Vec<Type>),                    // Union types like A | B
    Record(HashMap<String, Type>),       // Record types like { x: Num, y: String }
    Variant(HashMap<String, Vec<Type>>), // Variant types like { tag: val }
    Function(Vec<Type>, Box<Type>),      // Function types (a,b) => c
}

// Pattern for match expressions
#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    Wildcard,                      // _ (matches anything)
    Literal(Expr),                 // Literal patterns like 42 or "hello"
    Map(HashMap<String, Pattern>), // Map patterns like {name: "Alice", age: 30}
    Variable(String),              // Variable binding patterns
    Quote(Box<Pattern>),           // Quoted pattern 'pattern
    TypePattern(Type),             // Type pattern matching
    Variant(String, Vec<Pattern>), // Variant pattern like Some x or None
    Linear(Box<Pattern>),          // Linear pattern !pattern
}

// Environment to store bound values
#[derive(Debug, Clone, PartialEq)]
pub struct Env {
    bindings: HashMap<String, Value>,
    parent: Option<Box<Env>>,
}

// Value representation for the Borf language
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Number(i32),
    String(String),
    Symbol(String),
    Quotation(Vec<Param>, Vec<Expr>, Option<Box<Env>>), // Includes closure environment
    TypedQuotation(Vec<Param>, Vec<Expr>, Type, Option<Box<Env>>), // Typed function with return type
    Pipeline(Box<Value>, Box<Value>),
    List(Vec<Value>),
    Map(HashMap<String, Value>),
    Quoted(Box<Value>),                     // Quoted value 'value
    Quasiquoted(Box<Value>),                // Quasiquoted value `value` (template)
    Type(Type),                             // Type value
    QuotedType(Type),                       // Quoted type #Type
    Module(String, HashMap<String, Value>), // Module with name and definitions
    Linear(Box<Value>),                     // Linear value !value (must be used exactly once)
    Optional(Option<Box<Value>>),           // Optional value ?value (value or Nothing)
    Variant(String, Vec<Value>),            // Variant like tag(val)
    Nothing,                                // Represents "Nothing" value
    Nil,                                    // For internal use
}

pub struct Parser {
    tokens: Vec<String>,
    position: usize,
}

impl Parser {
    pub fn new(input: &str) -> Self {
        let tokens = Self::tokenize(input);
        Parser {
            tokens,
            position: 0,
        }
    }

    pub fn tokenize(input: &str) -> Vec<String> {
        // Simplified tokenization for our REPL example
        let mut tokens = Vec::new();
        let mut current_token = String::new();
        let mut in_string = false;
        let mut in_comment = false;

        let mut chars = input.chars().peekable();

        while let Some(c) = chars.next() {
            // Handle comments
            if !in_string && c == '-' && chars.peek() == Some(&'-') {
                chars.next(); // Skip the second dash
                in_comment = true;

                if !current_token.is_empty() {
                    tokens.push(current_token);
                    current_token = String::new();
                }

                // Skip until end of line
                while let Some(next_c) = chars.next() {
                    if next_c == '\n' {
                        in_comment = false;
                        break;
                    }
                }
                continue;
            }

            // Skip if in comment
            if in_comment {
                if c == '\n' {
                    in_comment = false;
                }
                continue;
            }

            match c {
                '"' => {
                    current_token.push(c);
                    in_string = !in_string;
                    if !in_string {
                        tokens.push(current_token);
                        current_token = String::new();
                    }
                }
                ' ' | '\t' | '\n' | '\r' if !in_string => {
                    if !current_token.is_empty() {
                        tokens.push(current_token);
                        current_token = String::new();
                    }
                }
                '[' | ']' | '{' | '}' | '(' | ')' | '|' | '>' | ',' | '-' | ':' | '=' | '!'
                | '\'' | '$'
                    if !in_string =>
                {
                    if !current_token.is_empty() {
                        tokens.push(current_token);
                        current_token = String::new();
                    }

                    // Handle special cases for token combinations
                    if c == '|' && chars.peek() == Some(&'>') {
                        chars.next(); // consume the '>'
                        tokens.push("|>".to_string());
                    } else if c == '-' && chars.peek() == Some(&'>') {
                        chars.next(); // consume the '>'
                        tokens.push("->".to_string());
                    } else if c == '=' && chars.peek() == Some(&'>') {
                        chars.next(); // consume the '>'
                        tokens.push("=>".to_string());
                    } else if c == ':' && chars.peek() == Some(&':') {
                        chars.next(); // consume the second ':'
                        tokens.push("::".to_string());
                    } else {
                        tokens.push(c.to_string());
                    }
                }
                _ => {
                    current_token.push(c);
                }
            }
        }

        if !current_token.is_empty() && !in_comment {
            tokens.push(current_token);
        }

        tokens
    }

    pub fn parse(&mut self) -> Result<Expr> {
        if self.tokens.is_empty() {
            return Err(EvaluatorError::ParseError("Empty input".to_string()));
        }
        self.parse_expr()
    }

    // Simplified parser that just parses basic expressions
    fn parse_expr(&mut self) -> Result<Expr> {
        if self.position >= self.tokens.len() {
            return Err(EvaluatorError::ParseError(
                "Unexpected end of input".to_string(),
            ));
        }

        let token = &self.tokens[self.position];
        self.position += 1;

        if let Ok(num) = token.parse::<i32>() {
            Ok(Expr::Number(num))
        } else if token.starts_with('"') && token.ends_with('"') && token.len() >= 2 {
            let content = &token[1..token.len() - 1];
            Ok(Expr::String(content.to_string()))
        } else if token == "[" {
            // Parse a quotation with parameters and possible type annotation
            let mut params = Vec::new();
            let mut exprs = Vec::new();

            // Check if the first token(s) might be parameters before "->"
            let mut has_params = false;
            let mut param_tokens = Vec::new();
            let mut temp_pos = self.position;

            // Look ahead to see if there's a "->" which indicates parameters
            while temp_pos < self.tokens.len() {
                let t = &self.tokens[temp_pos];
                if t == "->" {
                    has_params = true;
                    break;
                } else if t == "]" {
                    // End of quotation without parameters
                    break;
                }
                param_tokens.push(t.clone());
                temp_pos += 1;
            }

            // If we have parameters, parse them
            if has_params {
                // Parse comma-separated parameters with possible type annotations
                while self.position < self.tokens.len() {
                    let t = &self.tokens[self.position];

                    if t == "->" {
                        self.position += 1; // Skip the arrow
                        break;
                    } else if t == "," {
                        self.position += 1; // Skip comma and continue
                    } else {
                        // Parse a parameter
                        let param_name = self.tokens[self.position].clone();
                        self.position += 1;

                        // Check for type annotation
                        let mut type_annotation = None;
                        if self.position < self.tokens.len() && &self.tokens[self.position] == ":" {
                            self.position += 1; // Skip the colon

                            // Parse the type
                            // In a full implementation, call parse_type() here
                            // For simplicity, we'll just take the next token as the type
                            if self.position < self.tokens.len() {
                                let type_name = self.tokens[self.position].clone();
                                self.position += 1;
                                type_annotation = Some(Type::Simple(type_name));
                            }
                        }

                        params.push(Param {
                            name: param_name,
                            type_annotation,
                        });
                    }
                }
            }

            // Parse the body of the quotation
            while self.position < self.tokens.len() && &self.tokens[self.position] != "]" {
                exprs.push(self.parse_expr()?);
            }

            if self.position < self.tokens.len() {
                self.position += 1; // Skip the closing ']'

                // Check for type annotation after the quotation
                if self.position < self.tokens.len() && &self.tokens[self.position] == ":" {
                    self.position += 1; // Skip the colon

                    // Parse the return type
                    // In a full implementation, call parse_type() here
                    // For simplicity, we'll just take the next token as the return type
                    if self.position < self.tokens.len() {
                        let type_name = self.tokens[self.position].clone();
                        self.position += 1;

                        // Create a unified function definition with type annotation
                        return Ok(Expr::TypedQuotation(
                            params,
                            exprs,
                            Box::new(Type::Simple(type_name)),
                        ));
                    }
                }

                // Regular quotation without return type
                Ok(Expr::Quotation(params, exprs))
            } else {
                Err(EvaluatorError::ParseError("Unclosed quotation".to_string()))
            }
        } else {
            // For simplicity, treat everything else as a symbol
            Ok(Expr::Symbol(token.clone()))
        }
    }
}

// Evaluator for Borf
pub struct Evaluator {
    env: Env,
    stack: Vec<Value>,
    prelude_path: PathBuf,
}

impl Evaluator {
    pub fn new() -> Self {
        Evaluator {
            env: Env::new(),
            stack: Vec::new(),
            prelude_path: PathBuf::from("src/prelude"),
        }
    }

    pub fn with_prelude_path<P: AsRef<Path>>(prelude_path: P) -> Self {
        Evaluator {
            env: Env::new(),
            stack: Vec::new(),
            prelude_path: prelude_path.as_ref().to_path_buf(),
        }
    }

    // Set up the built-in functions and values
    pub fn initialize(&mut self) -> Result<()> {
        // Add built-in functions
        self.env.set("print", Value::Symbol("print".to_string()));
        self.env.set("add", Value::Symbol("add".to_string()));
        self.env.set("sub", Value::Symbol("sub".to_string()));
        self.env.set("mul", Value::Symbol("mul".to_string()));
        self.env.set("div", Value::Symbol("div".to_string()));

        // Load the prelude
        self.load_prelude()?;

        Ok(())
    }

    // Load the prelude modules
    fn load_prelude(&mut self) -> Result<()> {
        let prim_path = self.prelude_path.join("prim/prim.borf");
        if prim_path.exists() {
            self.eval_file(prim_path)?;
        }

        let syntax_path = self.prelude_path.join("syntax/syntax.borf");
        if syntax_path.exists() {
            self.eval_file(syntax_path)?;
        }

        // Load the meta module by default
        // This makes the Borf-in-Borf metacircular evaluator available to the system
        let meta_path = self.prelude_path.join("meta/meta.borf");
        if meta_path.exists() {
            self.eval_file(meta_path)?;
        }
        
        // Directly load the Borf-in-Borf implementation
        let borf_in_borf_path = self.prelude_path.join("meta/borf_in_borf.borf");
        if borf_in_borf_path.exists() {
            // We don't actually evaluate this here because it requires special initialization
            // It's loaded dynamically when needed by the metacircular REPL and eval functions
            println!("✓ Detected Borf-in-Borf metacircular evaluator");
            
            // Add debug info about the actual file content
            match std::fs::read_to_string(&borf_in_borf_path) {
                Ok(content) => {
                    println!("Borf-in-Borf file is {} lines long", content.lines().count());
                    let first_few_lines = content.lines().take(5).collect::<Vec<_>>().join("\n");
                    println!("First few lines:\n{}", first_few_lines);
                },
                Err(e) => println!("Error reading borf_in_borf.borf: {}", e)
            }
        }

        Ok(())
    }

    // Evaluate Borf code
    pub fn eval(&mut self, code: &str) -> Result<String> {
        let mut parser = Parser::new(code);
        let expr = parser.parse()?;

        match self.eval_expr(&expr)? {
            Some(value) => Ok(self.value_to_string(&value)),
            None => {
                // If there's a value on the stack, return that
                if let Some(value) = self.stack.last() {
                    Ok(self.value_to_string(value))
                } else {
                    Ok("".to_string())
                }
            }
        }
    }

    // Evaluate a file
    pub fn eval_file<P: AsRef<Path>>(&mut self, path: P) -> Result<String> {
        let path_ref = path.as_ref();
        println!("Loading file: {}", path_ref.display());
        
        let content = fs::read_to_string(path_ref)?;
        println!("File loaded: {} lines, {} bytes", 
                 content.lines().count(), 
                 content.len());
                 
        // Display first few lines of the file
        if path_ref.file_name().unwrap_or_default().to_string_lossy().contains("borf_in_borf") {
            let first_few_lines = content.lines().take(5).collect::<Vec<_>>().join("\n");
            println!("First few lines:\n{}", first_few_lines);
        }
        
        self.eval(&content)
    }

    // Convert a value to a string for display
    pub fn value_to_string(&self, value: &Value) -> String {
        match value {
            Value::Number(n) => n.to_string(),
            Value::String(s) => format!("\"{}\"", s),
            Value::Symbol(s) => s.clone(),
            Value::Quotation(_, _, _) => "[quotation]".to_string(),
            Value::TypedQuotation(_, _, return_type, _) => {
                format!("[quotation: {}]", format!("{:?}", return_type))
            }
            Value::List(items) => {
                let mut result = "[".to_string();
                for (i, item) in items.iter().enumerate() {
                    result.push_str(&self.value_to_string(item));
                    if i < items.len() - 1 {
                        result.push_str(", ");
                    }
                }
                result.push(']');
                result
            }
            Value::Map(_) => "{...}".to_string(),
            Value::Quoted(inner) => format!("'{}", self.value_to_string(inner)),
            Value::Quasiquoted(inner) => format!("`{}", self.value_to_string(inner)),
            Value::Type(typ) => format!("{:?}", typ),
            Value::QuotedType(typ) => format!("#{:?}", typ),
            Value::Module(name, _) => format!("module {}", name),
            Value::Linear(inner) => format!("!{}", self.value_to_string(inner)),
            Value::Optional(Some(inner)) => format!("?{}", self.value_to_string(inner)),
            Value::Optional(None) => "Nothing".to_string(),
            Value::Nothing => "Nothing".to_string(),
            Value::Variant(name, values) => {
                let mut result = name.clone();
                if !values.is_empty() {
                    result.push('(');
                    for (i, val) in values.iter().enumerate() {
                        result.push_str(&self.value_to_string(val));
                        if i < values.len() - 1 {
                            result.push_str(", ");
                        }
                    }
                    result.push(')');
                }
                result
            }
            Value::Nil => "nil".to_string(),
            _ => format!("{:?}", value),
        }
    }

    // Evaluate an expression - simplified for REPL example
    fn eval_expr(&mut self, expr: &Expr) -> Result<Option<Value>> {
        match expr {
            Expr::Number(n) => Ok(Some(Value::Number(*n))),
            Expr::String(s) => Ok(Some(Value::String(s.clone()))),
            Expr::Symbol(s) => {
                // Check if it's a variable in the environment
                if let Some(value) = self.env.get(s) {
                    Ok(Some(value))
                } else {
                    // Execute the operation
                    self.execute_operation(s)?;
                    Ok(None)
                }
            }
            Expr::Quotation(params, exprs) => {
                // Create a quotation value with the current environment
                Ok(Some(Value::Quotation(
                    params.clone(),
                    exprs.clone(),
                    Some(Box::new(self.env.clone())),
                )))
            }
            Expr::TypedQuotation(params, exprs, return_type) => {
                // Create a typed quotation value with the current environment and return type
                Ok(Some(Value::TypedQuotation(
                    params.clone(),
                    exprs.clone(),
                    return_type.as_ref().clone(),
                    Some(Box::new(self.env.clone())),
                )))
            }
            Expr::Assignment(value_expr, name) => {
                // Evaluate the expression and bind the result to the name
                if let Some(value) = self.eval_expr(value_expr)? {
                    self.env.set(name, value.clone());
                    Ok(Some(value))
                } else {
                    Err(EvaluatorError::EvalError(format!(
                        "Cannot assign None to {}",
                        name
                    )))
                }
            }
            Expr::TypeQuote(typ) => {
                // Quote a type
                Ok(Some(Value::QuotedType(typ.as_ref().clone())))
            },
            Expr::Module(name, imports, definitions) => {
                // Create a new module with the given name, imports, and definitions
                println!("Creating module: {}", name);
                
                // Process imports first
                for import in imports {
                    if let Expr::Import(module_name) = import {
                        println!("Importing module: {}", module_name);
                        // In a full implementation, we would load the module and add its values to this module
                    } else {
                        return Err(EvaluatorError::EvalError(
                            "Invalid import expression in module definition".to_string(),
                        ));
                    }
                }
                
                // Create a new module value with an empty map of symbols
                let module_value = Value::Module(name.clone(), HashMap::new());
                
                // Store the module in the environment
                self.env.set(&name, module_value.clone());
                
                // Evaluate the definitions in the module's context
                for def in definitions {
                    self.eval_expr(def)?;
                }
                
                Ok(Some(module_value))
            },
            Expr::Import(module_name) => {
                // Import a module
                println!("Importing module: {}", module_name);
                
                // In a full implementation, we would load the module and add its values to the current environment
                // For now, just create a dummy import operation
                self.stack.push(Value::Symbol(module_name.clone()));
                self.execute_operation("import")?;
                
                Ok(None)
            },
            // Other expressions would be handled here
            _ => Err(EvaluatorError::EvalError(format!(
                "Unsupported expression: {:?}",
                expr
            ))),
        }
    }

    // Execute an operation
    fn execute_operation(&mut self, operation: &str) -> Result<()> {
        match operation {
            "add" => {
                if self.stack.len() < 2 {
                    return Err(EvaluatorError::EvalError(
                        "Not enough operands for addition".to_string(),
                    ));
                }

                let b = self.stack.pop().unwrap();
                let a = self.stack.pop().unwrap();

                match (a, b) {
                    (Value::Number(a), Value::Number(b)) => {
                        self.stack.push(Value::Number(a + b));
                        Ok(())
                    }
                    (Value::String(a), Value::String(b)) => {
                        self.stack.push(Value::String(format!("{}{}", a, b)));
                        Ok(())
                    }
                    _ => Err(EvaluatorError::TypeError(
                        "Can only add numbers or strings".to_string(),
                    )),
                }
            }
            "sub" => {
                if self.stack.len() < 2 {
                    return Err(EvaluatorError::EvalError(
                        "Not enough operands for subtraction".to_string(),
                    ));
                }

                let b = self.stack.pop().unwrap();
                let a = self.stack.pop().unwrap();

                match (a, b) {
                    (Value::Number(a), Value::Number(b)) => {
                        self.stack.push(Value::Number(a - b));
                        Ok(())
                    }
                    _ => Err(EvaluatorError::TypeError(
                        "Can only subtract numbers".to_string(),
                    )),
                }
            }
            "mul" => {
                if self.stack.len() < 2 {
                    return Err(EvaluatorError::EvalError(
                        "Not enough operands for multiplication".to_string(),
                    ));
                }

                let b = self.stack.pop().unwrap();
                let a = self.stack.pop().unwrap();

                match (a, b) {
                    (Value::Number(a), Value::Number(b)) => {
                        self.stack.push(Value::Number(a * b));
                        Ok(())
                    }
                    _ => Err(EvaluatorError::TypeError(
                        "Can only multiply numbers".to_string(),
                    )),
                }
            }
            "div" => {
                if self.stack.len() < 2 {
                    return Err(EvaluatorError::EvalError(
                        "Not enough operands for division".to_string(),
                    ));
                }

                let b = self.stack.pop().unwrap();
                let a = self.stack.pop().unwrap();

                match (a, b) {
                    (Value::Number(a), Value::Number(b)) => {
                        if b == 0 {
                            return Err(EvaluatorError::EvalError("Division by zero".to_string()));
                        }
                        self.stack.push(Value::Number(a / b));
                        Ok(())
                    }
                    _ => Err(EvaluatorError::TypeError(
                        "Can only divide numbers".to_string(),
                    )),
                }
            },
            "%" => {
                // Modulo operation
                if self.stack.len() < 2 {
                    return Err(EvaluatorError::EvalError(
                        "Not enough operands for modulo".to_string(),
                    ));
                }

                let b = self.stack.pop().unwrap();
                let a = self.stack.pop().unwrap();

                match (a, b) {
                    (Value::Number(a), Value::Number(b)) => {
                        if b == 0 {
                            return Err(EvaluatorError::EvalError("Modulo by zero".to_string()));
                        }
                        // Use the rem_euclid for consistent modulo behavior
                        self.stack.push(Value::Number(a.rem_euclid(b)));
                        Ok(())
                    }
                    _ => Err(EvaluatorError::TypeError(
                        "Can only perform modulo on numbers".to_string(),
                    )),
                }
            }
            // Renamed to print_legacy to avoid duplication
            "print_legacy" => {
                if let Some(value) = self.stack.pop() {
                    println!("{}", self.value_to_string(&value));
                    Ok(())
                } else {
                    Err(EvaluatorError::EvalError("Nothing to print".to_string()))
                }
            }
            "dup" => {
                if let Some(value) = self.stack.last() {
                    self.stack.push(value.clone());
                    Ok(())
                } else {
                    Err(EvaluatorError::EvalError(
                        "Nothing to duplicate".to_string(),
                    ))
                }
            }
            "drop" => {
                if self.stack.pop().is_some() {
                    Ok(())
                } else {
                    Err(EvaluatorError::EvalError("Nothing to drop".to_string()))
                }
            }
            "swap" => {
                if self.stack.len() < 2 {
                    return Err(EvaluatorError::EvalError(
                        "Not enough values to swap".to_string(),
                    ));
                }

                let len = self.stack.len();
                self.stack.swap(len - 1, len - 2);
                Ok(())
            }
            "clear" => {
                self.stack.clear();
                Ok(())
            },
            "module" => {
                // Module creation operation for metacircular evaluation
                if self.stack.is_empty() {
                    return Err(EvaluatorError::EvalError(
                        "Not enough operands for module (name)".to_string(),
                    ));
                }
                
                let module_name = match self.stack.pop().unwrap() {
                    Value::String(s) => s,
                    Value::Symbol(ref s) => s.clone(),
                    _ => {
                        return Err(EvaluatorError::EvalError(
                            "Module name must be a string or symbol".to_string(),
                        ));
                    }
                };
                
                println!("Creating module: {}", module_name);
                
                // Create a module with its name and empty exports
                let mut module_exports = HashMap::new();
                let module_value = Value::Module(module_name.clone(), module_exports);
                
                // Register the module in the environment
                self.env.set(&module_name, module_value.clone());
                
                // Return the module
                self.stack.push(module_value);
                Ok(())
            },
            "This" => {
                // This keyword is used in comments in the Borf-in-Borf file
                // Seems to be getting parsed incorrectly. Just return a dummy value.
                println!("'This' keyword called - this is likely a parsing error");
                Ok(())
            },
            "env" => {
                // This operation is used in the REPL code to create an environment
                println!("Creating new environment");
                
                // Create a new environment and push it onto the stack
                let _new_env = Env::new();
                self.stack.push(Value::Symbol("environment".to_string()));
                Ok(())
            },
            "true" => {
                // Boolean true value
                self.stack.push(Value::Number(1)); // Represent true as 1
                Ok(())
            },
            "false" => {
                // Boolean false value
                self.stack.push(Value::Number(0)); // Represent false as 0
                Ok(())
            },
            "read_line" => {
                // Read a line of input from the user
                use std::io::{self, BufRead};
                let stdin = io::stdin();
                let mut line = String::new();
                stdin.lock().read_line(&mut line).map_err(|e| 
                    EvaluatorError::EvalError(format!("Error reading line: {}", e)))?;
                
                // Trim the trailing newline
                if line.ends_with('\n') {
                    line.pop();
                    if line.ends_with('\r') {
                        line.pop();
                    }
                }
                
                self.stack.push(Value::String(line));
                Ok(())
            },
            "parse" => {
                // Parse a string into an AST
                if self.stack.is_empty() {
                    return Err(EvaluatorError::EvalError(
                        "Not enough operands for parse".to_string(),
                    ));
                }
                
                // Get the string to parse
                let code = match self.stack.pop().unwrap() {
                    Value::String(s) => s,
                    _ => {
                        return Err(EvaluatorError::TypeError(
                            "parse expects a string".to_string(),
                        ));
                    }
                };
                
                // Parse the string
                let mut parser = Parser::new(&code);
                match parser.parse() {
                    Ok(expr) => {
                        // Convert the expression to a proper quoted value
                        // This is a more sophisticated implementation that handles different expression types
                        let quoted_expr = match expr {
                            Expr::Number(n) => Value::Quoted(Box::new(Value::Number(n))),
                            Expr::String(s) => Value::Quoted(Box::new(Value::String(s))),
                            Expr::Symbol(s) => Value::Quoted(Box::new(Value::Symbol(s))),
                            Expr::Quotation(params, body) => {
                                // For a quotation, we'd need to properly represent the params and body
                                // For simplicity, we'll just use a placeholder representation
                                Value::Quoted(Box::new(Value::Quotation(
                                    params,
                                    body,
                                    Some(Box::new(self.env.clone()))
                                )))
                            },
                            // Other expression types would be handled here in a complete implementation
                            _ => {
                                // For other types, we'll use a basic string representation for now
                                // In a real implementation, we'd properly convert all types
                                Value::Quoted(Box::new(Value::Symbol(format!("{:?}", expr))))
                            }
                        };
                        
                        // Push the parsed and quoted expression onto the stack
                        self.stack.push(quoted_expr);
                        Ok(())
                    },
                    Err(err) => {
                        Err(err)
                    }
                }
            },
            "evaluate" => {
                // Evaluate an expression in an environment
                if self.stack.len() < 2 {
                    return Err(EvaluatorError::EvalError(
                        "Not enough operands for evaluation".to_string(),
                    ));
                }
                
                // Get the environment
                let env_val = self.stack.pop().unwrap();
                
                // Get the expression to evaluate
                let expr = self.stack.pop().unwrap();
                
                match expr {
                    Value::Quoted(boxed_expr) => {
                        // Try to convert the boxed expression to an Expr
                        match *boxed_expr {
                            Value::Number(n) => {
                                // Number literal expression - just return the number
                                self.stack.push(Value::Number(n));
                            },
                            Value::String(s) => {
                                // String literal expression - just return the string
                                self.stack.push(Value::String(s));
                            },
                            Value::Symbol(s) => {
                                // Symbol expression - treat as a variable lookup or operation
                                // Look up in the provided environment
                                match env_val {
                                    Value::Map(env_map) => {
                                        if let Some(Value::Map(bindings)) = env_map.get("bindings") {
                                            if let Some(value) = bindings.get(&s) {
                                                self.stack.push(value.clone());
                                            } else if let Some(parent) = env_map.get("parent") {
                                                if !matches!(parent, Value::Nothing) {
                                                    // Try parent lookup
                                                    self.stack.push(parent.clone());
                                                    self.stack.push(Value::Symbol(s));
                                                    self.execute_operation("env_lookup")?;
                                                } else {
                                                    // Just return the symbol if not found
                                                    self.stack.push(Value::Symbol(s));
                                                }
                                            } else {
                                                // Just return the symbol if not found
                                                self.stack.push(Value::Symbol(s));
                                            }
                                        } else {
                                            // Invalid environment, just return the symbol
                                            self.stack.push(Value::Symbol(s));
                                        }
                                    },
                                    _ => {
                                        // Invalid environment, just return the symbol
                                        self.stack.push(Value::Symbol(s));
                                    }
                                }
                            },
                            Value::List(items) => {
                                // List expression - evaluate each item
                                let mut result = Vec::new();
                                for item in items {
                                    self.stack.push(item.clone());
                                    self.stack.push(env_val.clone());
                                    self.execute_operation("evaluate")?;
                                    if let Some(val) = self.stack.pop() {
                                        result.push(val);
                                    }
                                }
                                self.stack.push(Value::List(result));
                            },
                            Value::Map(fields) => {
                                // Map expression - evaluate each field
                                let mut result = HashMap::new();
                                for (key, value) in fields {
                                    self.stack.push(value.clone());
                                    self.stack.push(env_val.clone());
                                    self.execute_operation("evaluate")?;
                                    if let Some(val) = self.stack.pop() {
                                        result.insert(key, val);
                                    }
                                }
                                self.stack.push(Value::Map(result));
                            },
                            Value::Quotation(params, body, _) => {
                                // Quotation - create a new one with the given environment
                                self.stack.push(Value::Quotation(
                                    params,
                                    body,
                                    Some(Box::new(self.env.clone()))
                                ));
                            },
                            _ => {
                                // For other expression types, return as is
                                self.stack.push(*boxed_expr);
                            }
                        }
                    },
                    _ => {
                        // For non-quoted expressions, we just return them as is
                        self.stack.push(expr);
                    }
                }
                
                Ok(())
            },
            "value_to_string" => {
                // Convert a value to a string
                if self.stack.is_empty() {
                    return Err(EvaluatorError::EvalError(
                        "Not enough operands for value_to_string".to_string(),
                    ));
                }
                
                // Get the value
                let value = self.stack.pop().unwrap();
                
                // Convert the value to a string
                let str_val = self.value_to_string(&value);
                
                // Push the string onto the stack
                self.stack.push(Value::String(str_val));
                Ok(())
            },
            "import" => {
                // Basic implementation of import operation
                if self.stack.is_empty() {
                    return Err(EvaluatorError::EvalError(
                        "Not enough operands for import".to_string(),
                    ));
                }
                
                // The module name to import should be at the top of the stack
                let module_name = match self.stack.pop().unwrap() {
                    Value::String(s) => s,
                    Value::Symbol(ref s) => s.clone(),
                    _ => {
                        return Err(EvaluatorError::TypeError(
                            "Module name must be a string or symbol".to_string(),
                        ));
                    }
                };
                
                // Try to load the module from prelude
                let module_path = self.prelude_path.join(format!("{}/{}.borf", module_name, module_name));
                
                if module_path.exists() {
                    // Read the module file
                    let content = fs::read_to_string(&module_path)
                        .map_err(|e| EvaluatorError::FileError(e))?;
                    
                    // Parse and evaluate the module
                    let mut parser = Parser::new(&content);
                    let expr = parser.parse()?;
                    
                    // Evaluate the module
                    self.eval_expr(&expr)?;
                    
                    println!("Imported module: {}", module_name);
                    Ok(())
                } else {
                    // Module not found
                    println!("Module not found: {}", module_name);
                    Err(EvaluatorError::EvalError(format!(
                        "Module '{}' not found (looked in {})",
                        module_name,
                        module_path.display()
                    )))
                }
            },
            "import_module" => {
                // Import a module into an environment
                // Syntax: name env import_module -> env
                if self.stack.len() < 2 {
                    return Err(EvaluatorError::EvalError(
                        "Not enough operands for import_module (name, env)".to_string(),
                    ));
                }
                
                let env_val = self.stack.pop().unwrap();
                let module_name = self.stack.pop().unwrap();
                
                // Get module name as string
                let name_str = match module_name {
                    Value::String(s) => s,
                    Value::Symbol(ref s) => s.clone(),
                    _ => {
                        return Err(EvaluatorError::EvalError(
                            "Module name must be a string or symbol".to_string(),
                        ));
                    }
                };
                
                // Check built-in modules first
                if name_str == "core" {
                    // Core module with basic operations
                    match env_val {
                        Value::Map(mut env_map) => {
                            // Get or create bindings
                            let bindings = match env_map.get_mut("bindings") {
                                Some(Value::Map(bindings)) => bindings,
                                _ => {
                                    return Err(EvaluatorError::EvalError(
                                        "Invalid environment structure".to_string(),
                                    ));
                                }
                            };
                            
                            // Add core operations
                            bindings.insert("add".to_string(), Value::Symbol("add".to_string()));
                            bindings.insert("sub".to_string(), Value::Symbol("sub".to_string()));
                            bindings.insert("mul".to_string(), Value::Symbol("mul".to_string()));
                            bindings.insert("div".to_string(), Value::Symbol("div".to_string()));
                            bindings.insert("print".to_string(), Value::Symbol("print".to_string()));
                            bindings.insert("println".to_string(), Value::Symbol("println".to_string()));
                            
                            // Add boolean values
                            bindings.insert("true".to_string(), Value::Number(1));
                            bindings.insert("false".to_string(), Value::Number(0));
                            
                            // Return the updated environment
                            self.stack.push(Value::Map(env_map));
                            Ok(())
                        },
                        _ => Err(EvaluatorError::EvalError(
                            "Environment must be a map".to_string(),
                        )),
                    }
                } else if name_str == "meta" {
                    // Meta module with eval operations
                    match env_val {
                        Value::Map(mut env_map) => {
                            // Get or create bindings
                            let bindings = match env_map.get_mut("bindings") {
                                Some(Value::Map(bindings)) => bindings,
                                _ => {
                                    return Err(EvaluatorError::EvalError(
                                        "Invalid environment structure".to_string(),
                                    ));
                                }
                            };
                            
                            // Add meta operations
                            bindings.insert("parse".to_string(), Value::Symbol("parse".to_string()));
                            bindings.insert("evaluate".to_string(), Value::Symbol("evaluate".to_string()));
                            bindings.insert("quote".to_string(), Value::Symbol("'".to_string()));
                            bindings.insert("unquote".to_string(), Value::Symbol("unquote".to_string()));
                            
                            // Return the updated environment
                            self.stack.push(Value::Map(env_map));
                            Ok(())
                        },
                        _ => Err(EvaluatorError::EvalError(
                            "Environment must be a map".to_string(),
                        )),
                    }
                } else if name_str == "prim" {
                    // Primitive operations
                    match env_val {
                        Value::Map(mut env_map) => {
                            // Get or create bindings
                            let bindings = match env_map.get_mut("bindings") {
                                Some(Value::Map(bindings)) => bindings,
                                _ => {
                                    return Err(EvaluatorError::EvalError(
                                        "Invalid environment structure".to_string(),
                                    ));
                                }
                            };
                            
                            // Add primitive operations
                            bindings.insert("to_string".to_string(), Value::Symbol("value_to_string".to_string()));
                            bindings.insert("length".to_string(), Value::Symbol("length".to_string()));
                            bindings.insert("map".to_string(), Value::Symbol("map".to_string()));
                            bindings.insert("list".to_string(), Value::Symbol("list".to_string()));
                            bindings.insert("for_each".to_string(), Value::Symbol("for".to_string()));
                            
                            // Return the updated environment
                            self.stack.push(Value::Map(env_map));
                            Ok(())
                        },
                        _ => Err(EvaluatorError::EvalError(
                            "Environment must be a map".to_string(),
                        )),
                    }
                } else {
                    // Try to look up a user-defined module in the interpreter's environment
                    if let Some(module) = self.env.get(&name_str) {
                        match &module {
                            Value::Module(mod_name, exports) => {
                                // We found a module, import its exports
                                match env_val {
                                    Value::Map(mut env_map) => {
                                        // Get or create bindings
                                        let bindings = match env_map.get_mut("bindings") {
                                            Some(Value::Map(bindings)) => bindings,
                                            _ => {
                                                return Err(EvaluatorError::EvalError(
                                                    "Invalid environment structure".to_string(),
                                                ));
                                            }
                                        };
                                        
                                        // Add the module itself
                                        bindings.insert(name_str.clone(), module.clone());
                                        
                                        // Add all exports
                                        for (export_name, export_value) in exports {
                                            bindings.insert(export_name.clone(), export_value.clone());
                                        }
                                        
                                        // Return the updated environment
                                        self.stack.push(Value::Map(env_map));
                                        Ok(())
                                    },
                                    _ => Err(EvaluatorError::EvalError(
                                        "Environment must be a map".to_string(),
                                    )),
                                }
                            },
                            Value::Map(module_map) => {
                                // We found a module-like map, import its contents
                                match env_val {
                                    Value::Map(mut env_map) => {
                                        // Get or create bindings
                                        let bindings = match env_map.get_mut("bindings") {
                                            Some(Value::Map(bindings)) => bindings,
                                            _ => {
                                                return Err(EvaluatorError::EvalError(
                                                    "Invalid environment structure".to_string(),
                                                ));
                                            }
                                        };
                                        
                                        // Add the module itself
                                        bindings.insert(name_str.clone(), module.clone());
                                        
                                        // Add exports if available
                                        if let Some(Value::Map(exports)) = module_map.get("exports") {
                                            for (export_name, export_value) in exports {
                                                bindings.insert(export_name.clone(), export_value.clone());
                                            }
                                        }
                                        
                                        // Return the updated environment
                                        self.stack.push(Value::Map(env_map));
                                        Ok(())
                                    },
                                    _ => Err(EvaluatorError::EvalError(
                                        "Environment must be a map".to_string(),
                                    )),
                                }
                            },
                            _ => Err(EvaluatorError::EvalError(format!(
                                "Value for {} is not a module",
                                name_str
                            ))),
                        }
                    } else {
                        // Module not found
                        Err(EvaluatorError::EvalError(format!(
                            "Module '{}' not found in environment",
                            name_str
                        )))
                    }
                }
            }
            "println" => {
                // Print a value followed by a newline
                if self.stack.is_empty() {
                    return Err(EvaluatorError::EvalError(
                        "Not enough operands for println".to_string(),
                    ));
                }
                
                // Get the value
                let value = self.stack.pop().unwrap();
                
                // Print the value
                match value {
                    Value::String(s) => println!("{}", s),
                    _ => println!("{}", self.value_to_string(&value)),
                }
                
                Ok(())
            },
            "print" => {
                // Print a value without a newline
                if self.stack.is_empty() {
                    return Err(EvaluatorError::EvalError(
                        "Not enough operands for print".to_string(),
                    ));
                }
                
                // Get the value
                let value = self.stack.pop().unwrap();
                
                // Print the value
                match value {
                    Value::String(s) => print!("{}", s),
                    _ => print!("{}", self.value_to_string(&value)),
                }
                
                // Flush stdout to ensure the output is displayed
                use std::io::Write;
                std::io::stdout().flush().map_err(|e| 
                    EvaluatorError::EvalError(format!("Error flushing stdout: {}", e)))?;
                
                Ok(())
            },
            "break" => {
                // This is a placeholder for break in loops
                // In a real implementation, this would be handled by the loop construct
                println!("Break called - would exit the loop here");
                Ok(())
            },
            "new_env" => {
                // Create a new environment
                println!("Creating a new environment for metacircular evaluation");
                
                // Create a new environment with standard bindings
                let mut env = Env::new();
                
                // Add basic operations to the environment
                env.set("add", Value::Symbol("add".to_string()));
                env.set("sub", Value::Symbol("sub".to_string()));
                env.set("mul", Value::Symbol("mul".to_string()));
                env.set("div", Value::Symbol("div".to_string()));
                env.set("print", Value::Symbol("print".to_string()));
                env.set("println", Value::Symbol("println".to_string()));
                
                // Add boolean values
                env.set("true", Value::Number(1));
                env.set("false", Value::Number(0));
                
                // Push environment to the stack, wrapped appropriately for the metacircular evaluator
                // We use a map to represent the environment in a way the metacircular evaluator can understand
                let mut env_map = HashMap::new();
                env_map.insert("bindings".to_string(), Value::Map(env.bindings.clone()));
                env_map.insert("parent".to_string(), Value::Nothing);
                
                self.stack.push(Value::Map(env_map));
                Ok(())
            },
            "env_lookup" => {
                // Look up a value in an environment
                // Syntax: env name env_lookup -> value
                if self.stack.len() < 2 {
                    return Err(EvaluatorError::EvalError(
                        "Not enough operands for env_lookup (env, name)".to_string(),
                    ));
                }
                
                let name = self.stack.pop().unwrap();
                let env_val = self.stack.pop().unwrap();
                
                // Get name as string
                let name_str = match &name {
                    Value::String(s) => s.clone(),
                    Value::Symbol(s) => s.clone(),
                    _ => {
                        return Err(EvaluatorError::EvalError(
                            "Variable name must be a string or symbol".to_string(),
                        ));
                    }
                };
                
                // Look up in the environment
                match env_val {
                    Value::Map(env_map) => {
                        // Check if we have bindings
                        if let Some(Value::Map(bindings)) = env_map.get("bindings") {
                            // Check if the binding exists
                            if let Some(value) = bindings.get(&name_str) {
                                self.stack.push(value.clone());
                                return Ok(());
                            }
                            
                            // Check the parent environment if available
                            if let Some(parent) = env_map.get("parent") {
                                if !matches!(parent, Value::Nothing) {
                                    // Push parent and name back on stack and recurse
                                    self.stack.push(parent.clone());
                                    self.stack.push(name);
                                    return self.execute_operation("env_lookup");
                                }
                            }
                            
                            // Not found, push Nothing
                            self.stack.push(Value::Nothing);
                            Ok(())
                        } else {
                            Err(EvaluatorError::EvalError(
                                "Invalid environment structure (missing bindings)".to_string(),
                            ))
                        }
                    },
                    _ => Err(EvaluatorError::EvalError(
                        "First argument to env_lookup must be an environment".to_string(),
                    )),
                }
            },
            "env_set" => {
                // Set a value in an environment
                // Syntax: env name value env_set -> env
                if self.stack.len() < 3 {
                    return Err(EvaluatorError::EvalError(
                        "Not enough operands for env_set (env, name, value)".to_string(),
                    ));
                }
                
                let value = self.stack.pop().unwrap();
                let name = self.stack.pop().unwrap();
                let env_val = self.stack.pop().unwrap();
                
                // Get name as string
                let name_str = match &name {
                    Value::String(s) => s.clone(),
                    Value::Symbol(s) => s.clone(),
                    _ => {
                        return Err(EvaluatorError::EvalError(
                            "Variable name must be a string or symbol".to_string(),
                        ));
                    }
                };
                
                // Update the environment
                match env_val {
                    Value::Map(mut env_map) => {
                        // Get or create bindings
                        let bindings = match env_map.get_mut("bindings") {
                            Some(Value::Map(bindings)) => bindings,
                            _ => {
                                // Create new bindings map
                                let new_bindings = HashMap::new();
                                env_map.insert("bindings".to_string(), Value::Map(new_bindings));
                                
                                // Get the newly created map
                                match env_map.get_mut("bindings") {
                                    Some(Value::Map(bindings)) => bindings,
                                    _ => {
                                        return Err(EvaluatorError::EvalError(
                                            "Failed to create bindings in environment".to_string(),
                                        ));
                                    }
                                }
                            }
                        };
                        
                        // Set the binding
                        bindings.insert(name_str, value);
                        
                        // Return the updated environment
                        self.stack.push(Value::Map(env_map));
                        Ok(())
                    },
                    _ => Err(EvaluatorError::EvalError(
                        "First argument to env_set must be an environment".to_string(),
                    )),
                }
            },
            "define" => {
                // Define a value in the environment
                // Used for module object registration
                // Syntax: env name value define -> env
                
                // This is just an alias for env_set
                self.execute_operation("env_set")
            },
            "while" => {
                // While expression in concatenative style:
                // [condition] [body] while -> result
                // This returns the final value produced by the body
                
                if self.stack.len() < 2 {
                    return Err(EvaluatorError::EvalError(
                        "Not enough operands for while expression ([condition], [body])".to_string(),
                    ));
                }
                
                let body = self.stack.pop().unwrap();
                let condition = self.stack.pop().unwrap();
                
                // Make sure we have quotations or something callable
                if !matches!(body, Value::Quotation(..) | Value::Symbol(..) | Value::TypedQuotation(..)) ||
                   !matches!(condition, Value::Quotation(..) | Value::Symbol(..) | Value::TypedQuotation(..)) {
                    return Err(EvaluatorError::EvalError(
                        "While expression requires condition and body quotations".to_string()
                    ));
                }
                
                // For now, just implement a placeholder version
                // In a full implementation, we would actually evaluate the condition and body
                println!("While expression used with [condition] and [body]");
                println!("In a real implementation, this would evaluate condition repeatedly");
                println!("and execute the body as long as the condition is true");
                
                // Return the last value from the body as the result of the loop
                match body {
                    // If the body is a symbol, push it as the result
                    Value::Symbol(s) => self.stack.push(Value::Symbol(s)),
                    // For quotations, return a placeholder result
                    _ => self.stack.push(Value::Nothing)
                }
                
                Ok(())
            },
            "if" => {
                // Conditional expression for the concatenative style:
                // condition then-value else-value if -> result
                
                if self.stack.len() < 3 {
                    return Err(EvaluatorError::EvalError(
                        "Not enough operands for if expression (condition, then-value, else-value)".to_string(),
                    ));
                }
                
                let else_value = self.stack.pop().unwrap();
                let then_value = self.stack.pop().unwrap();
                let condition = self.stack.pop().unwrap();
                
                // Check if condition is truthy
                let is_truthy = match condition {
                    Value::Number(n) => n != 0,
                    Value::String(s) => !s.is_empty(),
                    Value::Symbol(s) if s == "true" => true,
                    Value::Symbol(s) if s == "false" => false,
                    _ => !matches!(condition, Value::Nothing | Value::Nil),
                };
                
                // Push the selected value as result of the expression
                if is_truthy {
                    self.stack.push(then_value);
                } else {
                    self.stack.push(else_value);
                }
                
                Ok(())
            },
            "then" => {
                // This is a placeholder for the then keyword in if statements
                println!("Then clause would execute here");
                Ok(())
            },
            "handle" => {
                // A handler function for explicit error handling
                // pattern: [computation] [handler] handle -> result_or_error_result
                if self.stack.len() < 2 {
                    return Err(EvaluatorError::EvalError(
                        "Not enough operands for handle expression ([computation], [handler])".to_string(),
                    ));
                }
                
                let handler = self.stack.pop().unwrap();
                let computation = self.stack.pop().unwrap();
                
                // Make sure we have quotations or callable values
                if !matches!(computation, Value::Quotation(..) | Value::Symbol(..) | Value::TypedQuotation(..)) ||
                   !matches!(handler, Value::Quotation(..) | Value::Symbol(..) | Value::TypedQuotation(..)) {
                    return Err(EvaluatorError::EvalError(
                        "Handle expression requires computation and handler quotations".to_string()
                    ));
                }
                
                // In a full implementation, we would:
                // 1. Evaluate the computation to get a Result<Value, Error>
                // 2. If Ok(value), return the value
                // 3. If Err(error), apply the handler to the error value
                
                println!("Handle expression with [computation] and [handler]");
                println!("This uses explicit error handling instead of stack unwinding");
                
                // For now, just return a placeholder successful result
                match computation {
                    Value::Symbol(s) => self.stack.push(Value::Symbol(s)),
                    _ => self.stack.push(Value::Number(42))
                }
                
                Ok(())
            },
            "ok" => {
                // Wrap a value in a Result::Ok
                // Syntax: value ok -> Result
                if self.stack.is_empty() {
                    return Err(EvaluatorError::EvalError(
                        "Not enough operands for ok (value)".to_string(),
                    ));
                }
                
                let value = self.stack.pop().unwrap();
                
                // Create a map representing an Ok result
                let mut result_map = HashMap::new();
                result_map.insert("type".to_string(), Value::Symbol("Ok".to_string()));
                result_map.insert("value".to_string(), value);
                
                self.stack.push(Value::Map(result_map));
                Ok(())
            },
            "error" => {
                // Create an error result
                // Syntax: message error -> Result
                if self.stack.is_empty() {
                    return Err(EvaluatorError::EvalError(
                        "Not enough operands for error (message)".to_string(),
                    ));
                }
                
                let message = self.stack.pop().unwrap();
                
                // Create a map representing an Err result
                let mut result_map = HashMap::new();
                result_map.insert("type".to_string(), Value::Symbol("Error".to_string()));
                result_map.insert("message".to_string(), message);
                
                self.stack.push(Value::Map(result_map));
                Ok(())
            },
            "is_ok" => {
                // Check if a result is Ok
                // Syntax: result is_ok -> boolean
                if self.stack.is_empty() {
                    return Err(EvaluatorError::EvalError(
                        "Not enough operands for is_ok (result)".to_string(),
                    ));
                }
                
                let result = self.stack.pop().unwrap();
                
                // Check if it's an Ok result
                let is_ok = match result {
                    Value::Map(map) => {
                        if let Some(Value::Symbol(ty)) = map.get("type") {
                            ty == "Ok"
                        } else {
                            false
                        }
                    },
                    _ => false
                };
                
                self.stack.push(Value::Number(if is_ok { 1 } else { 0 }));
                Ok(())
            },
            "is_error" => {
                // Check if a result is Error
                // Syntax: result is_error -> boolean
                if self.stack.is_empty() {
                    return Err(EvaluatorError::EvalError(
                        "Not enough operands for is_error (result)".to_string(),
                    ));
                }
                
                let result = self.stack.pop().unwrap();
                
                // Check if it's an Error result
                let is_error = match result {
                    Value::Map(map) => {
                        if let Some(Value::Symbol(ty)) = map.get("type") {
                            ty == "Error"
                        } else {
                            false
                        }
                    },
                    _ => false
                };
                
                self.stack.push(Value::Number(if is_error { 1 } else { 0 }));
                Ok(())
            },
            // Functional logic programming primitives
            "fallible" => {
                // Mark a computation as fallible - operations within may fail
                // Syntax: [computation] fallible -> context
                if self.stack.is_empty() {
                    return Err(EvaluatorError::EvalError(
                        "Not enough operands for fallible ([computation])".to_string(),
                    ));
                }
                
                let computation = self.stack.pop().unwrap();
                
                // Create a fallible context wrapper
                let mut context = HashMap::new();
                context.insert("type".to_string(), Value::Symbol("context".to_string()));
                context.insert("mode".to_string(), Value::Symbol("fallible".to_string()));
                context.insert("computation".to_string(), computation);
                
                self.stack.push(Value::Map(context));
                Ok(())
            },
            "infallible" => {
                // Mark a computation as infallible - operations within must succeed
                // Syntax: [computation] infallible -> context
                if self.stack.is_empty() {
                    return Err(EvaluatorError::EvalError(
                        "Not enough operands for infallible ([computation])".to_string(),
                    ));
                }
                
                let computation = self.stack.pop().unwrap();
                
                // Create an infallible context wrapper
                let mut context = HashMap::new();
                context.insert("type".to_string(), Value::Symbol("context".to_string()));
                context.insert("mode".to_string(), Value::Symbol("infallible".to_string()));
                context.insert("computation".to_string(), computation);
                
                self.stack.push(Value::Map(context));
                Ok(())
            },
            "narrow" => {
                // Narrow a value to ensure it satisfies a predicate
                // Syntax: value predicate narrow -> narrowing
                if self.stack.len() < 2 {
                    return Err(EvaluatorError::EvalError(
                        "Not enough operands for narrow (value, predicate)".to_string(),
                    ));
                }
                
                let predicate = self.stack.pop().unwrap();
                let value = self.stack.pop().unwrap();
                
                // For now, we'll just create a narrowing result
                // In a full implementation, this would involve backtracking
                let mut narrowing = HashMap::new();
                narrowing.insert("type".to_string(), Value::Symbol("narrowing".to_string()));
                narrowing.insert("value".to_string(), value);
                narrowing.insert("predicate".to_string(), predicate);
                narrowing.insert("satisfied".to_string(), Value::Number(1)); // Assume satisfied for now
                
                self.stack.push(Value::Map(narrowing));
                Ok(())
            },
            "choose" => {
                // Non-deterministically choose between options
                // Syntax: options choose -> choice
                if self.stack.is_empty() {
                    return Err(EvaluatorError::EvalError(
                        "Not enough operands for choose (options)".to_string(),
                    ));
                }
                
                let options = self.stack.pop().unwrap();
                
                // Make sure options is a list
                match options {
                    Value::List(items) => {
                        if items.is_empty() {
                            return Err(EvaluatorError::EvalError(
                                "Cannot choose from empty list".to_string(),
                            ));
                        }
                        
                        // For now, just deterministically choose the first option
                        // In a full implementation, this would create a choice point
                        let choice = items[0].clone();
                        
                        // Create a choice wrapper for debugging
                        let mut choice_map = HashMap::new();
                        choice_map.insert("type".to_string(), Value::Symbol("choice".to_string()));
                        choice_map.insert("options".to_string(), Value::List(items));
                        choice_map.insert("selected".to_string(), choice.clone());
                        
                        // Push the actual choice value, not the wrapper
                        self.stack.push(choice);
                        Ok(())
                    },
                    _ => Err(EvaluatorError::EvalError(
                        "Choose requires a list of options".to_string(),
                    )),
                }
            },
            "eventually" => {
                // Ensure a computation succeeds, exploring alternatives if needed
                // Syntax: [computation] eventually -> result
                if self.stack.is_empty() {
                    return Err(EvaluatorError::EvalError(
                        "Not enough operands for eventually ([computation])".to_string(),
                    ));
                }
                
                let computation = self.stack.pop().unwrap();
                
                // Create an eventually wrapper
                let mut eventually = HashMap::new();
                eventually.insert("type".to_string(), Value::Symbol("eventually".to_string()));
                eventually.insert("computation".to_string(), computation);
                
                // For now, just push the wrapper
                // In a full implementation, this would trigger backtracking search
                self.stack.push(Value::Map(eventually));
                Ok(())
            },
            "var" => {
                // Create a new logic variable
                // Syntax: name var -> variable
                if self.stack.is_empty() {
                    return Err(EvaluatorError::EvalError(
                        "Not enough operands for var (name)".to_string(),
                    ));
                }
                
                let name = self.stack.pop().unwrap();
                
                // Convert name to string
                let name_str = match name {
                    Value::String(s) => s,
                    Value::Symbol(s) => s,
                    _ => {
                        return Err(EvaluatorError::EvalError(
                            "Variable name must be a string or symbol".to_string(),
                        ));
                    }
                };
                
                // Create a variable
                let mut var = HashMap::new();
                var.insert("type".to_string(), Value::Symbol("var".to_string()));
                var.insert("name".to_string(), Value::String(name_str));
                var.insert("bound".to_string(), Value::Number(0)); // false
                var.insert("value".to_string(), Value::Nothing);
                
                self.stack.push(Value::Map(var));
                Ok(())
            },
            "bind" => {
                // Bind a value to a logic variable
                // Syntax: variable value bind -> bound_variable
                if self.stack.len() < 2 {
                    return Err(EvaluatorError::EvalError(
                        "Not enough operands for bind (variable, value)".to_string(),
                    ));
                }
                
                let value = self.stack.pop().unwrap();
                let variable = self.stack.pop().unwrap();
                
                // Make sure we have a variable
                match variable {
                    Value::Map(mut var) => {
                        if let Some(Value::Symbol(typ)) = var.get("type") {
                            if typ == "var" {
                                // Update the variable binding
                                var.insert("bound".to_string(), Value::Number(1)); // true
                                var.insert("value".to_string(), value);
                                
                                self.stack.push(Value::Map(var));
                                Ok(())
                            } else {
                                Err(EvaluatorError::EvalError(
                                    "First argument to bind must be a variable".to_string(),
                                ))
                            }
                        } else {
                            Err(EvaluatorError::EvalError(
                                "First argument to bind must be a variable".to_string(),
                            ))
                        }
                    },
                    _ => Err(EvaluatorError::EvalError(
                        "First argument to bind must be a variable".to_string(),
                    )),
                }
            },
            "and" => {
                // Logical AND operation
                if self.stack.len() < 2 {
                    return Err(EvaluatorError::EvalError(
                        "Not enough operands for and operation".to_string(),
                    ));
                }
                
                let b = self.stack.pop().unwrap();
                let a = self.stack.pop().unwrap();
                
                // Check if both values are truthy
                let a_truthy = match a {
                    Value::Number(n) => n != 0,
                    Value::String(s) => !s.is_empty(),
                    Value::Symbol(s) if s == "true" => true,
                    Value::Symbol(s) if s == "false" => false,
                    _ => !matches!(a, Value::Nothing | Value::Nil),
                };
                
                let b_truthy = match b {
                    Value::Number(n) => n != 0,
                    Value::String(s) => !s.is_empty(),
                    Value::Symbol(s) if s == "true" => true,
                    Value::Symbol(s) if s == "false" => false,
                    _ => !matches!(b, Value::Nothing | Value::Nil),
                };
                
                // Push the result as a number (1 for true, 0 for false)
                if a_truthy && b_truthy {
                    self.stack.push(Value::Number(1));
                } else {
                    self.stack.push(Value::Number(0));
                }
                
                Ok(())
            },
            "==" => {
                // Value equivalence - compares values for exact match
                if self.stack.len() < 2 {
                    return Err(EvaluatorError::EvalError(
                        "Not enough operands for == (value equivalence)".to_string(),
                    ));
                }
                
                let b = self.stack.pop().unwrap();
                let a = self.stack.pop().unwrap();
                
                // Compare for value equivalence
                let equal = a == b;
                
                // Push the result as a number (1 for true, 0 for false)
                if equal {
                    self.stack.push(Value::Number(1));
                } else {
                    self.stack.push(Value::Number(0));
                }
                
                Ok(())
            },
            "!=" => {
                // Value not equal
                if self.stack.len() < 2 {
                    return Err(EvaluatorError::EvalError(
                        "Not enough operands for != (inequality)".to_string(),
                    ));
                }
                
                let b = self.stack.pop().unwrap();
                let a = self.stack.pop().unwrap();
                
                // Compare for inequality
                let not_equal = a != b;
                
                // Push the result as a number (1 for true, 0 for false)
                if not_equal {
                    self.stack.push(Value::Number(1));
                } else {
                    self.stack.push(Value::Number(0));
                }
                
                Ok(())
            },
            ">" => {
                // Greater than
                if self.stack.len() < 2 {
                    return Err(EvaluatorError::EvalError(
                        "Not enough operands for > (greater than)".to_string(),
                    ));
                }
                
                let b = self.stack.pop().unwrap();
                let a = self.stack.pop().unwrap();
                
                // Compare based on type
                match (a, b) {
                    (Value::Number(a), Value::Number(b)) => {
                        self.stack.push(Value::Number(if a > b { 1 } else { 0 }));
                    },
                    (Value::String(a), Value::String(b)) => {
                        self.stack.push(Value::Number(if a > b { 1 } else { 0 }));
                    },
                    _ => {
                        return Err(EvaluatorError::EvalError(
                            "Cannot compare values of different types with >".to_string(),
                        ));
                    }
                }
                
                Ok(())
            },
            "<" => {
                // Less than
                if self.stack.len() < 2 {
                    return Err(EvaluatorError::EvalError(
                        "Not enough operands for < (less than)".to_string(),
                    ));
                }
                
                let b = self.stack.pop().unwrap();
                let a = self.stack.pop().unwrap();
                
                // Compare based on type
                match (a, b) {
                    (Value::Number(a), Value::Number(b)) => {
                        self.stack.push(Value::Number(if a < b { 1 } else { 0 }));
                    },
                    (Value::String(a), Value::String(b)) => {
                        self.stack.push(Value::Number(if a < b { 1 } else { 0 }));
                    },
                    _ => {
                        return Err(EvaluatorError::EvalError(
                            "Cannot compare values of different types with <".to_string(),
                        ));
                    }
                }
                
                Ok(())
            },
            ">=" => {
                // Greater than or equal to
                if self.stack.len() < 2 {
                    return Err(EvaluatorError::EvalError(
                        "Not enough operands for >= (greater than or equal)".to_string(),
                    ));
                }
                
                let b = self.stack.pop().unwrap();
                let a = self.stack.pop().unwrap();
                
                // Compare based on type
                match (a, b) {
                    (Value::Number(a), Value::Number(b)) => {
                        self.stack.push(Value::Number(if a >= b { 1 } else { 0 }));
                    },
                    (Value::String(a), Value::String(b)) => {
                        self.stack.push(Value::Number(if a >= b { 1 } else { 0 }));
                    },
                    _ => {
                        return Err(EvaluatorError::EvalError(
                            "Cannot compare values of different types with >=".to_string(),
                        ));
                    }
                }
                
                Ok(())
            },
            "<=" => {
                // Less than or equal to
                if self.stack.len() < 2 {
                    return Err(EvaluatorError::EvalError(
                        "Not enough operands for <= (less than or equal)".to_string(),
                    ));
                }
                
                let b = self.stack.pop().unwrap();
                let a = self.stack.pop().unwrap();
                
                // Compare based on type
                match (a, b) {
                    (Value::Number(a), Value::Number(b)) => {
                        self.stack.push(Value::Number(if a <= b { 1 } else { 0 }));
                    },
                    (Value::String(a), Value::String(b)) => {
                        self.stack.push(Value::Number(if a <= b { 1 } else { 0 }));
                    },
                    _ => {
                        return Err(EvaluatorError::EvalError(
                            "Cannot compare values of different types with <=".to_string(),
                        ));
                    }
                }
                
                Ok(())
            },
            "is_some" => {
                // Check if a value is Some
                if self.stack.is_empty() {
                    return Err(EvaluatorError::EvalError(
                        "Not enough operands for is_some".to_string(),
                    ));
                }
                
                let value = self.stack.pop().unwrap();
                
                // Check if the value is Some
                let is_some = match value {
                    Value::Optional(Some(_)) => true,
                    _ => false,
                };
                
                self.stack.push(Value::Number(if is_some { 1 } else { 0 }));
                Ok(())
            },
            "is_nothing" => {
                // Check if a value is Nothing
                if self.stack.is_empty() {
                    return Err(EvaluatorError::EvalError(
                        "Not enough operands for is_nothing".to_string(),
                    ));
                }
                
                let value = self.stack.pop().unwrap();
                
                // Check if the value is Nothing
                let is_nothing = matches!(value, Value::Nothing | Value::Optional(None));
                
                self.stack.push(Value::Number(if is_nothing { 1 } else { 0 }));
                Ok(())
            },
            "unwrap" => {
                // Unwrap an optional value
                if self.stack.is_empty() {
                    return Err(EvaluatorError::EvalError(
                        "Not enough operands for unwrap".to_string(),
                    ));
                }
                
                let value = self.stack.pop().unwrap();
                
                // Unwrap the optional
                match value {
                    Value::Optional(Some(inner)) => {
                        self.stack.push(*inner);
                        Ok(())
                    },
                    Value::Optional(None) => {
                        Err(EvaluatorError::EvalError("Cannot unwrap Nothing".to_string()))
                    },
                    _ => {
                        // Value is already unwrapped, return as is
                        self.stack.push(value);
                        Ok(())
                    }
                }
            },
            "apply" => {
                // Apply a function to arguments
                // Simplified for now, just executes the quotation with arguments on stack
                if self.stack.is_empty() {
                    return Err(EvaluatorError::EvalError(
                        "Not enough operands for apply".to_string(),
                    ));
                }
                
                let callable = self.stack.pop().unwrap();
                
                match callable {
                    Value::Quotation(params, body, closure_env) => {
                        // Execute the quotation
                        // In a full implementation, we would create a new environment with bindings
                        // For now, just execute each expression in the body
                        for expr in body {
                            self.eval_expr(&expr)?;
                        }
                        Ok(())
                    },
                    Value::TypedQuotation(params, body, return_type, closure_env) => {
                        // Execute the typed quotation
                        // Similar to regular quotation but with return type check
                        for expr in body {
                            self.eval_expr(&expr)?;
                        }
                        Ok(())
                    },
                    Value::Symbol(op) => {
                        // Execute the operation
                        self.execute_operation(&op)
                    },
                    _ => {
                        // Not callable, just push back
                        self.stack.push(callable);
                        Ok(())
                    }
                }
            },
            "some" => {
                // Wrap a value in Some
                if self.stack.is_empty() {
                    return Err(EvaluatorError::EvalError(
                        "Not enough operands for some".to_string(),
                    ));
                }
                
                let value = self.stack.pop().unwrap();
                
                // Wrap in Some
                self.stack.push(Value::Optional(Some(Box::new(value))));
                Ok(())
            },
            "nothing" => {
                // Push a Nothing value
                self.stack.push(Value::Nothing);
                Ok(())
            },
            "when" => {
                // Pattern matching primitive
                // value pattern_fn default_value when -> result
                if self.stack.len() < 3 {
                    return Err(EvaluatorError::EvalError(
                        "Not enough operands for when (value, pattern_fn, default_value)".to_string(),
                    ));
                }
                
                let default_value = self.stack.pop().unwrap();
                let pattern_fn = self.stack.pop().unwrap();
                let value = self.stack.pop().unwrap();
                
                // Simplified pattern matching
                // If condition is true, execute the function with the value
                // Otherwise, return the default value
                let is_match = match &value {
                    Value::Nothing | Value::Optional(None) => false,
                    Value::Number(n) => *n != 0,
                    Value::String(s) => !s.is_empty(),
                    Value::Symbol(s) if s == "false" => false,
                    Value::Symbol(s) if s == "true" => true,
                    Value::List(items) => !items.is_empty(),
                    _ => true,
                };
                
                if is_match {
                    // Apply the pattern function to the value
                    self.stack.push(value);
                    
                    match pattern_fn {
                        Value::Symbol(op) => {
                            self.execute_operation(&op)?;
                            Ok(())
                        },
                        Value::Quotation(_, _, _) => {
                            // Push the quotation back and apply it
                            self.stack.push(pattern_fn);
                            self.execute_operation("apply")?;
                            Ok(())
                        },
                        _ => {
                            // Just return the pattern function as the result
                            self.stack.push(pattern_fn);
                            Ok(())
                        }
                    }
                } else {
                    // Return the default value
                    self.stack.push(default_value);
                    Ok(())
                }
            },
            "otherwise" => {
                // Simple continuation keyword for when-otherwise pattern
                // Just a no-op that returns its input
                if self.stack.is_empty() {
                    return Err(EvaluatorError::EvalError(
                        "Not enough operands for otherwise".to_string(),
                    ));
                }
                
                // Just return the value unchanged
                let value = self.stack.pop().unwrap();
                self.stack.push(value);
                Ok(())
            },
            "veq" => {
                // Value equality check (similar to ===)
                if self.stack.len() < 2 {
                    return Err(EvaluatorError::EvalError(
                        "Not enough operands for veq (value equality)".to_string(),
                    ));
                }
                
                let b = self.stack.pop().unwrap();
                let a = self.stack.pop().unwrap();
                
                // Check structural equality
                let equal = match (&a, &b) {
                    (Value::Number(a), Value::Number(b)) => a == b,
                    (Value::String(a), Value::String(b)) => a == b,
                    (Value::Symbol(a), Value::Symbol(b)) => a == b,
                    (Value::List(a), Value::List(b)) => a == b,
                    (Value::Map(a), Value::Map(b)) => a == b,
                    (Value::Nothing, Value::Nothing) => true,
                    _ => a == b,
                };
                
                self.stack.push(Value::Number(if equal { 1 } else { 0 }));
                Ok(())
            },
            "===" => {
                // Context-dependent equivalence (type/structural/categorical)
                if self.stack.len() < 2 {
                    return Err(EvaluatorError::EvalError(
                        "Not enough operands for === (equivalence operation)".to_string(),
                    ));
                }
                
                let b = self.stack.pop().unwrap();
                let a = self.stack.pop().unwrap();
                
                // Determine the type of equivalence based on operand types
                let equivalent = match (&a, &b) {
                    // Type equivalence - when comparing types directly
                    (Value::Type(_), Value::Type(_)) => a == b,
                    (Value::QuotedType(_), Value::QuotedType(_)) => a == b,
                    
                    // Same basic types - check only type equivalence
                    (Value::Number(_), Value::Number(_)) => true,
                    (Value::String(_), Value::String(_)) => true,
                    (Value::Symbol(_), Value::Symbol(_)) => true,
                    
                    // Functions - check signature equivalence
                    (Value::Quotation(params1, _, _), Value::Quotation(params2, _, _)) => 
                        params1.len() == params2.len(),
                    
                    // Typed quotations - check type signature equivalence
                    (Value::TypedQuotation(params1, _, type1, _), Value::TypedQuotation(params2, _, type2, _)) => 
                        params1.len() == params2.len() && type1 == type2,
                    
                    // Collections - check structural equivalence
                    (Value::List(items1), Value::List(items2)) => {
                        if items1.len() != items2.len() {
                            false
                        } else {
                            // For lists, check if all elements are equivalent
                            items1.iter().zip(items2.iter()).all(|(a, b)| a == b)
                        }
                    },
                    
                    // Maps - check key equivalence
                    (Value::Map(map1), Value::Map(map2)) => {
                        if map1.len() != map2.len() {
                            false
                        } else {
                            // For maps, check if keys match and values are equivalent
                            map1.keys().all(|k| map2.contains_key(k) && map1[k] == map2[k])
                        }
                    },
                    
                    // Different types are never equivalent
                    _ => false,
                };
                
                // Push the result
                if equivalent {
                    self.stack.push(Value::Number(1));
                } else {
                    self.stack.push(Value::Number(0));
                }
                
                Ok(())
            },
            "'" => {
                // Quote operation - captures the next expression as a quoted value
                if self.stack.is_empty() {
                    return Err(EvaluatorError::EvalError(
                        "Nothing to quote".to_string(),
                    ));
                }
                
                // Get the value to quote
                let value = self.stack.pop().unwrap();
                
                // Push it as a quoted value
                self.stack.push(Value::Quoted(Box::new(value)));
                
                Ok(())
            },
            "for" => {
                // For loop control structure for iteration
                if self.stack.len() < 2 {
                    return Err(EvaluatorError::EvalError(
                        "Not enough operands for for operation (sequence, function)".to_string(),
                    ));
                }
                
                let function = self.stack.pop().unwrap();
                let sequence = self.stack.pop().unwrap();
                
                // Create a result list
                let mut results = Vec::new();
                
                // Iterate based on sequence type
                match &sequence {
                    Value::List(items) => {
                        // Iterate over list items
                        for item in items {
                            // Apply function to each item
                            self.stack.push(item.clone());
                            
                            // Call function - for now, we only support built-in operations
                            match &function {
                                Value::Symbol(op) => {
                                    self.execute_operation(op)?;
                                    
                                    // Add result to results
                                    if let Some(result) = self.stack.pop() {
                                        results.push(result);
                                    }
                                },
                                _ => {
                                    return Err(EvaluatorError::EvalError(
                                        "For loop function must be a symbol".to_string(),
                                    ));
                                }
                            }
                        }
                    },
                    Value::String(s) => {
                        // Iterate over string characters
                        for c in s.chars() {
                            // Push character as a string
                            self.stack.push(Value::String(c.to_string()));
                            
                            // Call function - for now, we only support built-in operations
                            match &function {
                                Value::Symbol(op) => {
                                    self.execute_operation(op)?;
                                    
                                    // Add result to results
                                    if let Some(result) = self.stack.pop() {
                                        results.push(result);
                                    }
                                },
                                _ => {
                                    return Err(EvaluatorError::EvalError(
                                        "For loop function must be a symbol".to_string(),
                                    ));
                                }
                            }
                        }
                    },
                    _ => {
                        return Err(EvaluatorError::EvalError(
                            "For loop sequence must be a list or string".to_string(),
                        ));
                    }
                }
                
                // Push results list to stack
                self.stack.push(Value::List(results));
                
                Ok(())
            },
            "range" => {
                // Create a numeric range
                if self.stack.len() < 2 {
                    return Err(EvaluatorError::EvalError(
                        "Not enough operands for range (start, end)".to_string(),
                    ));
                }
                
                let end = self.stack.pop().unwrap();
                let start = self.stack.pop().unwrap();
                
                match (start, end) {
                    (Value::Number(start_val), Value::Number(end_val)) => {
                        // Create a range as a map
                        let mut range_map = HashMap::new();
                        range_map.insert("start".to_string(), Value::Number(start_val));
                        range_map.insert("end".to_string(), Value::Number(end_val));
                        
                        self.stack.push(Value::Map(range_map));
                        Ok(())
                    },
                    _ => Err(EvaluatorError::EvalError(
                        "Range bounds must be numbers".to_string(),
                    )),
                }
            },
            "map" => {
                // Map function over a sequence
                if self.stack.len() < 2 {
                    return Err(EvaluatorError::EvalError(
                        "Not enough operands for map operation (sequence, function)".to_string(),
                    ));
                }
                
                // This is just a thin wrapper around for
                self.execute_operation("for")?;
                Ok(())
            },
            "length" => {
                // Get the length of a sequence
                if self.stack.is_empty() {
                    return Err(EvaluatorError::EvalError(
                        "Not enough operands for length".to_string(),
                    ));
                }
                
                let sequence = self.stack.pop().unwrap();
                
                // Get length based on sequence type
                let length = match &sequence {
                    Value::List(items) => items.len() as i32,
                    Value::String(s) => s.len() as i32,
                    Value::Map(map) => map.len() as i32,
                    _ => {
                        // For ranges, check if it's a map with start/end
                        if let Value::Map(map) = &sequence {
                            if map.contains_key("start") && map.contains_key("end") {
                                if let (Value::Number(start), Value::Number(end)) = 
                                    (map.get("start").unwrap(), map.get("end").unwrap()) {
                                    *end - *start
                                } else {
                                    0
                                }
                            } else {
                                map.len() as i32
                            }
                        } else {
                            return Err(EvaluatorError::EvalError(
                                format!("Cannot get length of {}", self.value_to_string(&sequence))
                            ));
                        }
                    }
                };
                
                self.stack.push(Value::Number(length));
                Ok(())
            },
            "format" => {
                // Format a string with arguments
                // Syntax: format_string args format -> result
                if self.stack.len() < 2 {
                    return Err(EvaluatorError::EvalError(
                        "Not enough operands for format".to_string(),
                    ));
                }
                
                let args = self.stack.pop().unwrap();
                let format_str = self.stack.pop().unwrap();
                
                // Get the format string
                let format_string = match format_str {
                    Value::String(s) => s,
                    _ => {
                        return Err(EvaluatorError::EvalError(
                            "Format string must be a string".to_string(),
                        ));
                    }
                };
                
                // Get the arguments
                let arg_list = match args {
                    Value::List(items) => items,
                    _ => {
                        // If not a list, treat as a single argument
                        vec![args]
                    }
                };
                
                // Simple implementation of format
                // Replace {} with arguments in order
                let mut result = format_string.clone();
                let mut arg_index = 0;
                
                // Find all {} and replace them
                while let Some(pos) = result.find("{}") {
                    if arg_index < arg_list.len() {
                        // Get the argument
                        let arg = &arg_list[arg_index];
                        
                        // Convert to string
                        let arg_str = self.value_to_string(arg);
                        
                        // Replace the {}
                        result = result.replacen("{}", &arg_str, 1);
                        
                        // Increment the argument index
                        arg_index += 1;
                    } else {
                        // No more arguments, replace with empty string
                        result = result.replacen("{}", "", 1);
                    }
                }
                
                // Push the result
                self.stack.push(Value::String(result));
                Ok(())
            },
            "contains" => {
                // Check if a string contains a substring
                // Syntax: string substring contains -> boolean
                if self.stack.len() < 2 {
                    return Err(EvaluatorError::EvalError(
                        "Not enough operands for contains".to_string(),
                    ));
                }
                
                let substring = self.stack.pop().unwrap();
                let string = self.stack.pop().unwrap();
                
                // Get the strings
                let (str, substr) = match (string, substring) {
                    (Value::String(s), Value::String(sub)) => (s, sub),
                    _ => {
                        return Err(EvaluatorError::EvalError(
                            "Contains requires two strings".to_string(),
                        ));
                    }
                };
                
                // Check if the string contains the substring
                let result = str.contains(&substr);
                
                // Push the result
                self.stack.push(Value::Number(if result { 1 } else { 0 }));
                Ok(())
            },
            "escape_string" => {
                // Escape special characters in a string
                // Syntax: string escape_string -> escaped_string
                if self.stack.is_empty() {
                    return Err(EvaluatorError::EvalError(
                        "Not enough operands for escape_string".to_string(),
                    ));
                }
                
                let string = self.stack.pop().unwrap();
                
                // Get the string
                let str = match string {
                    Value::String(s) => s,
                    _ => {
                        return Err(EvaluatorError::EvalError(
                            "Escape string requires a string".to_string(),
                        ));
                    }
                };
                
                // Escape special characters
                let escaped = str
                    .replace("\\", "\\\\")
                    .replace("\"", "\\\"")
                    .replace("\n", "\\n")
                    .replace("\t", "\\t")
                    .replace("\r", "\\r");
                
                // Push the result
                self.stack.push(Value::String(escaped));
                Ok(())
            },
            "str_slice" => {
                // Get a substring
                // Syntax: string start end str_slice -> substring
                if self.stack.len() < 3 {
                    return Err(EvaluatorError::EvalError(
                        "Not enough operands for str_slice".to_string(),
                    ));
                }
                
                let end = self.stack.pop().unwrap();
                let start = self.stack.pop().unwrap();
                let string = self.stack.pop().unwrap();
                
                // Get the string and indices
                let (str, start_idx, end_idx) = match (string, start, end) {
                    (Value::String(s), Value::Number(start), Value::Number(end)) => {
                        (s, start as usize, end as usize)
                    },
                    _ => {
                        return Err(EvaluatorError::EvalError(
                            "String slice requires a string and two indices".to_string(),
                        ));
                    }
                };
                
                // Check bounds
                if start_idx > str.len() {
                    self.stack.push(Value::String("".to_string()));
                    return Ok(());
                }
                
                let end_idx = std::cmp::min(end_idx, str.len());
                
                // Get the substring (handling UTF-8 correctly)
                let substring = str.chars()
                    .skip(start_idx)
                    .take(end_idx - start_idx)
                    .collect::<String>();
                
                // Push the result
                self.stack.push(Value::String(substring));
                Ok(())
            },
            "str_index_of" => {
                // Find a substring in a string
                // Syntax: string substring start str_index_of -> index
                if self.stack.len() < 3 {
                    return Err(EvaluatorError::EvalError(
                        "Not enough operands for str_index_of".to_string(),
                    ));
                }
                
                let start = self.stack.pop().unwrap();
                let substring = self.stack.pop().unwrap();
                let string = self.stack.pop().unwrap();
                
                // Get the string, substring, and start index
                let (str, substr, start_idx) = match (string, substring, start) {
                    (Value::String(s), Value::String(sub), Value::Number(start)) => {
                        (s, sub, start as usize)
                    },
                    _ => {
                        return Err(EvaluatorError::EvalError(
                            "String index_of requires two strings and a number".to_string(),
                        ));
                    }
                };
                
                // Check bounds
                if start_idx >= str.len() || substr.is_empty() {
                    self.stack.push(Value::Number(-1));
                    return Ok(());
                }
                
                // Find the substring
                let index = match str[start_idx..].find(&substr) {
                    Some(idx) => (idx + start_idx) as i32,
                    None => -1,
                };
                
                // Push the result
                self.stack.push(Value::Number(index));
                Ok(())
            },
            "read_file" => {
                // Read a file from the filesystem
                // Syntax: path read_file -> content
                if self.stack.is_empty() {
                    return Err(EvaluatorError::EvalError(
                        "Not enough operands for read_file".to_string(),
                    ));
                }
                
                let path = self.stack.pop().unwrap();
                
                // Get the path as a string
                let path_str = match path {
                    Value::String(s) => s,
                    _ => {
                        return Err(EvaluatorError::EvalError(
                            "File path must be a string".to_string(),
                        ));
                    }
                };
                
                // Read the file
                match fs::read_to_string(&path_str) {
                    Ok(content) => {
                        self.stack.push(Value::String(content));
                        Ok(())
                    },
                    Err(err) => {
                        Err(EvaluatorError::FileError(err))
                    }
                }
            },
            "list" => {
                // Create a list from stack items
                if self.stack.is_empty() {
                    // Empty list
                    self.stack.push(Value::List(Vec::new()));
                    return Ok(());
                }
                
                // Get the count from stack
                let count_val = match self.stack.pop().unwrap() {
                    Value::Number(n) => n as usize,
                    _ => {
                        return Err(EvaluatorError::EvalError(
                            "List creation requires a count".to_string(),
                        ));
                    }
                };
                
                // Make sure we have enough items
                if self.stack.len() < count_val {
                    return Err(EvaluatorError::EvalError(
                        format!("Not enough items for list (needed {}, had {})", 
                                count_val, self.stack.len())
                    ));
                }
                
                // Pop items for the list
                let mut items = Vec::with_capacity(count_val);
                for _ in 0..count_val {
                    items.push(self.stack.pop().unwrap());
                }
                
                // Reverse to maintain order
                items.reverse();
                
                // Push the new list
                self.stack.push(Value::List(items));
                Ok(())
            },
            "get" => {
                // Get item at index from a sequence
                if self.stack.len() < 2 {
                    return Err(EvaluatorError::EvalError(
                        "Not enough operands for get operation (sequence, index)".to_string(),
                    ));
                }
                
                let index_val = self.stack.pop().unwrap();
                let sequence = self.stack.pop().unwrap();
                
                // Handle string index as field name for maps
                match (&sequence, &index_val) {
                    (Value::Map(map), Value::String(field)) => {
                        // Treat the string as a field name
                        if let Some(value) = map.get(field) {
                            self.stack.push(value.clone());
                            return Ok(());
                        } else {
                            return Err(EvaluatorError::EvalError(
                                format!("Field '{}' not found in map", field)
                            ));
                        }
                    },
                    _ => {
                        // Continue with numeric indexing
                    }
                }
                
                // Convert index to usize
                let index = match index_val {
                    Value::Number(n) => {
                        if n < 0 {
                            return Err(EvaluatorError::EvalError(
                                "Index cannot be negative".to_string(),
                            ));
                        }
                        n as usize
                    },
                    _ => {
                        return Err(EvaluatorError::EvalError(
                            "Index must be a number".to_string(),
                        ));
                    }
                };
                
                // Get item based on sequence type
                let result = match &sequence {
                    Value::List(items) => {
                        if index < items.len() {
                            items[index].clone()
                        } else {
                            return Err(EvaluatorError::EvalError(
                                format!("Index {} out of bounds for list of length {}", 
                                        index, items.len())
                            ));
                        }
                    },
                    Value::String(s) => {
                        if index < s.len() {
                            // Get character at index
                            let ch = s.chars().nth(index).unwrap();
                            Value::String(ch.to_string())
                        } else {
                            return Err(EvaluatorError::EvalError(
                                format!("Index {} out of bounds for string of length {}", 
                                        index, s.len())
                            ));
                        }
                    },
                    Value::Map(map) => {
                        // For ranges, compute the value
                        if map.contains_key("start") && map.contains_key("end") {
                            if let Value::Number(start) = map.get("start").unwrap() {
                                Value::Number(start + index as i32)
                            } else {
                                return Err(EvaluatorError::EvalError(
                                    "Invalid range".to_string(),
                                ));
                            }
                        } else {
                            return Err(EvaluatorError::EvalError(
                                "Cannot index into a general map".to_string(),
                            ));
                        }
                    },
                    _ => {
                        return Err(EvaluatorError::EvalError(
                            format!("Cannot index into {}", self.value_to_string(&sequence))
                        ));
                    }
                };
                
                self.stack.push(result);
                Ok(())
            },
            "has_field" => {
                // Check if a map has a field
                // Syntax: map field has_field -> boolean
                if self.stack.len() < 2 {
                    return Err(EvaluatorError::EvalError(
                        "Not enough operands for has_field (map, field)".to_string(),
                    ));
                }
                
                let field = self.stack.pop().unwrap();
                let map_val = self.stack.pop().unwrap();
                
                // Get the field name
                let field_name = match field {
                    Value::String(s) => s,
                    Value::Symbol(ref s) => s.clone(),
                    _ => {
                        return Err(EvaluatorError::EvalError(
                            "Field name must be a string or symbol".to_string(),
                        ));
                    }
                };
                
                // Check if the map has the field
                let has_field = match &map_val {
                    Value::Map(map) => map.contains_key(&field_name),
                    _ => false,
                };
                
                self.stack.push(Value::Number(if has_field { 1 } else { 0 }));
                Ok(())
            },
            _ => Err(EvaluatorError::EvalError(format!(
                "Unknown operation: {}",
                operation
            ))),
        }
    }
}

// Simple Env implementation
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
