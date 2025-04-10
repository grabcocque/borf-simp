// src/repl/interpreter/parser.rs
// This module provides the parser for the Borf interpreter using pest

use std::collections::HashMap;
use pest::Parser;
use pest::iterators::{Pair, Pairs};
use pest::pratt_parser::{PrattParser, Assoc, Op};
use pest_derive::Parser;

use crate::repl::interpreter::errors::{BorfError, BorfSpan, Result};
use crate::repl::interpreter::types::{Expr, Param, Pattern, Type, Value};
use crate::repl::interpreter::stack_effects::{StackEffect, parse_stack_effect, translate_quotation};

#[derive(Parser)]
#[grammar = "repl/interpreter/borf.pest"]
pub struct BorfParser;

// Define the grammar rule enum so it can be referenced in the code
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Rule {
    // Top-level rules
    program, module_decl, import_decl, top_level_expr,
    
    // Expression rules
    expr, atom, infix_op,
    
    // Literal rules
    number, string_literal, escape_sequence, symbol, reserved,
    
    // Quotation rules
    quotation, params, param,
    
    // Assignment
    assignment,
    
    // Match expression
    match_block, pattern_case, pattern, record_pattern, field_pattern, quoted_pattern,
    
    // If expression
    if_branches,
    
    // Record and tuple expressions
    record_expr, field_expr, tuple_expr,
    
    // Meta-programming
    quoted_expr, unquoted_expr, quasiquoted_expr,
    
    // Stack effect
    stack_effect, stack_inputs, stack_outputs, stack_item,
    
    // Special rules
    WHITESPACE, COMMENT, EOI,
}

pub struct PestParser {
    source: String,
    pratt_parser: PrattParser<Rule>, // Pratt parser for handling operators with precedence
}

impl PestParser {
    pub fn new(input: &str) -> Self {
        // Define operator precedence and associativity
        let pratt = PrattParser::new()
            // Pipeline operator (highest precedence, left associative)
            .op(Op::infix(Rule::infix_op, Assoc::Left))
            .clone();
            
        PestParser {
            source: input.to_string(),
            pratt_parser: pratt,
        }
    }

    pub fn parse(&self) -> Result<Expr> {
        match BorfParser::parse(Rule::program, &self.source) {
            Ok(mut pairs) => {
                // Get the program node (should be the first and only top level rule)
                let program = pairs.next().unwrap();
                
                // Program should contain a list of expressions
                let mut exprs = Vec::new();
                
                for pair in program.into_inner() {
                    match pair.as_rule() {
                        Rule::top_level_expr => {
                            exprs.push(self.parse_expression(pair.into_inner().next().unwrap())?);
                        },
                        Rule::module_decl => {
                            // Handle module declaration
                            // For now, we just parse it but don't do anything with it
                        },
                        Rule::import_decl => {
                            // Handle import declaration
                            // For now, we just parse it but don't do anything with it
                        },
                        Rule::EOI => {
                            // End of input marker, ignore
                        },
                        _ => {
                            // Unexpected rule
                            return Err(BorfError::ParseError {
                                message: format!("Unexpected rule: {:?}", pair.as_rule()),
                                src: Some(self.source.clone()),
                                span: Some((pair.as_span().start(), pair.as_span().len()).into()),
                            });
                        }
                    }
                }
                
                // For simplicity, if we have a single expression, return it
                // Otherwise, create a sequence/block expression
                if exprs.len() == 1 {
                    Ok(exprs.remove(0))
                } else {
                    // In a concatenative language, multiple expressions at the top level
                    // are just executed in sequence
                    Ok(Expr::Sequence(exprs))
                }
            },
            Err(e) => {
                Err(BorfError::ParseError {
                    message: e.to_string(),
                    src: Some(self.source.clone()),
                    span: None, // Pest doesn't always provide span info for errors
                })
            }
        }
    }

