// src/repl/interpreter/errors.rs
// Error handling for the Borf interpreter

use std::fmt;
use miette::{Diagnostic, SourceSpan};
use pest::error::Error as PestError;
use pest::iterators::Pair;
use pest::Span;
use thiserror::Error;

/// Span information for error reporting
#[derive(Debug, Clone)]
pub struct BorfSpan {
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub column: usize,
    pub snippet: String,
}

impl BorfSpan {
    pub fn new(start: usize, end: usize, line: usize, column: usize, snippet: String) -> Self {
        Self {
            start,
            end,
            line,
            column,
            snippet,
        }
    }

    pub fn from_pest_span(span: Span) -> Self {
        Self {
            start: span.start(),
            end: span.end(),
            line: span.line_col().0,
            column: span.line_col().1,
            snippet: span.as_str().to_string(),
        }
    }

    pub fn to_source_span(&self) -> SourceSpan {
        (self.start, self.end - self.start).into()
    }
}

impl fmt::Display for BorfSpan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}, column {}", self.line, self.column)
    }
}

/// Error types for the Borf interpreter
#[derive(Error, Debug, Diagnostic)]
pub enum BorfError {
    #[error("File error: {0}")]
    #[diagnostic(code(borf::file_error))]
    FileError(#[from] std::io::Error),

    // More specific parse errors
    #[error("Parse error: {message}")]
    #[diagnostic(code(borf::parse_error), help("{help}"))]
    ParseError {
        message: String,
        #[source_code]
        src: Option<String>,
        #[label("here")]
        span: Option<SourceSpan>,
        help: String,
    },

    #[error("Unexpected token: found {found} but expected {expected}")]
    #[diagnostic(code(borf::unexpected_token))]
    UnexpectedToken {
        found: String,
        expected: String,
        #[source_code]
        src: Option<String>,
        #[label("unexpected token")]
        span: Option<SourceSpan>,
        #[help]
        help: Option<String>,
    },

    #[error("Unterminated delimiter: missing closing '{delimiter}'")]
    #[diagnostic(code(borf::unterminated_delimiter))]
    UnterminatedDelimiter {
        delimiter: char,
        #[source_code]
        src: Option<String>,
        #[label("opening delimiter here")]
        opening_span: Option<SourceSpan>,
        #[help]
        help: Option<String>,
    },

    #[error("Unmatched delimiter: found '{found}' with no matching opening delimiter")]
    #[diagnostic(code(borf::unmatched_delimiter))]
    UnmatchedDelimiter {
        found: char,
        #[source_code]
        src: Option<String>,
        #[label("unmatched delimiter")]
        span: Option<SourceSpan>,
    },

    // Stack effect errors
    #[error("Stack effect error: {message}")]
    #[diagnostic(code(borf::stack_effect_error), help("{help}"))]
    StackEffectError {
        message: String,
        #[source_code]
        src: Option<String>,
        #[label("here")]
        span: Option<SourceSpan>,
        help: String,
    },

    #[error("Invalid stack effect declaration: {message}")]
    #[diagnostic(code(borf::invalid_stack_effect))]
    InvalidStackEffect {
        message: String,
        #[source_code]
        src: Option<String>,
        #[label("invalid stack effect")]
        span: Option<SourceSpan>,
        #[help]
        help: Option<String>,
    },

    #[error("Stack underflow: attempted to access item at depth {depth} but stack only has {available} items")]
    #[diagnostic(code(borf::stack_underflow))]
    StackUnderflow {
        depth: usize,
        available: usize,
        #[source_code]
        src: Option<String>,
        #[label("stack access here")]
        span: Option<SourceSpan>,
        #[help]
        help: Option<String>,
    },

    // Parameter reference errors
    #[error("Unknown parameter: '{name}' is not a defined parameter")]
    #[diagnostic(code(borf::unknown_parameter))]
    UnknownParameter {
        name: String,
        #[source_code]
        src: Option<String>,
        #[label("unknown parameter")]
        span: Option<SourceSpan>,
        #[related]
        params_available: Vec<(SourceSpan, String)>,
    },

    #[error("Parameter depth error: could not access parameter '{name}' at calculated depth {depth}")]
    #[diagnostic(code(borf::parameter_depth_error))]
    ParameterDepthError {
        name: String,
        depth: isize,
        #[source_code]
        src: Option<String>,
        #[label("parameter reference here")]
        span: Option<SourceSpan>,
    },

    // Evaluation errors
    #[error("Evaluation error: {message}")]
    #[diagnostic(code(borf::eval_error), help("{help}"))]
    EvalError {
        message: String,
        #[source_code]
        src: Option<String>,
        #[label("here")]
        span: Option<SourceSpan>,
        help: String,
    },

    #[error("Undefined symbol: '{name}' is not defined in this scope")]
    #[diagnostic(code(borf::undefined_symbol))]
    UndefinedSymbol {
        name: String,
        #[source_code]
        src: Option<String>,
        #[label("undefined symbol")]
        span: Option<SourceSpan>,
        #[related]
        similar_names: Vec<(SourceSpan, String)>,
        #[help]
        help: Option<String>,
    },

    #[error("Invalid operation: {operation} cannot be applied to {types}")]
    #[diagnostic(code(borf::invalid_operation))]
    InvalidOperation {
        operation: String,
        types: String,
        #[source_code]
        src: Option<String>,
        #[label("invalid operation")]
        span: Option<SourceSpan>,
        #[help]
        help: Option<String>,
    },

    // Type errors
    #[error("Type error: {message}")]
    #[diagnostic(code(borf::type_error), help("{help}"))]
    TypeError {
        message: String,
        #[source_code]
        src: Option<String>,
        #[label("here")]
        span: Option<SourceSpan>,
        help: String,
    },

    #[error("Type mismatch: expected {expected} but found {found}")]
    #[diagnostic(code(borf::type_mismatch))]
    TypeMismatch {
        expected: String,
        found: String,
        #[source_code]
        src: Option<String>,
        #[label("type mismatch")]
        span: Option<SourceSpan>,
        #[help]
        help: Option<String>,
    },

    #[error("Missing field: record is missing required field '{field}'")]
    #[diagnostic(code(borf::missing_field))]
    MissingField {
        field: String,
        #[source_code]
        src: Option<String>,
        #[label("record here")]
        span: Option<SourceSpan>,
        #[help]
        help: Option<String>,
    },

    // Resource errors
    #[error("Resource error: {message}")]
    #[diagnostic(code(borf::resource_error))]
    ResourceError {
        message: String,
        #[source_code]
        src: Option<String>,
        #[label("here")]
        span: Option<SourceSpan>,
    },

    #[error("Import error: failed to import module '{module}': {reason}")]
    #[diagnostic(code(borf::import_error))]
    ImportError {
        module: String,
        reason: String,
        #[source_code]
        src: Option<String>,
        #[label("import failed")]
        span: Option<SourceSpan>,
        #[help]
        help: Option<String>,
    },

    // Generic error for extending with more specific variants later
    #[error("{message}")]
    #[diagnostic(code(borf::generic_error))]
    GenericError {
        message: String,
        #[source_code]
        src: Option<String>,
        #[label("{label}")]
        span: Option<SourceSpan>,
        label: String,
        #[help]
        help: Option<String>,
    },
}

impl From<PestError<crate::repl::interpreter::parser::Rule>> for BorfError {
    fn from(error: PestError<crate::repl::interpreter::parser::Rule>) -> Self {
        // Get the original message
        let message = error.to_string();
        let src = error.input().map(|s| s.to_string());
        
        // Extract line/column information if available
        let span = match error.line_col {
            Some((line, col)) => {
                // Calculate an approximate span based on line/column
                let start = (line - 1) * 80 + col; // Rough estimate
                let end = start + 1;
                Some((start, 1).into())
            },
            None => None,
        };
        
        // Extract the expected tokens for better error messages
        let (expected, found) = match &error.variant {
            pest::error::ErrorVariant::ParsingError { positives, negatives } => {
                let expected = if positives.is_empty() {
                    "end of input".to_string()
                } else {
                    positives.iter()
                        .map(|rule| format!("{:?}", rule))
                        .collect::<Vec<_>>()
                        .join(", ")
                };
                
                let found = if let Some(pos) = error.location {
                    if pos < error.input().unwrap_or("").len() {
                        format!("'{}'", &error.input().unwrap_or("")[pos..=pos])
                    } else {
                        "end of input".to_string()
                    }
                } else {
                    "unknown".to_string()
                };
                
                (expected, found)
            },
            _ => ("valid input".to_string(), "invalid input".to_string()),
        };
        
        // Provide helpful message based on error type
        match &error.variant {
            pest::error::ErrorVariant::ParsingError { .. } => {
                // Check if it looks like an unterminated delimiter issue
                if message.contains("expected") && (
                    message.contains("]") || message.contains(")") || message.contains("}") || message.contains("\"")
                ) {
                    let delimiter = if message.contains("]") {
                        ']'
                    } else if message.contains(")") {
                        ')'
                    } else if message.contains("}") {
                        '}'
                    } else {
                        '"'
                    };
                    
                    BorfError::UnterminatedDelimiter {
                        delimiter,
                        src,
                        opening_span: span,
                        help: Some(format!("Add closing '{}' to complete this expression", delimiter)),
                    }
                } else {
                    // General unexpected token error
                    BorfError::UnexpectedToken {
                        found,
                        expected,
                        src,
                        span,
                        help: Some(format!("Did you mean to use one of these: {}?", expected)),
                    }
                }
            },
            _ => {
                // Fallback to generic parse error
                BorfError::ParseError {
                    message,
                    src,
                    span,
                    help: "Check the syntax and ensure it follows Borf grammar rules".to_string(),
                }
            }
        }
    }
}

pub type Result<T> = std::result::Result<T, BorfError>;

/// Helper methods for creating specific errors with appropriate context
impl BorfError {
    /// Create a new stack underflow error with helpful context
    pub fn stack_underflow(depth: usize, available: usize, src: Option<String>, span: Option<SourceSpan>) -> Self {
        let help = if depth > available {
            format!(
                "You're trying to access an item at depth {}, but only {} item(s) are available on the stack. \
                 Make sure your stack has enough items before this operation.",
                depth, available
            )
        } else {
            "Check that your stack operations are balanced.".to_string()
        };
        
        Self::StackUnderflow {
            depth,
            available,
            src,
            span,
            help: Some(help),
        }
    }
    
