// tests/parser/grammar_tester.rs
// Tests for the Borf grammar using Pest - Skeleton file

use std::fs;
use std::path::Path;

/// Parse a string with the specified rule
fn parse_test_str(rule: &str, input: &str) -> bool {
    // This is a placeholder that will be replaced once the parser is fixed
    println!("Testing rule '{}' with input '{}'", rule, input);
    true
}

/// Test helper to check if an input successfully parses with a given rule
fn test_rule(rule: &str, input: &str, expect_success: bool) {
    let result = parse_test_str(rule, input);
    assert_eq!(
        result, 
        expect_success, 
        "Testing '{}' with rule {}\nExpected: {}, Got: {}",
        input, rule, expect_success, result
    );
}

/// Test helper for positive matches
fn should_parse(rule: &str, input: &str) {
    test_rule(rule, input, true);
}

/// Test helper for negative matches
fn should_not_parse(rule: &str, input: &str) {
    test_rule(rule, input, false);
}

#[test]
fn test_basic_parsing() {
    // This is a placeholder test that will be replaced when the parser is fixed
    assert!(true, "Basic parsing test");
}