    fn parse_expression(&self, pair: Pair<Rule>) -> Result<Expr> {
        match pair.as_rule() {
            Rule::expr => {
                // Use the Pratt parser to handle operator precedence
                let pairs = pair.into_inner();
                self.pratt_parser.map_primary(|primary| {
                    match primary.as_rule() {
                        Rule::atom => self.parse_atom(primary),
                        unexpected => Err(BorfError::ParseError {
                            message: format!("Expected atom, got {:?}", unexpected),
                            src: Some(self.source.clone()),
                            span: Some((primary.as_span().start(), primary.as_span().len()).into()),
                            help: "This shouldn't happen - internal parser error".to_string(),
                        }),
                    }
                })
                .map_infix(|lhs, op, rhs| {
                    // Handle the different infix operators
                    let op_str = op.into_inner().next().unwrap().as_str(); // Get the actual operator string
                    match op_str {
                        "|>" => {
                            // Pipeline operator
                            Ok(Expr::Pipeline(Box::new(lhs?), Box::new(rhs?)))
                        },
                        "match" => {
                            // Match expression - the right side should be a match block
                            let match_block = rhs?;
                            if let Expr::Match(_, cases) = match_block {
                                Ok(Expr::Match(Box::new(lhs?), cases))
                            } else {
                                Err(BorfError::ParseError {
                                    message: format!("Expected match block, got {:?}", match_block),
                                    src: Some(self.source.clone()),
                                    span: Some((op.as_span().start(), op.as_span().len()).into()),
                                    help: "Match expressions should be in the form: value { | pattern => expr }* match".to_string(),
                                })
                            }
                        },
                        "if" => {
                            // If expression - the right side should contain the branches
                            let branches = rhs?;
                            if let Expr::If(_, true_branch, false_branch) = branches {
                                Ok(Expr::If(Box::new(lhs?), true_branch, false_branch))
                            } else {
                                Err(BorfError::ParseError {
                                    message: format!("Expected if branches, got {:?}", branches),
                                    src: Some(self.source.clone()),
                                    span: Some((op.as_span().start(), op.as_span().len()).into()),
                                    help: "If expressions should be in the form: condition [true_branch] [false_branch] if".to_string(),
                                })
                            }
                        },
                        "times" => {
                            // Times loop - repeat code n times
                            // n [code] times
                            if let Ok(code) = rhs {
                                if let Expr::Quotation(_, _) = code {
                                    Ok(Expr::Times(Box::new(lhs?), Box::new(code)))
                                } else {
                                    Err(BorfError::ParseError {
                                        message: format!("Expected a quotation for times loop body, got {:?}", code),
                                        src: Some(self.source.clone()),
                                        span: Some((op.as_span().start(), op.as_span().len()).into()),
                                        help: "Times loops should be in the form: n [code] times".to_string(),
                                    })
                                }
                            } else {
                                Err(BorfError::ParseError {
                                    message: "Failed to parse 'times' loop body".to_string(),
                                    src: Some(self.source.clone()),
                                    span: Some((op.as_span().start(), op.as_span().len()).into()),
                                    help: "Times loops should be in the form: n [code] times".to_string(),
                                })
                            }
                        },
                        "loop" => {
                            // Infinite loop - [code] loop
                            if let Ok(code) = rhs {
                                if let Expr::Quotation(_, _) = code {
                                    Ok(Expr::Loop(Box::new(code)))
                                } else {
                                    Err(BorfError::ParseError {
                                        message: format!("Expected a quotation for loop body, got {:?}", code),
                                        src: Some(self.source.clone()),
                                        span: Some((op.as_span().start(), op.as_span().len()).into()),
                                        help: "Loops should be in the form: [code] loop".to_string(),
                                    })
                                }
                            } else {
                                Err(BorfError::ParseError {
                                    message: "Failed to parse 'loop' body".to_string(),
                                    src: Some(self.source.clone()),
                                    span: Some((op.as_span().start(), op.as_span().len()).into()),
                                    help: "Loops should be in the form: [code] loop".to_string(),
                                })
                            }
                        },
                        "while" => {
                            // While loop - [condition] [body] while
                            if let (Ok(condition), Ok(body)) = (lhs, rhs) {
                                if let (Expr::Quotation(_, _), Expr::Quotation(_, _)) = (&condition, &body) {
                                    Ok(Expr::While(Box::new(condition), Box::new(body)))
                                } else {
                                    Err(BorfError::ParseError {
                                        message: "Both condition and body must be quotations in while loop".to_string(),
                                        src: Some(self.source.clone()),
                                        span: Some((op.as_span().start(), op.as_span().len()).into()),
                                        help: "While loops should be in the form: [condition] [body] while".to_string(),
                                    })
                                }
                            } else {
                                Err(BorfError::ParseError {
                                    message: "Failed to parse 'while' loop components".to_string(),
                                    src: Some(self.source.clone()),
                                    span: Some((op.as_span().start(), op.as_span().len()).into()),
                                    help: "While loops should be in the form: [condition] [body] while".to_string(),
                                })
                            }
                        },
                        "for" => {
                            // For loop - [range] [body] for or range [body] for
                            if let (Ok(range), Ok(body)) = (lhs, rhs) {
                                if let Expr::Quotation(_, _) = &body {
                                    // We allow either a quotation containing the range or a direct range expression
                                    let range_expr = if let Expr::Quotation(_, _) = &range {
                                        range
                                    } else {
                                        // For non-quotation ranges, we need to handle them specially
                                        // This could be a tuple (start, end) or another iterable
                                        range
                                    };
                                    
                                    // For loops need an iteration variable (i) which is implicit
                                    // We'll create a special form of For that handles this
                                    Ok(Expr::For(Box::new(range_expr), Box::new(body), Box::new(Expr::Nil)))
                                } else {
                                    Err(BorfError::ParseError {
                                        message: "Body must be a quotation in for loop".to_string(),
                                        src: Some(self.source.clone()),
                                        span: Some((op.as_span().start(), op.as_span().len()).into()),
                                        help: "For loops should be in the form: [range] [body] for or range [body] for".to_string(),
                                    })
                                }
                            } else {
                                Err(BorfError::ParseError {
                                    message: "Failed to parse 'for' loop components".to_string(),
                                    src: Some(self.source.clone()),
                                    span: Some((op.as_span().start(), op.as_span().len()).into()),
                                    help: "For loops should be in the form: [range] [body] for or range [body] for".to_string(),
                                })
                            }
                        },
                        
                        // Joy-inspired combinators
                        "dip" => {
                            // Dip - temporarily hide top value, run quotation, restore value
                            if let Ok(quotation) = rhs {
                                if let Expr::Quotation(_, _) = &quotation {
                                    Ok(Expr::Dip(Box::new(quotation)))
                                } else {
                                    Err(BorfError::ParseError {
                                        message: "Expected a quotation for dip".to_string(),
                                        src: Some(self.source.clone()),
                                        span: Some((op.as_span().start(), op.as_span().len()).into()),
                                        help: "Dip should be in the form: a b [Q] dip -> a Q b".to_string(),
                                    })
                                }
                            } else {
                                Err(BorfError::ParseError {
                                    message: "Failed to parse quotation for dip".to_string(),
                                    src: Some(self.source.clone()),
                                    span: Some((op.as_span().start(), op.as_span().len()).into()),
                                    help: "Dip should be in the form: a b [Q] dip -> a Q b".to_string(),
                                })
                            }
                        },
                        "map" => {
                            // Map - apply quotation to each element in a sequence
                            if let (Ok(sequence), Ok(quotation)) = (lhs, rhs) {
                                if let Expr::Quotation(_, _) = &quotation {
                                    Ok(Expr::Map(Box::new(sequence), Box::new(quotation)))
                                } else {
                                    Err(BorfError::ParseError {
                                        message: "Expected a quotation for map".to_string(),
                                        src: Some(self.source.clone()),
                                        span: Some((op.as_span().start(), op.as_span().len()).into()),
                                        help: "Map should be in the form: sequence [Q] map".to_string(),
                                    })
                                }
                            } else {
                                Err(BorfError::ParseError {
                                    message: "Failed to parse components for map".to_string(),
                                    src: Some(self.source.clone()),
                                    span: Some((op.as_span().start(), op.as_span().len()).into()),
                                    help: "Map should be in the form: sequence [Q] map".to_string(),
                                })
                            }
                        },
                        "filter" => {
                            // Filter - keep only elements satisfying predicate
                            if let (Ok(sequence), Ok(predicate)) = (lhs, rhs) {
                                if let Expr::Quotation(_, _) = &predicate {
                                    Ok(Expr::Filter(Box::new(sequence), Box::new(predicate)))
                                } else {
                                    Err(BorfError::ParseError {
                                        message: "Expected a quotation for filter".to_string(),
                                        src: Some(self.source.clone()),
                                        span: Some((op.as_span().start(), op.as_span().len()).into()),
                                        help: "Filter should be in the form: sequence [P] filter".to_string(),
                                    })
                                }
                            } else {
                                Err(BorfError::ParseError {
                                    message: "Failed to parse components for filter".to_string(),
                                    src: Some(self.source.clone()),
                                    span: Some((op.as_span().start(), op.as_span().len()).into()),
                                    help: "Filter should be in the form: sequence [P] filter".to_string(),
                                })
                            }
                        },
                        "fold" => {
                            // Fold - reduce sequence with binary operator
                            // sequence init [F] fold
                            if let Ok(quotation) = rhs {
                                if let Expr::Quotation(_, _) = &quotation {
                                    if let Ok(init_sequence) = lhs {
                                        // We'll need to extract the initial value and sequence
                                        // This is a simplification - in practice we'd need to handle nested expressions
                                        if let Expr::Tuple(elements) = &init_sequence {
                                            if elements.len() == 2 {
                                                let sequence = elements[0].clone();
                                                let initial = elements[1].clone();
                                                Ok(Expr::Fold(Box::new(sequence), Box::new(initial), Box::new(quotation)))
                                            } else {
                                                Err(BorfError::ParseError {
                                                    message: "Expected sequence and initial value for fold".to_string(),
                                                    src: Some(self.source.clone()),
                                                    span: Some((op.as_span().start(), op.as_span().len()).into()),
                                                    help: "Fold should be in the form: sequence init [F] fold".to_string(),
                                                })
                                            }
                                        } else {
                                            // If not a tuple, assume the lhs is the sequence and use a default initial value (nil)
                                            Ok(Expr::Fold(Box::new(init_sequence), Box::new(Expr::Nil), Box::new(quotation)))
                                        }
                                    } else {
                                        Err(BorfError::ParseError {
                                            message: "Failed to parse sequence for fold".to_string(),
                                            src: Some(self.source.clone()),
                                            span: Some((op.as_span().start(), op.as_span().len()).into()),
                                            help: "Fold should be in the form: sequence init [F] fold".to_string(),
                                        })
                                    }
                                } else {
                                    Err(BorfError::ParseError {
                                        message: "Expected a quotation for fold".to_string(),
                                        src: Some(self.source.clone()),
                                        span: Some((op.as_span().start(), op.as_span().len()).into()),
                                        help: "Fold should be in the form: sequence init [F] fold".to_string(),
                                    })
                                }
                            } else {
                                Err(BorfError::ParseError {
                                    message: "Failed to parse components for fold".to_string(),
                                    src: Some(self.source.clone()),
                                    span: Some((op.as_span().start(), op.as_span().len()).into()),
                                    help: "Fold should be in the form: sequence init [F] fold".to_string(),
                                })
                            }
                        },
                        "bi" => {
                            // Bi - apply two quotations to the same value
                            // lhs: x, rhs: [P] [Q]
                            if let (Ok(value), Ok(quotations)) = (lhs, rhs) {
                                if let Expr::Tuple(parts) = &quotations {
                                    if parts.len() == 2 {
                                        let p = parts[0].clone();
                                        let q = parts[1].clone();
                                        if let (Expr::Quotation(_, _), Expr::Quotation(_, _)) = (&p, &q) {
                                            Ok(Expr::Bi(Box::new(value), Box::new(p), Box::new(q)))
                                        } else {
                                            Err(BorfError::ParseError {
                                                message: "Expected two quotations for bi".to_string(),
                                                src: Some(self.source.clone()),
                                                span: Some((op.as_span().start(), op.as_span().len()).into()),
                                                help: "Bi should be in the form: x [P] [Q] bi".to_string(),
                                            })
                                        }
                                    } else {
                                        Err(BorfError::ParseError {
                                            message: "Expected exactly two quotations for bi".to_string(),
                                            src: Some(self.source.clone()),
                                            span: Some((op.as_span().start(), op.as_span().len()).into()),
                                            help: "Bi should be in the form: x [P] [Q] bi".to_string(),
                                        })
                                    }
                                } else {
                                    Err(BorfError::ParseError {
                                        message: "Expected tuple of quotations for bi".to_string(),
                                        src: Some(self.source.clone()),
                                        span: Some((op.as_span().start(), op.as_span().len()).into()),
                                        help: "Bi should be in the form: x [P] [Q] bi".to_string(),
                                    })
                                }
                            } else {
                                Err(BorfError::ParseError {
                                    message: "Failed to parse components for bi".to_string(),
                                    src: Some(self.source.clone()),
                                    span: Some((op.as_span().start(), op.as_span().len()).into()),
                                    help: "Bi should be in the form: x [P] [Q] bi".to_string(),
                                })
                            }
                        },
                        "tri" => {
                            // Tri - apply three quotations to the same value
                            // lhs: x, rhs: [P] [Q] [R]
                            if let (Ok(value), Ok(quotations)) = (lhs, rhs) {
                                if let Expr::Tuple(parts) = &quotations {
                                    if parts.len() == 3 {
                                        let p = parts[0].clone();
                                        let q = parts[1].clone();
                                        let r = parts[2].clone();
                                        if let (Expr::Quotation(_, _), Expr::Quotation(_, _), Expr::Quotation(_, _)) = (&p, &q, &r) {
                                            Ok(Expr::Tri(Box::new(value), Box::new(p), Box::new(q), Box::new(r)))
                                        } else {
                                            Err(BorfError::ParseError {
                                                message: "Expected three quotations for tri".to_string(),
                                                src: Some(self.source.clone()),
                                                span: Some((op.as_span().start(), op.as_span().len()).into()),
                                                help: "Tri should be in the form: x [P] [Q] [R] tri".to_string(),
                                            })
                                        }
                                    } else {
                                        Err(BorfError::ParseError {
                                            message: "Expected exactly three quotations for tri".to_string(),
                                            src: Some(self.source.clone()),
                                            span: Some((op.as_span().start(), op.as_span().len()).into()),
                                            help: "Tri should be in the form: x [P] [Q] [R] tri".to_string(),
                                        })
                                    }
                                } else {
                                    Err(BorfError::ParseError {
                                        message: "Expected tuple of quotations for tri".to_string(),
                                        src: Some(self.source.clone()),
                                        span: Some((op.as_span().start(), op.as_span().len()).into()),
                                        help: "Tri should be in the form: x [P] [Q] [R] tri".to_string(),
                                    })
                                }
                            } else {
                                Err(BorfError::ParseError {
                                    message: "Failed to parse components for tri".to_string(),
                                    src: Some(self.source.clone()),
                                    span: Some((op.as_span().start(), op.as_span().len()).into()),
                                    help: "Tri should be in the form: x [P] [Q] [R] tri".to_string(),
                                })
                            }
                        },
                        "keep" => {
                            // Keep - execute quotation but keep the original value
                            if let Ok(quotation) = rhs {
                                if let Expr::Quotation(_, _) = &quotation {
                                    Ok(Expr::Keep(Box::new(quotation)))
                                } else {
                                    Err(BorfError::ParseError {
                                        message: "Expected a quotation for keep".to_string(),
                                        src: Some(self.source.clone()),
                                        span: Some((op.as_span().start(), op.as_span().len()).into()),
                                        help: "Keep should be in the form: x [Q] keep -> x Q(x)".to_string(),
                                    })
                                }
                            } else {
                                Err(BorfError::ParseError {
                                    message: "Failed to parse quotation for keep".to_string(),
                                    src: Some(self.source.clone()),
                                    span: Some((op.as_span().start(), op.as_span().len()).into()),
                                    help: "Keep should be in the form: x [Q] keep -> x Q(x)".to_string(),
                                })
                            }
                        },
                        "dip2" => {
                            // Dip2 - temporarily hide two values, run quotation, restore values
                            if let Ok(quotation) = rhs {
                                if let Expr::Quotation(_, _) = &quotation {
                                    Ok(Expr::Dip2(Box::new(quotation)))
                                } else {
                                    Err(BorfError::ParseError {
                                        message: "Expected a quotation for dip2".to_string(),
                                        src: Some(self.source.clone()),
                                        span: Some((op.as_span().start(), op.as_span().len()).into()),
                                        help: "Dip2 should be in the form: a b c [Q] dip2 -> a Q b c".to_string(),
                                    })
                                }
                            } else {
                                Err(BorfError::ParseError {
                                    message: "Failed to parse quotation for dip2".to_string(),
                                    src: Some(self.source.clone()),
                                    span: Some((op.as_span().start(), op.as_span().len()).into()),
                                    help: "Dip2 should be in the form: a b c [Q] dip2 -> a Q b c".to_string(),
                                })
                            }
                        },
                        "bi*" => {
                            // Bi* - apply different quotations to different values
                            // lhs: x y, rhs: [P] [Q]
                            if let (Ok(values), Ok(quotations)) = (lhs, rhs) {
                                if let Expr::Tuple(value_parts) = &values {
                                    if value_parts.len() == 2 {
                                        if let Expr::Tuple(quotation_parts) = &quotations {
                                            if quotation_parts.len() == 2 {
                                                let p = quotation_parts[0].clone();
                                                let q = quotation_parts[1].clone();
                                                if let (Expr::Quotation(_, _), Expr::Quotation(_, _)) = (&p, &q) {
                                                    Ok(Expr::BiStar(Box::new(values), Box::new(p), Box::new(q)))
                                                } else {
                                                    Err(BorfError::ParseError {
                                                        message: "Expected two quotations for bi*".to_string(),
                                                        src: Some(self.source.clone()),
                                                        span: Some((op.as_span().start(), op.as_span().len()).into()),
                                                        help: "Bi* should be in the form: x y [P] [Q] bi*".to_string(),
                                                    })
                                                }
                                            } else {
                                                Err(BorfError::ParseError {
                                                    message: "Expected exactly two quotations for bi*".to_string(),
                                                    src: Some(self.source.clone()),
                                                    span: Some((op.as_span().start(), op.as_span().len()).into()),
                                                    help: "Bi* should be in the form: x y [P] [Q] bi*".to_string(),
                                                })
                                            }
                                        } else {
                                            Err(BorfError::ParseError {
                                                message: "Expected tuple of quotations for bi*".to_string(),
                                                src: Some(self.source.clone()),
                                                span: Some((op.as_span().start(), op.as_span().len()).into()),
                                                help: "Bi* should be in the form: x y [P] [Q] bi*".to_string(),
                                            })
                                        }
                                    } else {
                                        Err(BorfError::ParseError {
                                            message: "Expected exactly two values for bi*".to_string(),
                                            src: Some(self.source.clone()),
                                            span: Some((op.as_span().start(), op.as_span().len()).into()),
                                            help: "Bi* should be in the form: x y [P] [Q] bi*".to_string(),
                                        })
                                    }
                                } else {
                                    Err(BorfError::ParseError {
                                        message: "Expected tuple of values for bi*".to_string(),
                                        src: Some(self.source.clone()),
                                        span: Some((op.as_span().start(), op.as_span().len()).into()),
                                        help: "Bi* should be in the form: x y [P] [Q] bi*".to_string(),
                                    })
                                }
                            } else {
                                Err(BorfError::ParseError {
                                    message: "Failed to parse components for bi*".to_string(),
                                    src: Some(self.source.clone()),
                                    span: Some((op.as_span().start(), op.as_span().len()).into()),
                                    help: "Bi* should be in the form: x y [P] [Q] bi*".to_string(),
                                })
                            }
                        },
                        "bi@" => {
                            // Bi@ - apply same quotation to two values
                            // lhs: x y, rhs: [P]
                            if let (Ok(values), Ok(quotation)) = (lhs, rhs) {
                                if let Expr::Tuple(value_parts) = &values {
                                    if value_parts.len() == 2 {
                                        if let Expr::Quotation(_, _) = &quotation {
                                            Ok(Expr::BiAt(Box::new(values), Box::new(quotation)))
                                        } else {
                                            Err(BorfError::ParseError {
                                                message: "Expected a quotation for bi@".to_string(),
                                                src: Some(self.source.clone()),
                                                span: Some((op.as_span().start(), op.as_span().len()).into()),
                                                help: "Bi@ should be in the form: x y [P] bi@".to_string(),
                                            })
                                        }
                                    } else {
                                        Err(BorfError::ParseError {
                                            message: "Expected exactly two values for bi@".to_string(),
                                            src: Some(self.source.clone()),
                                            span: Some((op.as_span().start(), op.as_span().len()).into()),
                                            help: "Bi@ should be in the form: x y [P] bi@".to_string(),
                                        })
                                    }
                                } else {
                                    Err(BorfError::ParseError {
                                        message: "Expected tuple of values for bi@".to_string(),
                                        src: Some(self.source.clone()),
                                        span: Some((op.as_span().start(), op.as_span().len()).into()),
                                        help: "Bi@ should be in the form: x y [P] bi@".to_string(),
                                    })
                                }
                            } else {
                                Err(BorfError::ParseError {
                                    message: "Failed to parse components for bi@".to_string(),
                                    src: Some(self.source.clone()),
                                    span: Some((op.as_span().start(), op.as_span().len()).into()),
                                    help: "Bi@ should be in the form: x y [P] bi@".to_string(),
                                })
                            }
                        },
                        
                        // Advanced stack manipulation operators (amazing Forth names)
                        "nip" => {
                            // Nip - drop the second item on the stack
                            // a b n nip -> b
                            if let (Ok(stack_items), Ok(n)) = (lhs, rhs) {
                                // The n parameter is just for symmetry with the other stack operators
                                // In classic Forth, nip doesn't take an index parameter, but we're making it
                                // consistent with pick and roll for a more uniform interface
                                Ok(Expr::Nip(Box::new(n)))
                            } else {
                                Err(BorfError::ParseError {
                                    message: "Failed to parse components for nip".to_string(),
                                    src: Some(self.source.clone()),
                                    span: Some((op.as_span().start(), op.as_span().len()).into()),
                                    help: "Nip should be in the form: a b n nip".to_string(),
                                })
                            }
                        },
                        "tuck" => {
                            // Tuck - copy top item before second item
                            // a b n tuck -> b a b
                            if let (Ok(stack_items), Ok(n)) = (lhs, rhs) {
                                // Like nip, the n parameter is for symmetry
                                Ok(Expr::Tuck(Box::new(n)))
                            } else {
                                Err(BorfError::ParseError {
                                    message: "Failed to parse components for tuck".to_string(),
                                    src: Some(self.source.clone()),
                                    span: Some((op.as_span().start(), op.as_span().len()).into()),
                                    help: "Tuck should be in the form: a b n tuck".to_string(),
                                })
                            }
                        },
                        "pick" => {
                            // Pick - copy item n deep in stack
                            // ... a b c 2 pick -> ... a b c a
                            if let (Ok(stack_items), Ok(n)) = (lhs, rhs) {
                                // Here n is actually used to determine the depth
                                Ok(Expr::Pick(Box::new(n)))
                            } else {
                                Err(BorfError::ParseError {
                                    message: "Failed to parse components for pick".to_string(),
                                    src: Some(self.source.clone()),
                                    span: Some((op.as_span().start(), op.as_span().len()).into()),
                                    help: "Pick should be in the form: ... items n pick".to_string(),
                                })
                            }
                        },
                        "roll" => {
                            // Roll - move item n deep to top
                            // ... a b c 2 roll -> ... b c a
                            if let (Ok(stack_items), Ok(n)) = (lhs, rhs) {
                                // Here n determines which item to roll to the top
                                Ok(Expr::Roll(Box::new(n)))
                            } else {
                                Err(BorfError::ParseError {
                                    message: "Failed to parse components for roll".to_string(),
                                    src: Some(self.source.clone()),
                                    span: Some((op.as_span().start(), op.as_span().len()).into()),
                                    help: "Roll should be in the form: ... items n roll".to_string(),
                                })
                            }
                        },
                        "cleave" => {
                            // Cleave - apply multiple quotations to same value
                            // lhs: x, rhs: [P] [Q] [R] ...
                            if let (Ok(value), Ok(quotations)) = (lhs, rhs) {
                                if let Expr::Tuple(parts) = &quotations {
                                    let all_quotations = parts.iter().all(|p| {
                                        if let Expr::Quotation(_, _) = p {
                                            true
                                        } else {
                                            false
                                        }
                                    });
                                    
                                    if all_quotations {
                                        Ok(Expr::Cleave(Box::new(value), parts.clone()))
                                    } else {
                                        Err(BorfError::ParseError {
                                            message: "Expected all elements to be quotations for cleave".to_string(),
                                            src: Some(self.source.clone()),
                                            span: Some((op.as_span().start(), op.as_span().len()).into()),
                                            help: "Cleave should be in the form: x [P] [Q] [R] ... cleave".to_string(),
                                        })
                                    }
                                } else {
                                    Err(BorfError::ParseError {
                                        message: "Expected tuple of quotations for cleave".to_string(),
                                        src: Some(self.source.clone()),
                                        span: Some((op.as_span().start(), op.as_span().len()).into()),
                                        help: "Cleave should be in the form: x [P] [Q] [R] ... cleave".to_string(),
                                    })
                                }
                            } else {
                                Err(BorfError::ParseError {
                                    message: "Failed to parse components for cleave".to_string(),
                                    src: Some(self.source.clone()),
                                    span: Some((op.as_span().start(), op.as_span().len()).into()),
                                    help: "Cleave should be in the form: x [P] [Q] [R] ... cleave".to_string(),
                                })
                            }
                        },
                        _ => {
                            Err(BorfError::ParseError {
                                message: format!("Unknown operator: {}", op_str),
                                src: Some(self.source.clone()),
                                span: Some((op.as_span().start(), op.as_span().len()).into()),
                                help: "Valid operators include: |>, match, if, times, loop, while, for, dip, map, filter, fold, bi, tri, etc.".to_string(),
                            })
                        }
                    }
                })
                .parse(pairs)
            },
            // Just pass through other expression types to the atom parser
            _ => self.parse_atom(pair),
        }
    }
    
