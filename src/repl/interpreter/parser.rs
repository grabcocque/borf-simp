// src/repl/interpreter/parser.rs
// This module provides the parser for the Borf interpreter

use std::collections::HashMap;
use crate::repl::interpreter::types::{EvaluatorError, Expr, Param, Pattern, Result, Type, Value};
use crate::repl::interpreter::stack_effects::translate_quotation;

// Define the Parser struct
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
        
        // Parse the initial expression
        let expr = self.parse_basic_expr(token)?;
        
        // Check if there's a pipeline operator after this expression
        if self.position < self.tokens.len() && &self.tokens[self.position] == "|>" {
            self.position += 1; // Skip the |> token
            
            // Parse the right side of the pipeline
            if self.position < self.tokens.len() {
                let right = self.parse_expr()?;
                return Ok(Expr::Pipeline(Box::new(expr), Box::new(right)));
            } else {
                return Err(EvaluatorError::ParseError("Expected expression after |>".to_string()));
            }
        }
        
        Ok(expr)
    }
    
    // Parse a basic expression without looking for pipelines
    fn parse_basic_expr(&mut self, token: &str) -> Result<Expr> {
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
                        // Apply translation from named parameters to explicit stack operations
                        if !params.is_empty() {
                            // Only translate if we have named parameters
                            match translate_quotation(&params, &exprs) {
                                Ok(translated_exprs) => return Ok(Expr::TypedQuotation(
                                    Vec::new(),
                                    translated_exprs,
                                    Box::new(Type::Simple(type_name)),
                                )),
                                Err(e) => return Err(e)
                            }
                        } else {
                            // No parameters, no translation needed
                            return Ok(Expr::TypedQuotation(
                                params,
                                exprs,
                                Box::new(Type::Simple(type_name)),
                            ));
                        }
                    }
                }

                // Regular quotation without return type
                // Apply translation from named parameters to explicit stack operations
                if !params.is_empty() {
                    // Only translate if we have named parameters
                    match translate_quotation(&params, &exprs) {
                        Ok(translated_exprs) => Ok(Expr::Quotation(Vec::new(), translated_exprs)),
                        Err(e) => Err(e)
                    }
                } else {
                    // No parameters, no translation needed
                    Ok(Expr::Quotation(params, exprs))
                }
            } else {
                Err(EvaluatorError::ParseError("Unclosed quotation".to_string()))
            }
        } else {
            // For simplicity, treat everything else as a symbol
            Ok(Expr::Symbol(token.to_string()))
        }
    }
}