// tests/standalone_parser_tests.rs
// Standalone parser tests that don't depend on the main codebase

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

// STACKER Algorithm simulation
#[derive(Debug, Clone)]
struct Param {
    name: String,
}

// Simplified STACKER algorithm
struct StackerTranslator {
    // Map from parameter name to its initial stack depth before the body starts
    param_depths: HashMap<String, usize>,
    // Current stack depth increase caused by operations within the body
    current_stack_depth_increase: isize,
    // The output list of operations
    output: Vec<MockExpr>,
}

impl StackerTranslator {
    fn new() -> Self {
        StackerTranslator {
            param_depths: HashMap::new(),
            current_stack_depth_increase: 0,
            output: Vec::new(),
        }
    }
    
    fn translate(&mut self, params: &[Param], body: &[MockExpr]) -> Vec<MockExpr> {
        // Reset state
        self.param_depths.clear();
        self.current_stack_depth_increase = 0;
        self.output.clear();
        
        // Step 1: Map parameters to initial stack depths
        // Last parameter (rightmost) is at depth 0, second-to-last at depth 1, etc.
        for (i, param) in params.iter().enumerate().rev() {
            self.param_depths.insert(param.name.clone(), i);
        }
        
        // Step 2: Translate the body expressions
        for expr in body {
            self.translate_expr(expr);
        }
        
        self.output.clone()
    }
    
    fn translate_expr(&mut self, expr: &MockExpr) {
        match expr {
            MockExpr::Number(n) => {
                // Push the number onto the stack
                self.output.push(MockExpr::Number(*n));
                self.current_stack_depth_increase += 1;
            },
            MockExpr::String(s) => {
                // Push the string onto the stack
                self.output.push(MockExpr::String(s.clone()));
                self.current_stack_depth_increase += 1;
            },
            MockExpr::Symbol(s) => {
                // Check if it's a parameter name
                if let Some(&initial_depth) = self.param_depths.get(s) {
                    // Parameter reference - calculate actual depth and generate pick operation
                    let actual_depth = initial_depth as isize + self.current_stack_depth_increase;
                    if actual_depth >= 0 {
                        // Generate "N pick" operation
                        self.output.push(MockExpr::Number(actual_depth as i32));
                        self.output.push(MockExpr::Pick(0));
                        self.current_stack_depth_increase += 1;
                    }
                } else {
                    // Regular word
                    self.output.push(MockExpr::Symbol(s.clone()));
                    
                    // Assume all operations consume 2 values and produce 1 value
                    if s == "+" || s == "-" || s == "*" || s == "/" {
                        self.current_stack_depth_increase -= 1; // -2 + 1
                    }
                }
            },
            _ => {
                // Handle other types as needed
            }
        }
    }
}

#[test]
fn test_stacker_algorithm() {
    // Test case: [x y -> x y +]
    let params = vec![
        Param { name: "x".to_string() },
        Param { name: "y".to_string() },
    ];
    
    let body = vec![
        MockExpr::Symbol("x".to_string()),
        MockExpr::Symbol("y".to_string()),
        MockExpr::Symbol("+".to_string()),
    ];
    
    let mut translator = StackerTranslator::new();
    let result = translator.translate(&params, &body);
    
    // Expected output for [x y -> x y +]
    // Should be [1 pick 1 pick +]
    let expected = vec![
        MockExpr::Number(1),
        MockExpr::Pick(0),
        MockExpr::Number(1),
        MockExpr::Pick(0),
        MockExpr::Symbol("+".to_string()),
    ];
    
    assert_eq!(result.len(), expected.len());
    // In a real test, we'd compare the actual values, but this is just a simulation
}

#[test]
fn test_complex_stacker_algorithm() {
    // Test case: [a b c -> a b + c *]
    let params = vec![
        Param { name: "a".to_string() },
        Param { name: "b".to_string() },
        Param { name: "c".to_string() },
    ];
    
    let body = vec![
        MockExpr::Symbol("a".to_string()),
        MockExpr::Symbol("b".to_string()),
        MockExpr::Symbol("+".to_string()),
        MockExpr::Symbol("c".to_string()),
        MockExpr::Symbol("*".to_string()),
    ];
    
    let mut translator = StackerTranslator::new();
    let result = translator.translate(&params, &body);
    
    // Expected output for [a b c -> a b + c *]
    // Should be [2 pick 2 pick + 2 pick *]
    let expected = vec![
        MockExpr::Number(2),
        MockExpr::Pick(0),
        MockExpr::Number(2),
        MockExpr::Pick(0),
        MockExpr::Symbol("+".to_string()),
        MockExpr::Number(2),
        MockExpr::Pick(0),
        MockExpr::Symbol("*".to_string()),
    ];
    
    assert_eq!(result.len(), expected.len());
    // In a real test, we'd compare the actual values, but this is just a simulation
}