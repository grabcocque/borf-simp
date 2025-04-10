// src/repl/interpreter/stack_effects.rs
// Implementation of the STACKER algorithm for translating named parameters to stack operations

use std::collections::HashMap;
use crate::repl::interpreter::errors::{BorfError, Result};
use crate::repl::interpreter::types::{Expr, Param};

/// Represents the stack effect of a word
#[derive(Debug, Clone, PartialEq)]
pub struct StackEffect {
    pub inputs: Vec<String>,   // Names of input items
    pub outputs: Vec<String>,  // Names of output items
}

impl StackEffect {
    pub fn new(inputs: Vec<String>, outputs: Vec<String>) -> Self {
        Self { inputs, outputs }
    }
    
    // Calculate the net change in stack depth
    pub fn stack_depth_change(&self) -> isize {
        self.outputs.len() as isize - self.inputs.len() as isize
    }
}

/// Parse a stack effect declaration string
pub fn parse_stack_effect(effect_str: &str) -> Result<StackEffect> {
    // Stack effect format: ( inputs -- outputs )
    let effect_str = effect_str.trim();
    
    // Check for basic format
    if !effect_str.starts_with('(') || !effect_str.ends_with(')') {
        return Err(BorfError::StackEffectError {
            message: format!("Invalid stack effect format: {}", effect_str),
            src: None,
            span: None,
            help: "Stack effect declarations should have the form '( input1 input2 -- output1 output2 )'".to_string(),
        });
    }
    
    // Remove the outer parentheses
    let inner = &effect_str[1..effect_str.len()-1].trim();
    
    // Split by the "--" separator
    let parts: Vec<&str> = inner.split("--").collect();
    if parts.len() != 2 {
        return Err(BorfError::StackEffectError {
            message: format!("Stack effect must contain exactly one '--' separator: {}", effect_str),
            src: None,
            span: None,
            help: "Stack effect declarations should have the form '( input1 input2 -- output1 output2 )'".to_string(),
        });
    }
    
    // Parse inputs and outputs
    let inputs = parts[0]
        .trim()
        .split_whitespace()
        .map(|s| s.to_string())
        .collect();
    
    let outputs = parts[1]
        .trim()
        .split_whitespace()
        .map(|s| s.to_string())
        .collect();
    
    Ok(StackEffect::new(inputs, outputs))
}

