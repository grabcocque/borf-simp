# Parser Fixes and Testing Framework

This document outlines the issues that need to be fixed in the Borf parser implementation and provides a comprehensive testing framework to verify the parser once the fixes are applied.

## Current Issues

The following issues need to be fixed before the parser can work correctly:

1. **Left Recursion in Grammar Rules**: The grammar has several rules with left recursion, which is not supported by Pest. The affected rules include:
   - `pipeline`
   - `match_expr`
   - `if_expr`
   - `expr` (due to its reference to `pipeline`)

2. **Module Import Issues**: The error module is not properly imported:
   ```rust
   use crate::repl::interpreter::errors::{BorfError, BorfSpan, Result};
   ```

3. **Rule Type Definition**: The `Rule` type is not properly defined or accessible in several functions.

## Fixing Left Recursion

To fix left recursion in the grammar, we need to use Pest's Pratt parser for handling operator precedence. Here's the modified approach:

1. Change the grammar to eliminate left recursion:
   ```pest
   // For expressions with precedence
   expr = { term ~ (binary_op ~ term)* }
   term = { 
       number | string_literal | symbol | quotation | record_expr | tuple_expr |
       quoted_expr | unquoted_expr | quasiquoted_expr | stack_effect |
       "(" ~ expr ~ ")"
   }
   binary_op = { "|>" | "+" | "-" | "*" | "/" | "==" | "!=" | "<" | ">" | "<=" | ">=" }
   ```

2. Then handle operator precedence in the parser implementation using `PrattParser`.

## Testing Framework Structure

The testing framework consists of multiple components to test different aspects of the parser:

### 1. Grammar Tests (`grammar_tester.rs`)

Tests individual grammar rules to ensure they can correctly parse valid inputs:

```rust
fn test_rule(rule: Rule, input: &str, expect_success: bool) {
    let result = parse_test_str(rule, input);
    assert_eq!(
        result, 
        expect_success, 
        "Testing '{}' with rule {:?}\nExpected: {}, Got: {}",
        input, rule, expect_success, result
    );
}
```

### 2. AST Tests (`ast_tester.rs`)

Tests that parsing produces the expected AST:

```rust
fn test_ast(input: &str, expected_expr: Expr) {
    match parse(input) {
        Ok(expr) => {
            assert_eq!(
                expr, 
                expected_expr, 
                "Parsing '{}' produced unexpected AST.\nExpected: {:?}\nGot: {:?}",
                input, expected_expr, expr
            );
        },
        Err(e) => {
            panic!("Failed to parse valid input '{}': {}", input, e);
        }
    }
}
```

### 3. STACKER Tests (`stacker_tester.rs`)

Tests the STACKER algorithm that translates named parameters to stack operations:

```rust
fn test_stacker_translation(params: Vec<&str>, body: Vec<Expr>, expected: Vec<Expr>) {
    // Convert string parameters to Param structs
    let params: Vec<Param> = params.into_iter()
        .map(|name| Param {
            name: name.to_string(),
            type_annotation: None,
        })
        .collect();

    // Translate the quotation
    let result = translate_quotation(&params, &body).expect("STACKER translation failed");
    
    // Compare the result with the expected output
    assert_eq!(
        result, 
        expected,
        "STACKER translation produced unexpected result.\nExpected: {:?}\nGot: {:?}",
        expected, result
    );
}
```

### 4. Integration Tests (`integration_tests.rs`)

Tests complete programs from parsing through STACKER translation:

```rust
fn test_program(input: &str, expected_expr: Expr) {
    match parse(input) {
        Ok(expr) => {
            assert_eq!(
                expr, 
                expected_expr, 
                "Parsing '{}' produced unexpected AST.\nExpected: {:?}\nGot: {:?}",
                input, expected_expr, expr
            );
        },
        Err(e) => {
            panic!("Failed to parse valid input '{}': {}", input, e);
        }
    }
}
```

## Example Test Cases

### Grammar Test Cases
```rust
#[test]
fn test_basic_literals() {
    should_parse(Rule::number, "42");
    should_parse(Rule::number, "-42");
    should_parse(Rule::number, "3.14");
    should_not_parse(Rule::number, "abc");
}
```

### AST Test Cases
```rust
#[test]
fn test_simple_literals() {
    test_ast("42", Expr::Number(42));
    test_ast("-42", Expr::Number(-42));
    test_ast("\"hello\"", Expr::String("hello".to_string()));
}
```

### STACKER Test Cases
```rust
#[test]
fn test_simple_parameter_translation() {
    // Test case: [x -> x]
    // Should translate to [0 pick]
    test_stacker_translation(
        vec!["x"],
        vec![Expr::Symbol("x".to_string())],
        vec![
            Expr::Number(0),
            Expr::Symbol("pick".to_string())
        ]
    );
}
```

## Next Steps

1. Fix the grammar to eliminate left recursion
2. Fix the error module imports
3. Properly define the `Rule` type
4. Implement the testing framework
5. Run tests incrementally, starting with the simplest rules
6. Add more complex test cases as the parser implementation improves

This approach allows for incremental testing of the parser, focusing on individual components before testing the entire system.