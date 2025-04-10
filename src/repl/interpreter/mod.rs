// src/repl/interpreter/mod.rs
// This module provides the interpreter for the Borf language

mod types;
mod env;
mod parser;
mod evaluator;
mod stack_effects;
mod effects;

// Re-export the public types
pub use types::{Env, EvaluatorError, Expr, Param, Pattern, Result, Type, TypeParam, Value};
pub use parser::Parser;
pub use evaluator::Evaluator;
pub use stack_effects::{StackEffect, get_word_effect};
pub use effects::{EffectType, ResourceManager, ResourceValue};