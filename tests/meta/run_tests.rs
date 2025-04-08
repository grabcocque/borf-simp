// Metacircular Evaluator Test Runner
// A Rust program to run the Borf metacircular evaluator tests

use std::env;
use std::fs;
use std::path::Path;
use std::process;
use borf_lib::repl::interpreter::{Evaluator, Result};

fn run_test_file(evaluator: &mut Evaluator, path: &Path) -> Result<bool> {
    if !path.exists() {
        eprintln!("Error: Test file not found at {}", path.display());
        process::exit(1);
    }

    println!("\nRunning tests from {}", path.display());
    
    match evaluator.eval_file(&path) {
        Ok(result) => {
            // Check if all tests passed (result should be "true")
            let passed = result.trim() == "true";
            if passed {
                println!("All tests in {} passed!", path.display());
            } else {
                eprintln!("Some tests in {} failed.", path.display());
            }
            Ok(passed)
        },
        Err(err) => {
            eprintln!("Error running tests from {}: {}", path.display(), err);
            Ok(false)
        }
    }
}

fn main() -> Result<()> {
    println!("Borf Metacircular Evaluator Test Runner");
    println!("======================================");

    // Create a new evaluator
    let mut evaluator = Evaluator::new();
    evaluator.initialize()?;

    // Define test files to run
    let test_files = vec![
        "tests/meta/self_eval_tests.borf",
        "tests/meta/borf_in_borf_in_borf_test.borf"
    ];
    
    let mut all_passed = true;
    
    // Run all test files
    for test_file in test_files {
        let test_path = Path::new(test_file);
        match run_test_file(&mut evaluator, test_path) {
            Ok(passed) => {
                if !passed {
                    all_passed = false;
                }
            },
            Err(err) => {
                eprintln!("Error executing test file {}: {}", test_file, err);
                all_passed = false;
            }
        }
    }
    
    println!("\n===================================");
    if all_passed {
        println!("All metacircular evaluator tests passed!");
        Ok(())
    } else {
        eprintln!("Some metacircular evaluator tests failed.");
        process::exit(1);
    }
}