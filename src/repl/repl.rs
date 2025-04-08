// src/repl/repl.rs
// REPL implementation with rustyline

use std::borrow::Cow::{self, Borrowed, Owned};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::time::Duration;

use colored::*;
use rustyline::completion::{Completer, FilenameCompleter, Pair};
use rustyline::error::ReadlineError;
use rustyline::highlight::{Highlighter, MatchingBracketHighlighter};
use rustyline::hint::{Hinter, HistoryHinter};
use rustyline::validate::{self, MatchingBracketValidator, Validator};
use rustyline::{CompletionType, Config, EditMode, Editor};
use rustyline::history::{DefaultHistory, History};
use rustyline_derive::Helper;

use crate::repl::interpreter::{Evaluator, Result, EvaluatorError};

// Add FromError implementation for ReadlineError
impl From<ReadlineError> for EvaluatorError {
    fn from(err: ReadlineError) -> Self {
        EvaluatorError::EvalError(format!("Readline error: {}", err))
    }
}

// Helper for rustyline integration
#[derive(Helper)]
struct BorfHelper {
    highlighter: MatchingBracketHighlighter,
    validator: MatchingBracketValidator,
    hinter: HistoryHinter,
    colored_prompt: String,
    completer: FilenameCompleter,
}

impl Completer for BorfHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        ctx: &rustyline::Context<'_>,
    ) -> std::result::Result<(usize, Vec<Pair>), ReadlineError> {
        self.completer.complete(line, pos, ctx)
    }
}

impl Hinter for BorfHelper {
    type Hint = String;

    fn hint(&self, line: &str, pos: usize, ctx: &rustyline::Context<'_>) -> Option<String> {
        if line.is_empty() || pos < line.len() {
            return None;
        }

        self.hinter.hint(line, pos, ctx)
    }
}

impl Highlighter for BorfHelper {
    fn highlight_prompt<'b, 's: 'b, 'p: 'b>(
        &'s self,
        prompt: &'p str,
        default: bool,
    ) -> Cow<'b, str> {
        if default {
            Borrowed(&self.colored_prompt)
        } else {
            Borrowed(prompt)
        }
    }

    fn highlight_hint<'h>(&self, hint: &'h str) -> Cow<'h, str> {
        Owned(hint.bright_black().to_string())
    }

    fn highlight<'l>(&self, line: &'l str, pos: usize) -> Cow<'l, str> {
        self.highlighter.highlight(line, pos)
    }

    fn highlight_char(&self, line: &str, pos: usize) -> bool {
        self.highlighter.highlight_char(line, pos)
    }
}

impl Validator for BorfHelper {
    fn validate(
        &self,
        ctx: &mut validate::ValidationContext,
    ) -> rustyline::Result<validate::ValidationResult> {
        self.validator.validate(ctx)
    }

    fn validate_while_typing(&self) -> bool {
        self.validator.validate_while_typing()
    }
}

pub struct Repl {
    editor: Editor<BorfHelper, DefaultHistory>,
    evaluator: Evaluator,
    history_file: PathBuf,
    multiline_input: String,
    in_multiline: bool,
}

impl Repl {
    pub fn new() -> Result<Self> {
        // Configure rustyline
        let config = Config::builder()
            .history_ignore_space(true)
            .completion_type(CompletionType::List)
            .edit_mode(EditMode::Emacs)
            .build();

        // Set up the helper
        let helper = BorfHelper {
            highlighter: MatchingBracketHighlighter::new(),
            validator: MatchingBracketValidator::new(),
            hinter: HistoryHinter {},
            colored_prompt: "borf> ".green().to_string(),
            completer: FilenameCompleter::new(),
        };

        // Create editor with config and helper
        let mut editor = Editor::with_config(config)?;
        editor.set_helper(Some(helper));

        // Set up history file
        let history_file = if let Some(mut path) = dirs::home_dir() {
            path.push(".borf_history");
            path
        } else {
            PathBuf::from(".borf_history")
        };

        // Try to load history
        if history_file.exists() {
            let _ = editor.load_history(&history_file);
        }

        // Create evaluator and initialize it
        let mut evaluator = Evaluator::new();
        evaluator.initialize()?;

        Ok(Repl {
            editor,
            evaluator,
            history_file,
            multiline_input: String::new(),
            in_multiline: false,
        })
    }