/// Get the stack effect for a built-in word
pub fn get_word_effect(word: &str) -> Option<StackEffect> {
    match word {
        // Core stack operations
        "dup" => Some(StackEffect::new(
            vec!["a".to_string()], 
            vec!["a".to_string(), "a".to_string()]
        )),
        "drop" => Some(StackEffect::new(
            vec!["a".to_string()], 
            vec![]
        )),
        "swap" => Some(StackEffect::new(
            vec!["a".to_string(), "b".to_string()], 
            vec!["b".to_string(), "a".to_string()]
        )),
        "rot" => Some(StackEffect::new(
            vec!["a".to_string(), "b".to_string(), "c".to_string()], 
            vec!["b".to_string(), "c".to_string(), "a".to_string()]
        )),
        "over" => Some(StackEffect::new(
            vec!["a".to_string(), "b".to_string()], 
            vec!["a".to_string(), "b".to_string(), "a".to_string()]
        )),
        "tuck" => Some(StackEffect::new(
            vec!["a".to_string(), "b".to_string()], 
            vec!["b".to_string(), "a".to_string(), "b".to_string()]
        )),
        "nip" => Some(StackEffect::new(
            vec!["a".to_string(), "b".to_string()], 
            vec!["b".to_string()]
        )),
        "pick" => None, // Special case handled separately
        "roll" => None, // Special case handled separately
        
        // Arithmetic operations
        "+" | "add" => Some(StackEffect::new(
            vec!["a".to_string(), "b".to_string()], 
            vec!["sum".to_string()]
        )),
        "-" | "sub" => Some(StackEffect::new(
            vec!["a".to_string(), "b".to_string()], 
            vec!["diff".to_string()]
        )),
        "*" | "mul" => Some(StackEffect::new(
            vec!["a".to_string(), "b".to_string()], 
            vec!["product".to_string()]
        )),
        "/" | "div" => Some(StackEffect::new(
            vec!["a".to_string(), "b".to_string()], 
            vec!["quotient".to_string()]
        )),
        "mod" => Some(StackEffect::new(
            vec!["a".to_string(), "b".to_string()], 
            vec!["remainder".to_string()]
        )),
        "sqrt" => Some(StackEffect::new(
            vec!["a".to_string()], 
            vec!["sqrt".to_string()]
        )),
        
        // Logical operations
        "and" => Some(StackEffect::new(
            vec!["a".to_string(), "b".to_string()], 
            vec!["result".to_string()]
        )),
        "or" => Some(StackEffect::new(
            vec!["a".to_string(), "b".to_string()], 
            vec!["result".to_string()]
        )),
        "not" => Some(StackEffect::new(
            vec!["a".to_string()], 
            vec!["result".to_string()]
        )),
        
        // Comparison operations
        "==" | "eq" => Some(StackEffect::new(
            vec!["a".to_string(), "b".to_string()], 
            vec!["result".to_string()]
        )),
        "!=" => Some(StackEffect::new(
            vec!["a".to_string(), "b".to_string()], 
            vec!["result".to_string()]
        )),
        "<" => Some(StackEffect::new(
            vec!["a".to_string(), "b".to_string()], 
            vec!["result".to_string()]
        )),
        ">" => Some(StackEffect::new(
            vec!["a".to_string(), "b".to_string()], 
            vec!["result".to_string()]
        )),
        "<=" => Some(StackEffect::new(
            vec!["a".to_string(), "b".to_string()], 
            vec!["result".to_string()]
        )),
        ">=" => Some(StackEffect::new(
            vec!["a".to_string(), "b".to_string()], 
            vec!["result".to_string()]
        )),
        
        // Joy-inspired combinators
        "dip" => Some(StackEffect::new(
            vec!["x".to_string(), "quot".to_string()], 
            vec!["quot(x)".to_string()]
        )),
        "bi" => Some(StackEffect::new(
            vec!["x".to_string(), "p".to_string(), "q".to_string()], 
            vec!["p(x)".to_string(), "q(x)".to_string()]
        )),
        "tri" => Some(StackEffect::new(
            vec!["x".to_string(), "p".to_string(), "q".to_string(), "r".to_string()], 
            vec!["p(x)".to_string(), "q(x)".to_string(), "r(x)".to_string()]
        )),
        "keep" => Some(StackEffect::new(
            vec!["x".to_string(), "quot".to_string()], 
            vec!["x".to_string(), "quot(x)".to_string()]
        )),
        "bi*" => Some(StackEffect::new(
            vec!["x".to_string(), "y".to_string(), "p".to_string(), "q".to_string()], 
            vec!["p(x)".to_string(), "q(y)".to_string()]
        )),
        "bi@" => Some(StackEffect::new(
            vec!["x".to_string(), "y".to_string(), "p".to_string()], 
            vec!["p(x)".to_string(), "p(y)".to_string()]
        )),
        
        // Special cases for literals
        _ if word.parse::<i32>().is_ok() => Some(StackEffect::new(
            vec![], 
            vec!["n".to_string()]
        )),
        _ if word.starts_with('"') && word.ends_with('"') => Some(StackEffect::new(
            vec![], 
            vec!["str".to_string()]
        )),
        
        // Unknown word
        _ => None,
    }
}