    // Parse atomic expressions (primary expressions without operators)
    fn parse_atom(&self, pair: Pair<Rule>) -> Result<Expr> {
        match pair.as_rule() {
            Rule::atom => {
                // Get the inner expression from the atom
                let inner = pair.into_inner().next().unwrap();
                self.parse_atom(inner)
            },
            Rule::number => {
                let text = pair.as_str();
                if text.contains('.') {
                    // For now, we'll parse floats as i32 by truncating
                    // In a production parser, you'd handle this properly
                    let float_val: f64 = text.parse().map_err(|_| {
                        BorfError::ParseError {
                            message: format!("Invalid float: {}", text),
                            src: Some(self.source.clone()),
                            span: Some((pair.as_span().start(), pair.as_span().len()).into()),
                            help: "Check that the number is properly formatted".to_string(),
                        }
                    })?;
                    Ok(Expr::Number(float_val as i32))
                } else {
                    let int_val: i32 = text.parse().map_err(|_| {
                        BorfError::ParseError {
                            message: format!("Invalid integer: {}", text),
                            src: Some(self.source.clone()),
                            span: Some((pair.as_span().start(), pair.as_span().len()).into()),
                            help: "Check that the number is properly formatted".to_string(),
                        }
                    })?;
                    Ok(Expr::Number(int_val))
                }
            },
            Rule::string_literal => {
                // Remove the quotes from the string
                let text = pair.as_str();
                let content = &text[1..text.len() - 1];
                // In a real parser, you'd also handle escape sequences here
                Ok(Expr::String(content.to_string()))
            },
            Rule::symbol => {
                let name = pair.as_str();
                // Check if it's a reserved word
                if ["true", "false", "nil"].contains(&name) {
                    // Handle literals
                    match name {
                        "true" => Ok(Expr::Boolean(true)),
                        "false" => Ok(Expr::Boolean(false)),
                        "nil" => Ok(Expr::Nil),
                        _ => unreachable!(),
                    }
                } else {
                    Ok(Expr::Symbol(name.to_string()))
                }
            },
            Rule::quotation => {
                // Parse a quotation with parameters
                let mut inner_pairs = pair.into_inner();
                
                // Check if we have parameters
                let first_pair = inner_pairs.next().unwrap();
                let (params, body_pairs) = if first_pair.as_rule() == Rule::params {
                    // Parse parameters
                    let params = self.parse_params(first_pair)?;
                    
                    // Skip the "->" token
                    let arrow = inner_pairs.next().unwrap();
                    assert_eq!(arrow.as_str(), "->");
                    
                    (params, inner_pairs)
                } else if first_pair.as_str() == "->" {
                    // No parameters, but we have an arrow
                    (Vec::new(), inner_pairs)
                } else {
                    // No parameters, this is part of the body
                    let mut body = vec![first_pair];
                    body.extend(inner_pairs);
                    (Vec::new(), body.into_iter())
                };
                
                // Parse body expressions
                let mut body = Vec::new();
                for body_pair in body_pairs {
                    if body_pair.as_rule() == Rule::expr {
                        body.push(self.parse_expression(body_pair)?);
                    }
                }
                
                // Apply named parameter translation if we have parameters
                if !params.is_empty() {
                    match translate_quotation(&params, &body) {
                        Ok(translated_body) => Ok(Expr::Quotation(Vec::new(), translated_body)),
                        Err(e) => Err(e)
                    }
                } else {
                    Ok(Expr::Quotation(params, body))
                }
            },
            Rule::assignment => {
                // Parse an assignment
                let mut inner_pairs = pair.into_inner();
                let value = self.parse_expression(inner_pairs.next().unwrap())?;
                let name = inner_pairs.next().unwrap().as_str().to_string();
                
                Ok(Expr::Assignment(Box::new(value), name))
            },
            Rule::match_block => {
                // Parse a match block
                let mut cases = Vec::new();
                for case_pair in pair.into_inner() {
                    if case_pair.as_rule() == Rule::pattern_case {
                        let mut case_inner = case_pair.into_inner();
                        let pattern = self.parse_pattern(case_inner.next().unwrap())?;
                        let expr = self.parse_expression(case_inner.next().unwrap())?;
                        cases.push((pattern, expr));
                    }
                }
                
                // Return a placeholder Match expression
                // The actual subject will be filled in by the infix operator handler
                Ok(Expr::Match(Box::new(Expr::Nil), cases))
            },
            Rule::if_branches => {
                // Parse the if branches
                let mut inner_pairs = pair.into_inner();
                
                // Parse true branch
                let mut true_branch = Vec::new();
                let true_branch_pair = inner_pairs.next().unwrap();
                for expr_pair in true_branch_pair.into_inner() {
                    if expr_pair.as_rule() == Rule::expr {
                        true_branch.push(self.parse_expression(expr_pair)?);
                    }
                }
                
                // Parse false branch
                let mut false_branch = Vec::new();
                let false_branch_pair = inner_pairs.next().unwrap();
                for expr_pair in false_branch_pair.into_inner() {
                    if expr_pair.as_rule() == Rule::expr {
                        false_branch.push(self.parse_expression(expr_pair)?);
                    }
                }
                
                // Return a placeholder If expression
                // The actual condition will be filled in by the infix operator handler
                Ok(Expr::If(
                    Box::new(Expr::Nil),
                    Box::new(Expr::Sequence(true_branch)),
                    Box::new(Expr::Sequence(false_branch)),
                ))
            },
            Rule::record_expr => {
                // Parse a record expression
                let mut fields = HashMap::new();
                
                for field_pair in pair.into_inner() {
                    if field_pair.as_rule() == Rule::field_expr {
                        let mut field_inner = field_pair.into_inner();
                        let value = self.parse_expression(field_inner.next().unwrap())?;
                        let name = field_inner.next().unwrap().as_str().to_string();
                        fields.insert(name, value);
                    }
                }
                
                Ok(Expr::Record(fields))
            },
            Rule::tuple_expr => {
                // Parse a tuple expression
                let mut elements = Vec::new();
                
                for elem_pair in pair.into_inner() {
                    if elem_pair.as_rule() == Rule::expr {
                        elements.push(self.parse_expression(elem_pair)?);
                    }
                }
                
                Ok(Expr::Tuple(elements))
            },
            Rule::quoted_expr => {
                // Parse a quoted expression
                let inner = pair.into_inner().next().unwrap();
                let expr = self.parse_expression(inner)?;
                Ok(Expr::Quote(Box::new(expr)))
            },
            Rule::unquoted_expr => {
                // Parse an unquoted expression
                let inner = pair.into_inner().next().unwrap();
                let expr = self.parse_expression(inner)?;
                Ok(Expr::Unquote(Box::new(expr)))
            },
            Rule::quasiquoted_expr => {
                // Parse a quasiquoted expression
                let inner = pair.into_inner().next().unwrap();
                let expr = self.parse_expression(inner)?;
                Ok(Expr::Quasiquote(Box::new(expr)))
            },
            Rule::stack_effect => {
                // Parse a stack effect declaration
                let effect = parse_stack_effect(pair.as_str())?;
                Ok(Expr::StackEffect(effect))
            },
            _ => {
                Err(BorfError::ParseError {
                    message: format!("Unexpected expression rule: {:?}", pair.as_rule()),
                    src: Some(self.source.clone()),
                    span: Some((pair.as_span().start(), pair.as_span().len()).into()),
                    help: format!("This rule is not handled by the parser: {:?}", pair.as_rule()),
                })
            }
        }
    }