    /// Create a new type mismatch error with helpful context
    pub fn type_mismatch(expected: &str, found: &str, src: Option<String>, span: Option<SourceSpan>) -> Self {
        let help = format!(
            "Expected a value of type '{}' but found '{}'.
             Check that the types of your expressions match what the operation expects.",
            expected, found
        );
        
        Self::TypeMismatch {
            expected: expected.to_string(),
            found: found.to_string(),
            src,
            span,
            help: Some(help),
        }
    }
    
    /// Create a new undefined symbol error with possible suggestions
    pub fn undefined_symbol(name: &str, similar: Vec<String>, src: Option<String>, span: Option<SourceSpan>) -> Self {
        let related = similar.iter()
            .map(|s| (span.unwrap_or((0, 0).into()), format!("Did you mean '{}'?", s)))
            .collect();
            
        let help = if !similar.is_empty() {
            Some(format!("Did you mean '{}'?", similar[0]))
        } else {
            Some("Make sure the symbol is defined before it's used.".to_string())
        };
        
        Self::UndefinedSymbol {
            name: name.to_string(),
            src,
            span,
            similar_names: related,
            help,
        }
    }
    
    /// Create a new unknown parameter error with available parameters
    pub fn unknown_parameter(name: &str, available: Vec<String>, src: Option<String>, span: Option<SourceSpan>) -> Self {
        let related = available.iter()
            .map(|s| (span.unwrap_or((0, 0).into()), format!("Available parameter: '{}'", s)))
            .collect();
            
        Self::UnknownParameter {
            name: name.to_string(),
            src,
            span,
            params_available: related,
        }
    }
    
    /// Create a new parameter depth error with context
    pub fn parameter_depth_error(name: &str, depth: isize, src: Option<String>, span: Option<SourceSpan>) -> Self {
        Self::ParameterDepthError {
            name: name.to_string(),
            depth,
            src,
            span,
        }
    }
    
    /// Create a new invalid stack effect declaration error
    pub fn invalid_stack_effect(message: &str, src: Option<String>, span: Option<SourceSpan>) -> Self {
        let help = Some(format!(
            "Stack effect declarations should have the form '( input1 input2 -- output1 output2 )'. \
             Check that you have the correct format with inputs, the -- separator, and outputs."
        ));
        
        Self::InvalidStackEffect {
            message: message.to_string(),
            src,
            span,
            help,
        }
    }
}