/// STACKER Algorithm Implementation
/// Enhanced with both Strategy 1 (Peephole Optimization) and Strategy 2 (Usage Tracking)
pub struct StackerTranslator {
    // Map from parameter name to its initial stack depth before the body starts
    param_depths: HashMap<String, usize>,
    // Current stack depth increase caused by operations within the body
    current_stack_depth_increase: isize,
    // The output list of stack operations
    output: Vec<Expr>,
    // Parameter usage tracking for optimization
    param_usage_count: HashMap<String, usize>,
    param_last_use: HashMap<String, usize>,
    // Track which parameters have been consumed already
    consumed_params: Vec<String>,
    // Map of adjusted parameter depths after consumption
    adjusted_param_depths: HashMap<String, isize>,
}

impl StackerTranslator {
    pub fn new() -> Self {
        StackerTranslator {
            param_depths: HashMap::new(),
            current_stack_depth_increase: 0,
            output: Vec::new(),
            param_usage_count: HashMap::new(),
            param_last_use: HashMap::new(),
            consumed_params: Vec::new(),
            adjusted_param_depths: HashMap::new(),
        }
    }

    // Translate a quotation with named parameters to explicit stack operations
    pub fn translate(&mut self, params: &[Param], body: &[Expr]) -> Result<Vec<Expr>> {
        // Reset state
        self.param_depths.clear();
        self.current_stack_depth_increase = 0;
        self.output.clear();
        self.param_usage_count.clear();
        self.param_last_use.clear();
        self.consumed_params.clear();
        self.adjusted_param_depths.clear();
        
        // Step 1: Map parameters to initial stack depths
        // Last parameter (rightmost) is at depth 0, second-to-last at depth 1, etc.
        for (i, param) in params.iter().enumerate().rev() {
            self.param_depths.insert(param.name.clone(), i);
            self.adjusted_param_depths.insert(param.name.clone(), i as isize);
        }
        
        // Step 1.5: Scan the body to count parameter usage and track last use
        self.analyze_parameter_usage(body);
        
        // Step 2: Translate the body expressions with enhanced strategy
        for (index, expr) in body.iter().enumerate() {
            self.translate_expr_enhanced(expr, index)?;
        }
        
        // Step 3: Apply peephole optimizations to the output
        let optimized = self.apply_peephole_optimizations();
        
        Ok(optimized)
    }
    
    // Analyze parameter usage in the body to track usage patterns
    fn analyze_parameter_usage(&mut self, body: &[Expr]) {
        // First pass - count occurrences
        for (index, expr) in body.iter().enumerate() {
            match expr {
                Expr::Symbol(s) => {
                    if self.param_depths.contains_key(s) {
                        // Increment usage count
                        *self.param_usage_count.entry(s.clone()).or_insert(0) += 1;
                        // Update last use index
                        self.param_last_use.insert(s.clone(), index);
                    }
                },
                // Recursively analyze nested expressions
                Expr::Pipeline(left, right) => {
                    self.analyze_parameter_usage(&[*left.clone(), *right.clone()]);
                },
                Expr::Quotation(_, inner_body) => {
                    // For simplicity, we don't track parameter usage across quotation boundaries
                    // A more sophisticated implementation would handle this
                },
                _ => {}
            }
        }
    }
    
