// src/repl/interpreter/stack_effects.rs
// This module provides the stack effect declarations and translation for the Borf interpreter

use std::collections::HashMap;
use crate::repl::interpreter::types::{EvaluatorError, Expr, Param, Result};

// Stack effect declaration for a word
#[derive(Debug, Clone, PartialEq)]
pub struct StackEffect {
    pub inputs: usize,
    pub outputs: usize,
}

impl StackEffect {
    pub fn new(inputs: usize, outputs: usize) -> Self {
        StackEffect { inputs, outputs }
    }

    // Calculate the net change in stack depth
    pub fn stack_depth_change(&self) -> isize {
        self.outputs as isize - self.inputs as isize
    }
}

// Function to look up the stack effect of a known word
pub fn get_word_effect(word: &str) -> Option<StackEffect> {
    match word {
        // Core stack operations
        "dup" => Some(StackEffect::new(1, 2)),  // ( a -- a a )
        "drop" => Some(StackEffect::new(1, 0)), // ( a -- )
        "swap" => Some(StackEffect::new(2, 2)), // ( a b -- b a )
        "rot" => Some(StackEffect::new(3, 3)),  // ( a b c -- b c a )
        
        // Arithmetic operations
        "+" | "add" => Some(StackEffect::new(2, 1)),  // ( a b -- a+b )
        "-" | "sub" => Some(StackEffect::new(2, 1)),  // ( a b -- a-b )
        "*" | "mul" => Some(StackEffect::new(2, 1)),  // ( a b -- a*b )
        "/" | "div" => Some(StackEffect::new(2, 1)),  // ( a b -- a/b )
        "sqrt" => Some(StackEffect::new(1, 1)),       // ( a -- sqrt(a) )
        
        // Logical operations
        "and" => Some(StackEffect::new(2, 1)),        // ( a b -- a&b )
        "or" => Some(StackEffect::new(2, 1)),         // ( a b -- a|b )
        "not" => Some(StackEffect::new(1, 1)),        // ( a -- !a )
        "==" | "eq" => Some(StackEffect::new(2, 1)),  // ( a b -- a==b )
        
        // Other built-in operations
        "print" => Some(StackEffect::new(1, 0)),      // ( a -- )
        
        // Stack effect for literals (they just push one value)
        _ if word.parse::<i32>().is_ok() => Some(StackEffect::new(0, 1)),   // ( -- n )
        _ if word.starts_with('"') && word.ends_with('"') => Some(StackEffect::new(0, 1)), // ( -- s )
        
        // Unknown word
        _ => None,
    }
}

// Translator to convert named parameter quotations to explicit stack operations
pub struct StackTranslator {
    // Map from parameter name to its initial stack depth before the body starts
    param_depths: HashMap<String, usize>,
    // Current stack depth increase caused by operations within the body
    current_stack_depth_increase: isize,
    // The output list of stack operations
    output: Vec<String>,
}

impl StackTranslator {
    pub fn new() -> Self {
        StackTranslator {
            param_depths: HashMap::new(),
            current_stack_depth_increase: 0,
            output: Vec::new(),
        }
    }

    // Translate a quotation with named parameters to explicit stack operations
    pub fn translate_quotation(&mut self, params: &[Param], body: &[Expr]) -> Result<Vec<Expr>> {
        // Reset state
        self.param_depths.clear();
        self.current_stack_depth_increase = 0;
        self.output.clear();
        
        // Map initial parameter depths
        for (i, param) in params.iter().enumerate().rev() {
            // Last parameter (rightmost) is at depth 0, second-to-last at depth 1, etc.
            self.param_depths.insert(param.name.clone(), i);
        }
        
        // Translate the body expressions
        for expr in body {
            self.translate_expr(expr)?;
        }
        
        // Convert output strings back to expressions
        let mut result = Vec::new();
        for op in &self.output {
            if op.contains(' ') {
                // Handle "N pick" operations
                let parts: Vec<&str> = op.split_whitespace().collect();
                if parts.len() == 2 && parts[1] == "pick" {
                    // Parse N from "N pick"
                    if let Ok(n) = parts[0].parse::<i32>() {
                        result.push(Expr::Number(n));
                        result.push(Expr::Symbol("pick".to_string()));
                    } else {
                        return Err(EvaluatorError::ParseError(format!("Invalid pick depth: {}", parts[0])));
                    }
                } else {
                    // Other multi-word operations (unexpected)
                    return Err(EvaluatorError::ParseError(format!("Unexpected multi-word operation: {}", op)));
                }
            } else if let Ok(n) = op.parse::<i32>() {
                // Number literal
                result.push(Expr::Number(n));
            } else if op.starts_with('"') && op.ends_with('"') {
                // String literal
                result.push(Expr::String(op[1..op.len()-1].to_string()));
            } else {
                // Symbol or operation
                result.push(Expr::Symbol(op.to_string()));
            }
        }
        
        Ok(result)
    }
    