    pub fn run(&mut self) -> Result<()> {
        println!("{}", "Borf REPL v0.1.0".bold().blue());
        println!("Type {} to exit, {} for help", ":quit".yellow(), ":help".yellow());

        loop {
            let prompt = if self.in_multiline {
                "...> ".green()
            } else {
                "borf> ".green()
            };

            match self.editor.readline(&prompt) {
                Ok(line) => {
                    // Handle REPL commands
                    if !self.in_multiline && line.trim().starts_with(':') {
                        match line.trim() {
                            ":quit" | ":q" => {
                                println!("Goodbye!");
                                break;
                            }
                            ":help" | ":h" => {
                                self.show_help();
                                continue;
                            }
                            ":clear" => {
                                self.editor.clear_screen()?;
                                continue;
                            }
                            ":history" => {
                                self.show_history();
                                continue;
                            }
                            cmd if cmd.starts_with(":load ") => {
                                if let Some(filename) = cmd.split_whitespace().nth(1) {
                                    self.load_file(filename)?;
                                } else {
                                    println!("{}", "Error: Expected filename after :load".red());
                                }
                                continue;
                            }
                            cmd if cmd.starts_with(":save ") => {
                                if let Some(filename) = cmd.split_whitespace().nth(1) {
                                    self.save_history(filename)?;
                                } else {
                                    println!("{}", "Error: Expected filename after :save".red());
                                }
                                continue;
                            }
                            _ => {
                                println!("{}", "Unknown command. Type :help for help.".red());
                                continue;
                            }
                        }
                    }

                    // Handle multiline input
                    if line.trim() == "\\" || line.ends_with('\\') {
                        // Start or continue multiline input
                        if !self.in_multiline {
                            self.in_multiline = true;
                            self.multiline_input.clear();
                        }
                        
                        // Add the line without the trailing backslash
                        if line.ends_with('\\') {
                            self.multiline_input.push_str(&line[..line.len() - 1]);
                        }
                        self.multiline_input.push('\n');
                        
                        // Don't add the line to history yet
                        continue;
                    } else if self.in_multiline {
                        // End multiline input and evaluate
                        self.multiline_input.push_str(&line);
                        
                        // Create a copy of the multiline input for evaluation
                        let input_to_eval = self.multiline_input.clone();
                        
                        // Reset multiline state
                        self.in_multiline = false;
                        
                        // Add the whole multiline input to history
                        self.editor.add_history_entry(&input_to_eval)?;
                        
                        // Evaluate the multiline input
                        self.evaluate_and_print(&input_to_eval);
                        
                        // Clear the multiline buffer for next time
                        self.multiline_input.clear();
                        continue;
                    }

                    // For empty lines, just continue
                    if line.trim().is_empty() {
                        continue;
                    }

                    // Add to history and evaluate normal input
                    self.editor.add_history_entry(&line)?;
                    self.evaluate_and_print(&line);
                }
                Err(ReadlineError::Interrupted) => {
                    // Ctrl-C pressed, cancel current input
                    if self.in_multiline {
                        self.in_multiline = false;
                        self.multiline_input.clear();
                        println!("Multiline input cancelled");
                    } else {
                        println!("Press Ctrl-D or type :quit to exit");
                    }
                }
                Err(ReadlineError::Eof) => {
                    // Ctrl-D pressed, exit REPL
                    println!("Goodbye!");
                    break;
                }
                Err(err) => {
                    println!("Error: {}", err);
                    break;
                }
            }
        }

        // Save history
        if let Err(err) = self.editor.save_history(&self.history_file) {
            eprintln!("Error saving history: {}", err);
        }

        Ok(())
    }