    // Translate a single expression with enhanced strategy 2
    fn translate_expr_enhanced(&mut self, expr: &Expr, index: usize) -> Result<()> {
        match expr {
            Expr::Number(n) => {
                // Push the number onto the stack
                self.output.push(Expr::Number(*n));
                self.current_stack_depth_increase += 1;
            },
            Expr::String(s) => {
                // Push the string onto the stack
                self.output.push(Expr::String(s.clone()));
                self.current_stack_depth_increase += 1;
            },
            Expr::Boolean(b) => {
                // Push the boolean onto the stack
                self.output.push(Expr::Boolean(*b));
                self.current_stack_depth_increase += 1;
            },
            Expr::Symbol(s) => {
                // Check if it's a parameter name
                if let Some(&initial_depth) = self.param_depths.get(s) {
                    // Skip if this parameter has already been consumed
                    if self.consumed_params.contains(&s.clone()) {
                        return Err(BorfError::StackEffectError {
                            message: format!("Parameter '{}' has already been consumed and cannot be used again", s),
                            src: None,
                            span: None,
                            help: format!("This parameter was marked as consumed in a previous operation. Parameters can only be consumed once with Strategy 2."),
                        });
                    }
                    
                    // Check if this is the last use of this parameter
                    let is_last_use = self.param_last_use.get(s) == Some(&index);
                    
                    // Get the adjusted depth considering consumed parameters
                    let actual_depth = self.adjusted_param_depths[s] + self.current_stack_depth_increase;
                    
                    if actual_depth < 0 {
                        return Err(BorfError::StackEffectError {
                            message: format!("Invalid stack depth for parameter '{}': {}", s, actual_depth),
                            src: None,
                            span: None,
                            help: format!("This usually happens when stack operations have consumed too many items before the parameter is used. Check the stack effect of operations before this point."),
                        });
                    }
                    
                    // Strategy 2: Consume parameter if it's the last use
                    if is_last_use {
                        // If parameter is at the top of the stack, just consume it (no operation needed)
                        if actual_depth == 0 {
                            // No operation needed - it's already on top
                            // Mark as consumed
                            self.consumed_params.push(s.clone());
                        }
                        // If parameter is just below the top, use swap and then consume
                        else if actual_depth == 1 {
                            self.output.push(Expr::Symbol("swap".to_string()));
                            // swap doesn't change net stack depth
                            // Mark as consumed
                            self.consumed_params.push(s.clone());
                        }
                        // If parameter is deeper, use roll to bring to top and consume
                        else if actual_depth > 1 {
                            if actual_depth <= 3 {
                                // For depths <= 3, use rot or specific roll combinations
                                if actual_depth == 2 {
                                    self.output.push(Expr::Symbol("rot".to_string()));
                                } else { // depth == 3
                                    // rot works on top 3 items, so we'd need 2 rots for depth 3
                                    // For simplicity use roll with depth marker
                                    self.output.push(Expr::Number(actual_depth as i32));
                                    self.output.push(Expr::Symbol("roll".to_string()));
                                }
                            } else {
                                // Use roll for deeper items
                                self.output.push(Expr::Number(actual_depth as i32));
                                self.output.push(Expr::Symbol("roll".to_string()));
                            }
                            // Mark as consumed
                            self.consumed_params.push(s.clone());
                        }
                        
                        // Update adjusted depths for all remaining parameters
                        // When we consume a parameter, all deeper parameters move up by 1
                        for (param, depth) in self.adjusted_param_depths.iter_mut() {
                            if *depth > actual_depth {
                                *depth -= 1;
                            }
                        }
                    }
                    // Not the last use, so use pick to copy the parameter
                    else {
                        // Generate "N pick" operation
                        self.output.push(Expr::Number(actual_depth as i32));
                        self.output.push(Expr::Symbol("pick".to_string()));
                        // pick adds an item to the stack
                        self.current_stack_depth_increase += 1;
                    }
                } else {
                    // Regular word - look up its stack effect
                    let stack_effect = get_word_effect(s).ok_or_else(|| BorfError::StackEffectError {
                        message: format!("Unknown word '{}' with no stack effect declaration", s),
                        src: None,
                        span: None,
                        help: format!("Make sure '{}' is a valid Borf word or declare its stack effect.", s),
                    })?;
                    
                    // Special case for pick and roll
                    if s == "pick" || s == "roll" {
                        // The actual depth difference depends on the value on top of the stack
                        if let Some(Expr::Number(_)) = self.output.last() {
                            // pick consumes the depth but produces a copy of the item at that depth
                            // roll consumes the depth and moves the item
                            self.current_stack_depth_increase += 0; // -1 (for the depth) + 1 (for the result)
                        } else {
                            // If we can't determine the depth, use a default
                            self.current_stack_depth_increase += 0; // Assume no net change
                        }
                    } else {
                        // Update stack depth based on the word's effect
                        self.current_stack_depth_increase += stack_effect.stack_depth_change();
                    }
                    
                    // Add the word to the output
                    self.output.push(Expr::Symbol(s.clone()));
                }
            },
            Expr::Pipeline(left, right) => {
                // Handle pipeline by translating the left side, then the right
                // The |> operator is just syntactic sugar and doesn't translate to any operation
                self.translate_expr_enhanced(left, index)?;
                self.translate_expr_enhanced(right, index)?;
            },
            Expr::Quotation(inner_params, inner_body) => {
                // For nested quotations, we need to store the current state
                let saved_params = self.param_depths.clone();
                let saved_adjusted_depths = self.adjusted_param_depths.clone();
                let saved_consumed = self.consumed_params.clone();
                let saved_depth = self.current_stack_depth_increase;
                
                // Start a quotation
                self.output.push(Expr::Symbol("[".to_string()));
                
                // Translate the inner quotation if it has parameters
                if !inner_params.is_empty() {
                    let mut inner_translator = StackerTranslator::new();
                    match inner_translator.translate(inner_params, inner_body) {
                        Ok(translated_body) => {
                            // Add the translated body to our output
                            self.output.extend(translated_body);
                        },
                        Err(e) => return Err(e)
                    }
                } else {
                    // No parameters, just add the body as is
                    for (inner_index, expr) in inner_body.iter().enumerate() {
                        self.translate_expr_enhanced(expr, inner_index)?;
                    }
                }
                
                // End the quotation
                self.output.push(Expr::Symbol("]".to_string()));
                
                // A quotation is a single item on the stack
                self.current_stack_depth_increase += 1;
                
                // Restore the outer quotation state
                self.param_depths = saved_params;
                self.adjusted_param_depths = saved_adjusted_depths;
                self.consumed_params = saved_consumed;
                // Note: we keep the updated stack depth increase
            },
            // Handle other expression types as needed
            _ => {
                return Err(BorfError::StackEffectError {
                    message: format!("Unsupported expression in translation: {:?}", expr),
                    src: None,
                    span: None,
                    help: "The STACKER algorithm currently only supports basic expressions like literals, symbols, and quotations.".to_string(),
                });
            }
        }
        
        Ok(())
    }
    
