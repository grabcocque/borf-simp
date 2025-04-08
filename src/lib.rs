// This is the library portion of the Borf implementation
// It exposes core functionality to be used by the main program and REPL

mod calculator;
pub mod repl;
pub mod test_helper;

// Re-export the calculator functionality for WebAssembly
use std::cell::RefCell;
use crate::calculator::exports::vscode::example::types::{ Guest, GuestEngine, Operation };

struct EngineImpl {
    left: Option<u32>,
    right: Option<u32>,
}

impl EngineImpl {
    fn new() -> Self {
        EngineImpl {
            left: None,
            right: None,
        }
    }

    fn push_operand(&mut self, operand: u32) {
        if self.left == None {
            self.left = Some(operand);
        } else {
            self.right = Some(operand);
        }
    }

    fn push_operation(&mut self, operation: Operation) {
        let left = self.left.unwrap();
        let right = self.right.unwrap();
        self.left = Some(match operation {
            Operation::Add => left + right,
            Operation::Sub => left - right,
            Operation::Mul => left * right,
            Operation::Div => left / right,
        });
    }

    fn execute(&mut self) -> u32 {
        self.left.unwrap()
    }
}

struct CalcEngine {
    stack: RefCell<EngineImpl>,
}

impl GuestEngine for CalcEngine {
    fn new() -> Self {
        CalcEngine {
            stack: RefCell::new(EngineImpl::new())
        }
    }

    fn push_operand(&self, operand: u32) {
        self.stack.borrow_mut().push_operand(operand);
    }

    fn push_operation(&self, operation: Operation) {
        self.stack.borrow_mut().push_operation(operation);
    }

    fn execute(&self) -> u32 {
        return self.stack.borrow_mut().execute();
    }
}

struct Implementation;
impl Guest for Implementation {
    type Engine = CalcEngine;
}

calculator::export!(Implementation with_types_in calculator);

// Expose the core Borf structure and functions from main.rs
pub use crate::repl::interpreter::Evaluator;