    fn parse_params(&self, pair: Pair<Rule>) -> Result<Vec<Param>> {
        let mut params = Vec::new();
        
        for param_pair in pair.into_inner() {
            if param_pair.as_rule() == Rule::param {
                let param_name = param_pair.as_str().to_string();
                params.push(Param {
                    name: param_name,
                    type_annotation: None,
                });
            }
        }
        
        Ok(params)
    }

    fn parse_pattern(&self, pair: Pair<Rule>) -> Result<Pattern> {
        match pair.as_rule() {
            Rule::pattern => {
                // A pattern rule will contain one of the pattern types
                self.parse_pattern(pair.into_inner().next().unwrap())
            },
            Rule::string_literal => {
                // Remove the quotes from the string
                let text = pair.as_str();
                let content = &text[1..text.len() - 1];
                // In a real parser, you'd also handle escape sequences here
                Ok(Pattern::Literal(Expr::String(content.to_string())))
            },
            Rule::number => {
                let text = pair.as_str();
                if text.contains('.') {
                    // For now, we'll parse floats as i32 by truncating
                    let float_val: f64 = text.parse().map_err(|_| {
                        BorfError::ParseError {
                            message: format!("Invalid float: {}", text),
                            src: Some(self.source.clone()),
                            span: Some((pair.as_span().start(), pair.as_span().len()).into()),
                        }
                    })?;
                    Ok(Pattern::Literal(Expr::Number(float_val as i32)))
                } else {
                    let int_val: i32 = text.parse().map_err(|_| {
                        BorfError::ParseError {
                            message: format!("Invalid integer: {}", text),
                            src: Some(self.source.clone()),
                            span: Some((pair.as_span().start(), pair.as_span().len()).into()),
                        }
                    })?;
                    Ok(Pattern::Literal(Expr::Number(int_val)))
                }
            },
            Rule::symbol => {
                let name = pair.as_str();
                if name == "_" {
                    Ok(Pattern::Wildcard)
                } else {
                    Ok(Pattern::Variable(name.to_string()))
                }
            },
            Rule::record_pattern => {
                // Parse a record pattern
                let mut fields = HashMap::new();
                
                for field_pair in pair.into_inner() {
                    if field_pair.as_rule() == Rule::field_pattern {
                        let mut field_inner = field_pair.into_inner();
                        let pattern = self.parse_pattern(field_inner.next().unwrap())?;
                        let name = field_inner.next().unwrap().as_str().to_string();
                        fields.insert(name, pattern);
                    }
                }
                
                Ok(Pattern::Map(fields))
            },
            Rule::quoted_pattern => {
                // Parse a quoted pattern
                let inner = pair.into_inner().next().unwrap();
                let pattern = self.parse_pattern(inner)?;
                Ok(Pattern::Quote(Box::new(pattern)))
            },
            _ => {
                Err(BorfError::ParseError {
                    message: format!("Unexpected pattern rule: {:?}", pair.as_rule()),
                    src: Some(self.source.clone()),
                    span: Some((pair.as_span().start(), pair.as_span().len()).into()),
                })
            }
        }
    }
}

pub fn parse(input: &str) -> Result<Expr> {
    let parser = PestParser::new(input);
    parser.parse()
}