    // Apply peephole optimizations to the translated output
    fn apply_peephole_optimizations(&self) -> Vec<Expr> {
        if self.output.is_empty() {
            return Vec::new();
        }
        
        let mut optimized = Vec::new();
        let mut i = 0;
        
        while i < self.output.len() {
            // Pattern: 1 pick 1 pick + -> +
            // (If we have two items x and y on top of the stack, just apply the operator)
            if i + 4 <= self.output.len() && 
                is_expr_number(&self.output[i], 1) && 
                is_expr_symbol(&self.output[i+1], "pick") && 
                is_expr_number(&self.output[i+2], 1) && 
                is_expr_symbol(&self.output[i+3], "pick") && 
                i + 4 < self.output.len() && 
                is_binary_op(&self.output[i+4]) {
                    // Skip the picks and just add the binary operator
                    optimized.push(self.output[i+4].clone());
                    i += 5;
            }
            // Pattern: 0 pick 1 pick + -> swap +
            // (If we need to swap the order of the top two items)
            else if i + 4 <= self.output.len() && 
                is_expr_number(&self.output[i], 0) && 
                is_expr_symbol(&self.output[i+1], "pick") && 
                is_expr_number(&self.output[i+2], 1) && 
                is_expr_symbol(&self.output[i+3], "pick") && 
                i + 4 < self.output.len() && 
                is_binary_op(&self.output[i+4]) {
                    // Replace with swap + binary operator
                    optimized.push(Expr::Symbol("swap".to_string()));
                    optimized.push(self.output[i+4].clone());
                    i += 5;
            }
            // Pattern: 0 pick <op> -> <op>
            // (If we're performing an operation on the top item)
            else if i + 2 <= self.output.len() && 
                is_expr_number(&self.output[i], 0) && 
                is_expr_symbol(&self.output[i+1], "pick") && 
                i + 2 < self.output.len() && 
                is_unary_op(&self.output[i+2]) {
                    // Skip the pick and just add the unary operator
                    optimized.push(self.output[i+2].clone());
                    i += 3;
            }
            // Pattern: 1 pick drop -> nip
            // (Copy second item then drop it? Just use nip)
            else if i + 2 <= self.output.len() && 
                is_expr_number(&self.output[i], 1) && 
                is_expr_symbol(&self.output[i+1], "pick") && 
                i + 2 < self.output.len() && 
                is_expr_symbol(&self.output[i+2], "drop") {
                    // Replace with nip
                    optimized.push(Expr::Symbol("nip".to_string()));
                    i += 3;
            }
            // Pattern: 0 pick drop -> drop
            // (Copy top item then drop it? Just drop it)
            else if i + 2 <= self.output.len() && 
                is_expr_number(&self.output[i], 0) && 
                is_expr_symbol(&self.output[i+1], "pick") && 
                i + 2 < self.output.len() && 
                is_expr_symbol(&self.output[i+2], "drop") {
                    // Replace with drop
                    optimized.push(Expr::Symbol("drop".to_string()));
                    i += 3;
            }
            // Pattern: 1 pick (as last word) -> drop
            // (If the last thing we do is copy the second item to the top)
            else if i + 2 <= self.output.len() && 
                is_expr_number(&self.output[i], 1) && 
                is_expr_symbol(&self.output[i+1], "pick") && 
                i + 2 == self.output.len() {
                    // Replace with drop (to discard the top item, leaving the second one)
                    optimized.push(Expr::Symbol("swap".to_string()));
                    optimized.push(Expr::Symbol("drop".to_string()));
                    i += 2;
            }
            // Pattern: 0 pick (as last word) -> no-op
            // (If the last thing we do is copy the top item to the top - redundant)
            else if i + 2 <= self.output.len() && 
                is_expr_number(&self.output[i], 0) && 
                is_expr_symbol(&self.output[i+1], "pick") && 
                i + 2 == self.output.len() {
                    // Skip it entirely - the item is already on top
                    i += 2;
            }
            // Pattern: roll roll -> roll2 (hypothetical combined operation)
            // Could create more optimizations like this if needed
            
            // No optimization applies, copy as is
            else {
                optimized.push(self.output[i].clone());
                i += 1;
            }
        }
        
        optimized
    }
}

// Helper functions for pattern matching in peephole optimization
fn is_expr_number(expr: &Expr, value: i32) -> bool {
    match expr {
        Expr::Number(n) => *n == value,
        _ => false,
    }
}

fn is_expr_symbol(expr: &Expr, name: &str) -> bool {
    match expr {
        Expr::Symbol(s) => s == name,
        _ => false,
    }
}

fn is_binary_op(expr: &Expr) -> bool {
    match expr {
        Expr::Symbol(s) => {
            matches!(s.as_str(), "+" | "-" | "*" | "/" | "mod" | "==" | "!=" | "<" | ">" | "<=" | ">=" | "and" | "or")
        },
        _ => false,
    }
}

fn is_unary_op(expr: &Expr) -> bool {
    match expr {
        Expr::Symbol(s) => {
            matches!(s.as_str(), "not" | "sqrt" | "sin" | "cos" | "tan" | "abs" | "neg")
        },
        _ => false,
    }
}

/// Translate a named parameter quotation to explicit stack operations
pub fn translate_quotation(params: &[Param], body: &[Expr]) -> Result<Vec<Expr>> {
    let mut translator = StackerTranslator::new();
    translator.translate(params, body)
}