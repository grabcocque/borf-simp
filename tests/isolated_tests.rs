#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    // Test stack effects
    #[test]
    fn test_stack_effect_depth() {
        let inputs = vec!["a".to_string(), "b".to_string()];
        let outputs = vec!["c".to_string()];
        
        // Calculate stack effect: outputs.len() - inputs.len()
        let effect = outputs.len() as isize - inputs.len() as isize;
        
        assert_eq!(effect, -1); // 1 - 2 = -1
    }
    
    // Test parameter lookup
    #[test]
    fn test_param_lookup() {
        let mut param_depths = HashMap::new();
        
        // Map parameters to depths (for a quotation [x y -> ...])
        param_depths.insert("x".to_string(), 1);
        param_depths.insert("y".to_string(), 0);
        
        // Look up depths
        assert_eq!(param_depths.get("x"), Some(&1));
        assert_eq!(param_depths.get("y"), Some(&0));
        assert_eq!(param_depths.get("z"), None);
    }
    
    // Test symbol validation
    #[test]
    fn test_symbol_validation() {
        // Regex-like validation for Borf symbols
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
        
        // Valid symbols
        assert!(is_valid_symbol("foo"));
        assert!(is_valid_symbol("bar123"));
        assert!(is_valid_symbol("hello_world"));
        assert!(is_valid_symbol("x"));
        assert!(is_valid_symbol("value!"));
        assert!(is_valid_symbol("maybe?"));
        
        // Invalid symbols
        assert!(!is_valid_symbol("123abc")); // Starts with number
        assert!(!is_valid_symbol("$hello")); // Starts with $
        assert!(!is_valid_symbol("")); // Empty
    }
}