    // Translate a single expression
    fn translate_expr(&mut self, expr: &Expr) -> Result<()> {
        match expr {
            Expr::Number(n) => {
                // Push the number onto the stack
                self.output.push(n.to_string());
                self.current_stack_depth_increase += 1;
            },
            Expr::String(s) => {
                // Push the string onto the stack
                self.output.push(format!("\"{}\"", s));
                self.current_stack_depth_increase += 1;
            },
            Expr::Symbol(s) => {
                // Check if it's a parameter name
                if let Some(&initial_depth) = self.param_depths.get(s) {
                    // Parameter reference - calculate actual depth and generate pick operation
                    let actual_depth = initial_depth as isize + self.current_stack_depth_increase;
                    if actual_depth < 0 {
                        return Err(EvaluatorError::ParseError(
                            format!("Invalid stack depth for parameter '{}': {}", s, actual_depth)
                        ));
                    }
                    self.output.push(format!("{} pick", actual_depth));
                    self.current_stack_depth_increase += 1;
                } else {
                    // Regular word - look up its stack effect
                    let stack_effect = get_word_effect(s)
                        .ok_or_else(|| EvaluatorError::ParseError(
                            format!("Unknown word '{}' with no stack effect declaration", s)
                        ))?;
                    
                    // Add the word to the output
                    self.output.push(s.clone());
                    
                    // Update stack depth based on the word's effect
                    self.current_stack_depth_increase += stack_effect.stack_depth_change();
                }
            },
            Expr::Pipeline(left, right) => {
                // Handle pipeline by translating the left side, then the right
                // The |> operator is just syntactic sugar and doesn't translate to any operation
                self.translate_expr(left)?;
                self.translate_expr(right)?;
            },
            Expr::Quotation(inner_params, inner_body) => {
                // For nested quotations, we need to store the current state
                let saved_params = self.param_depths.clone();
                let saved_depth = self.current_stack_depth_increase;
                
                // Translate the inner quotation
                if !inner_params.is_empty() {
                    match translate_quotation(inner_params, inner_body) {
                        Ok(translated_exprs) => {
                            // Push the quotation as a literal to the output
                            self.output.push("[".to_string());
                            for expr in translated_exprs {
                                self.translate_expr(&expr)?;
                            }
                            self.output.push("]".to_string());
                        },
                        Err(e) => return Err(e)
                    }
                } else {
                    // No parameters in inner quotation, just output it directly
                    self.output.push("[".to_string());
                    for expr in inner_body {
                        self.translate_expr(expr)?;
                    }
                    self.output.push("]".to_string());
                }
                
                // A quotation is a single item on the stack
                self.current_stack_depth_increase += 1;
                
                // Restore the outer quotation state
                self.param_depths = saved_params;
                // Note: we keep the updated stack depth increase
            },
            // For other expression types, we'd need more complex translation
            // This is a simplified implementation focusing on basic expressions
            _ => {
                return Err(EvaluatorError::ParseError(
                    format!("Unsupported expression in translation: {:?}", expr)
                ));
            }
        }
        
        Ok(())
    }
}

// Function to translate a named parameter quotation to explicit stack operations
pub fn translate_quotation(params: &[Param], body: &[Expr]) -> Result<Vec<Expr>> {
    let mut translator = StackTranslator::new();
    translator.translate_quotation(params, body)
}