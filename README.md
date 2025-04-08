# Borf Programming Language

Borf is a concatenative, homoiconic programming language with advanced type-level metaprogramming capabilities. It features a minimal core with powerful metaprogramming facilities, allowing for infinite extensibility while maintaining a small, comprehensible language.

## Core Philosophy

Borf is designed around these key principles:

- **Minimal Core**: Keep the language core as small as possible, comparable to or smaller than Scheme's 17 special forms
- **Metaprogramming First**: Extend functionality through metaprogramming rather than built-in syntax
- **Concatenative Style**: Embrace the stack-based, data-flow oriented approach for composability
- **Type Metaprogramming**: Use the type system as a metaprogramming platform

## Features

- Concatenative programming style with stack-based operations
- Quotations with named parameters and type annotations
- Pipeline operator (`|>`) for explicit data flow
- Linear types for resource management
- Homoiconicity for metaprogramming with quotation/unquotation
- Pattern matching for control flow
- Module system for code organization
- Type-level metaprogramming with type quotation
- Categorical foundations inspired by Catlab.jl
- Lafont-style interaction nets for computation

## Metacircular Evaluator

This project includes a metacircular evaluator for Borf. The evaluator is implemented in Rust and can interpret Borf code, including the advanced features like quotation, linear types, and metaprogramming.

## REPL

Borf now includes a powerful REPL with:

- Readline-like functionality (command history, line editing)
- Multiline input support
- History search
- Tab completion
- Persistent history between sessions

### Using the REPL

Start the REPL by running:

```bash
cargo run
```

Or explicitly:

```bash
cargo run -- repl
```

### REPL Commands

- `:quit`, `:q` - Exit the REPL
- `:help`, `:h` - Show help
- `:clear` - Clear the screen
- `:history` - Show command history
- `:load <filename>` - Load and execute a file
- `:save <filename>` - Save command history to file

### Multiline Input

- End a line with `\` or type `\` alone to start multiline mode
- Press `Enter` to submit multiline input
- Press `Ctrl-C` to cancel multiline input

## Running Borf Files

```bash
cargo run -- -f examples/your_file.borf
```

## Evaluating Expressions

```bash
cargo run -- eval "5 10 add"
```

## Style Guide

## Syntax Guidelines

### Variable Assignment

- Use `:` for variable assignment with value-first syntax:
  ```borf
  42 : answer
  ```

### Type Annotations

- Use `<:` for type annotations in parameters and return values:
  ```borf
  [x <: Int, y <: Int -> x + y] <: Int
  ```

### Function Definitions

- Name functions using the assignment syntax with quotations:
  ```borf
  add: [x <: Int, y <: Int -> x y +] <: Int
  ```

### Quotations and Lambdas

- Use `->` to introduce the body of a quotation or lambda:
  ```borf
  [x, y -> x + y]
  ```

### Metaprogramming

- Use backticks for quasiquotation:
  ```borf
  `{
    value: $x,
    compute: $(x * 2)
  }
  ```

### Data Flow

- Use the pipe operator `|>` for data flow when it improves readability:
  ```borf
  data |> transform |> process |> render
  ```

### Metaprogramming Best Practices

1. **Extend, Don't Replace**: Build on existing patterns rather than creating entirely new ones
2. **Document Abstractions**: Always document syntax extensions clearly
3. **Test Metaprogramming**: Test all metaprogramming extensions thoroughly
4. **Layer Abstractions**: Build higher-level abstractions on top of well-tested lower-level ones
5. **Keep It Simple**: Avoid overly complex metaprogramming that obscures the intent

## Code Examples

Basic arithmetic:
```borf
5 10 add
```

Defining a function:
```borf
add_function: [x, y -> x y +]
5 10 add_function
```

Using the pipeline operator:
```borf
5 |> [x -> x 3 +] |> compute
```

Quotation and unquotation:
```borf
'[x -> x 2 *] : quoted_func
quoted_func $ 4 |> execute
```

Type annotations:
```borf
[x <: Num, y <: String -> x y] <: (Num, String)
```

Pattern matching:
```borf
{ Some 42 } |> match {
  | Some(x) -> x
  | None -> 0
}
```

Type-level metaprogramming:
```borf
make_pair_type: [t1 <: Type, t2 <: Type -> 
  #{ first: $t1, second: $t2 }
] <: Type

#Int #String make_pair_type |> : int_string_pair
```

Extending the language with metaprogramming:
```borf
-- Define a new control structure using quasiquoting
for_each_syntax: [items, variable, body ->
  `{
    -- Define a recursive helper
    for_helper: [remaining, idx ->
      if remaining is_empty then
        -- Done iterating
        {}
      else
        -- Get current item
        current : remaining[0]
        rest : remaining[1:]
        
        -- Execute body with variable bound to current item
        $variable : current
        $body
        
        -- Process the rest recursively
        rest idx + 1 for_helper
      end
    ]
    
    -- Start the iteration
    $items 0 for_helper
  }
] <: Code
```

## Pre-requisites

To build and run Borf you need:

- [Rust](https://www.rust-lang.org/): installation instructions can be found [here](https://www.rust-lang.org/tools/install)

## Building from Source

```bash
git clone <repository-url>
cd borf
cargo build --release
```

The executable will be available at `target/release/borf`

## WebAssembly Support

The project also includes WebAssembly support using the WASM Component Model, allowing Borf to be integrated into VS Code and web environments.
