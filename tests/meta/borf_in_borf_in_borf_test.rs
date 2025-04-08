// Borf-in-Borf-in-Borf Test Runner
// This file tests whether the metacircular evaluator can evaluate itself in 3 layers

use std::path::Path;
use borf::repl::interpreter::{Evaluator, Result, EvaluatorError};

/// Run the Borf-in-Borf-in-Borf test
pub fn run_borf_in_borf_in_borf_test() -> Result<()> {
    println!("Running Borf-in-Borf-in-Borf Test");
    println!("=================================");
    println!("This test verifies that the Borf-in-Borf evaluator can evaluate itself and then evaluate Borf code");
    
    // Create a clean evaluator
    let mut evaluator = Evaluator::new();
    
    // Initialize with basics but without the meta modules
    // We need to bootstrap the system properly
    evaluator.initialize()?;
    
    // Define the test file path
    let test_file = Path::new("tests/meta/borf_in_borf_in_borf_test.borf");
    if !test_file.exists() {
        return Err(EvaluatorError::FileError(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Test file not found at {}", test_file.display())
        )));
    }
    
    // Run the test
    match evaluator.eval_file(test_file) {
        Ok(result) => {
            // If the test returns "true", it passed
            let passed = result.trim() == "true";
            
            if passed {
                println!("Borf-in-Borf-in-Borf test passed! The metacircular evaluator successfully evaluated itself through multiple layers.");
                Ok(())
            } else {
                eprintln!("Borf-in-Borf-in-Borf test failed. The metacircular evaluator may not be able to evaluate itself completely.");
                Err(EvaluatorError::EvalError("Test failed".to_string()))
            }
        },
        Err(err) => {
            eprintln!("Error running Borf-in-Borf-in-Borf test: {}", err);
            Err(err)
        }
    }
}

/// Main function - run it directly for testing
#[allow(dead_code)]
fn main() -> Result<()> {
    run_borf_in_borf_in_borf_test()
}