    fn evaluate_and_print(&mut self, input: &str) {
        // Measure evaluation time
        let start = std::time::Instant::now();

        // Evaluate the input
        match self.evaluator.eval(input) {
            Ok(result) => {
                let duration = start.elapsed();
                if !result.is_empty() {
                    println!("{}", result.green());
                }
                if duration > Duration::from_millis(100) {
                    println!("{}", format!("Executed in {:.2?}", duration).bright_black());
                }
            }
            Err(err) => {
                println!("{}", format!("Error: {}", err).red());
            }
        }
    }

    fn show_help(&self) {
        println!("{}", "Borf REPL Help".bold().blue());
        println!("Commands:");
        println!("  {:15} - Exit the REPL", ":quit, :q".yellow());
        println!("  {:15} - Show this help", ":help, :h".yellow());
        println!("  {:15} - Clear the screen", ":clear".yellow());
        println!("  {:15} - Show command history", ":history".yellow());
        println!("  {:15} - Load and execute a file", ":load <filename>".yellow());
        println!("  {:15} - Save command history to file", ":save <filename>".yellow());
        println!("\nMultiline Input:");
        println!("  End a line with {} or type {} alone to start multiline mode", "\\".yellow(), "\\".yellow());
        println!("  Press {} to submit multiline input", "Enter".yellow());
        println!("  Press {} to cancel multiline input", "Ctrl-C".yellow());
        
        println!("\nKeyboard Shortcuts:");
        println!("  {:15} - Previous command", "Up arrow".yellow());
        println!("  {:15} - Next command", "Down arrow".yellow());
        println!("  {:15} - Move cursor left", "Left arrow".yellow());
        println!("  {:15} - Move cursor right", "Right arrow".yellow());
        println!("  {:15} - Delete character under cursor", "Delete".yellow());
        println!("  {:15} - Delete character before cursor", "Backspace".yellow());
        println!("  {:15} - Move to start of line", "Home, Ctrl-A".yellow());
        println!("  {:15} - Move to end of line", "End, Ctrl-E".yellow());
        println!("  {:15} - Clear the line", "Ctrl-U".yellow());
        println!("  {:15} - Tab completion", "Tab".yellow());
        println!("  {:15} - Insert newline in multiline mode", "Alt-Enter".yellow());
        
        println!("\nBorf Language Examples:");
        println!("  5 10 add            => Add two numbers");
        println!("  [x y -> x y add]    => Define a function that adds its arguments");
        println!("  5 |> [x -> x 2 mul] => Use the pipeline operator");
        println!("  'expr               => Quote an expression");
        println!("  [x: Num -> x]       => Use type annotations");
        println!("  #Type               => Quote a type");
    }

    fn show_history(&self) {
        let history = self.editor.history();
        if history.len() == 0 {
            println!("No history available");
            return;
        }

        println!("{}", "Command History:".bold());
        for (i, entry) in history.iter().enumerate() {
            println!("{:4}: {}", i + 1, entry);
        }
    }

    fn load_file(&mut self, filename: &str) -> Result<()> {
        println!("Loading file: {}", filename);
        match self.evaluator.eval_file(filename) {
            Ok(result) => {
                if !result.is_empty() {
                    println!("{}", result.green());
                }
                println!("File loaded successfully");
                Ok(())
            }
            Err(err) => {
                println!("{}", format!("Error loading file: {}", err).red());
                Err(err)
            }
        }
    }

    fn save_history(&self, filename: &str) -> Result<()> {
        let history = self.editor.history();
        if history.len() == 0 {
            println!("No history to save");
            return Ok(());
        }

        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(filename)?;

        for entry in history.iter() {
            writeln!(file, "{}", entry)?;
        }

        println!("History saved to {}", filename);
        Ok(())
    }
}