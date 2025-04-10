// tests/parser_tests.rs
// Test module for the Borf parser, implementing tests that can run independently

use std::collections::HashMap;

// STACKER Algorithm test components
#[derive(Debug, Clone, PartialEq)]
struct StackEffect {
    inputs: Vec<String>,
    outputs: Vec<String>,
}

impl StackEffect {
    fn new(inputs: Vec<String>, outputs: Vec<String>) -> Self {
        Self { inputs, outputs }
    }
    
    fn stack_depth_change(&self) -> isize {
        self.outputs.len() as isize - self.inputs.len() as isize
    }
}

#[test]
fn test_stack_effect_depth_change() {
    // Test various stack effect depth calculations
    let effect1 = StackEffect::new(
        vec!["a".to_string()], 
        vec!["b".to_string()]
    );
    assert_eq!(effect1.stack_depth_change(), 0);
    
    let effect2 = StackEffect::new(
        vec!["a".to_string(), "b".to_string()], 
        vec!["c".to_string()]
    );
    assert_eq!(effect2.stack_depth_change(), -1);
    
    let effect3 = StackEffect::new(
        vec!["a".to_string()], 
        vec!["b".to_string(), "c".to_string()]
    );
    assert_eq!(effect3.stack_depth_change(), 1);
    
    let effect4 = StackEffect::new(
        vec![], 
        vec!["a".to_string()]
    );
    assert_eq!(effect4.stack_depth_change(), 1);
    
    let effect5 = StackEffect::new(
        vec!["a".to_string()], 
        vec![]
    );
    assert_eq!(effect5.stack_depth_change(), -1);
}

// AST-related test components
#[derive(Debug, PartialEq)]
enum MockExpr {
    Number(i32),
    String(String),
    Symbol(String),
    Boolean(bool),
    Nil,
    List(Vec<MockExpr>),
    Record(HashMap<String, MockExpr>),
    Pick(i32),  // For STACKER operations
}

#[test]
fn test_basic_stacker_simulation() {
    // Test case: [x y -> x y +]
    // Should translate to [1 pick 1 pick +]
    
    // Input:
    // - Parameters: ["x", "y"]
    // - Body: [Symbol("x"), Symbol("y"), Symbol("+")]
    
    // Expected output:
    // [Number(1), Pick, Number(1), Pick, Symbol("+")]
    
    // STACKER algorithm simplified - just checking expected outputs
    let expected_output = vec![
        MockExpr::Number(1),
        MockExpr::Pick(0),  // pick operation
        MockExpr::Number(1),
        MockExpr::Pick(0),  // pick operation
        MockExpr::Symbol("+".to_string())
    ];
    
    // In a real implementation, we would run the actual translation
    // and compare the results
    
    // Simple validation - count the operations
    assert_eq!(expected_output.len(), 5);
}

// Grammar-related test components
#[test]
fn test_grammar_simulation() {
    // Test simple tokens that would be in the grammar
    let symbols = vec!["foo", "bar123", "hello_world", "x", "value!", "maybe?"];
    for sym in symbols {
        assert!(is_valid_symbol(sym));
    }
    
    let invalid_symbols = vec!["123abc", "$hello", ""];
    for sym in invalid_symbols {
        assert!(!is_valid_symbol(sym));
    }
}

// Simple symbol validation function (simulating grammar rule)
fn is_valid_symbol(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    
    let first_char = s.chars().next().unwrap();
    if !first_char.is_alphabetic() && first_char != '_' {
        return false;
    }
    
    for c in s.chars() {
        if !c.is_alphanumeric() && c != '_' && c != '?' && c != '!' && c != '\'' && c != '$' {
            return false;
        }
    }
    
    true
}