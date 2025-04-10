// src/repl/interpreter/types.rs
// This module defines the core type definitions for the Borf interpreter

use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;
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
    Resource(usize, Box<Value>),            // Resource value with ID and inner value
    BorrowedResource(usize, Box<Value>),    // Borrowed resource that can't be consumed
    Optional(Option<Box<Value>>),           // Optional value ?value (value or Nothing)
    Variant(String, Vec<Value>),            // Variant like tag(val)
    Nothing,                                // Represents "Nothing" value
    Nil,                                    // For internal use
}

// Environment to store bound values
#[derive(Debug, Clone, PartialEq)]
pub struct Env {
    pub bindings: HashMap<String, Value>,
    pub parent: Option<Box<Env>>,
}

// Implement Display for Value
impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Number(n) => write!(f, "{}", n),
            Value::String(s) => write!(f, "\"{}\"", s),
            Value::Symbol(s) => write!(f, "{}", s),
            Value::Quotation(_, _, _) => write!(f, "[...]"),
            Value::TypedQuotation(_, _, _, _) => write!(f, "[...] : Type"),
            Value::Pipeline(_, _) => write!(f, "pipeline"),
            Value::List(items) => {
                write!(f, "[")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", item)?;
                }
                write!(f, "]")
            }
            Value::Map(_) => write!(f, "{{...}}"),
            Value::Quoted(inner) => write!(f, "'{}", inner),
            Value::Quasiquoted(inner) => write!(f, "`{}", inner),
            Value::Type(typ) => write!(f, "{:?}", typ),
            Value::QuotedType(typ) => write!(f, "#{:?}", typ),
            Value::Module(name, _) => write!(f, "module {}", name),
            Value::Resource(id, inner) => write!(f, "resource({}, {})", id, inner),
            Value::BorrowedResource(id, inner) => write!(f, "borrowed({}, {})", id, inner),
            Value::Optional(Some(inner)) => write!(f, "?{}", inner),
            Value::Optional(None) => write!(f, "Nothing"),
            Value::Nothing => write!(f, "Nothing"),
            Value::Variant(name, values) => {
                write!(f, "{}", name)?;
                if !values.is_empty() {
                    write!(f, "(")?;
                    for (i, val) in values.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{}", val)?;
                    }
                    write!(f, ")")?;
                }
                Ok(())
            }
            Value::Nil => write!(f, "nil"),
        }
    }
}

// Extension methods for Value
impl Value {
    // Check if the value is empty (for backward compatibility)
    pub fn is_empty(&self) -> bool {
        match self {
            Value::String(s) => s.is_empty(),
            Value::List(l) => l.is_empty(),
            Value::Map(m) => m.is_empty(),
            Value::Nil => true,
            Value::Nothing => true,
            _ => false,
        }
    }
    
    // Get a colored (green) representation (for backward compatibility)
    pub fn green(&self) -> String {
        format!("{}", self)
    }
    
    // Get a trimmed representation (for backward compatibility)
    pub fn trim(&self) -> String {
        format!("{}", self).trim().to_string()
    }
}