# Borf Language Design Document

## 1. Current Implementation Status

### 1.1 Core Features
- **Parser**: Handles basic Borf syntax including numbers, strings, symbols, quotations with named parameters, pipelines, and assignments
- **AST Representation**: Using Rust enums to represent dwhenerent expression types
- **Stack-based Evaluation**: Values are pushed and popped from a stack during execution
- **Environment System**: Tracks variable bindings and their values
- **Basic Operations**: Supports arithmetic, string manipulation, and basic control flow
- **Metacircular Capabilities**: Initial support for self-evaluation

### 1.2 Syntax Support
- Quotations with named parameters: `[x, y -> ...]`
- Pipeline operator: `|>` for explicit data flow
- Variable assignment: `x -> value`
- Comments: Lua-style `--` and `--[[...]]--`
- Basic symbols and literals

### 1.3 Limitations
- No support for linear types and resource tracking
- Limited pattern matching capabilities
- No advanced metaprogramming operations
- Missing category theory structures
- No interaction net reduction model
- Incomplete module system

## 2. Standard Library Analysis

### 2.1 prim.borf Analysis

The `prim.borf` file defines core primitives and shows the intended structure of the standard library.

#### Key Components:
1. **Module System**: Uses `module` and `import` directives
2. **Documentation**: Rich documentation format with markdown-like syntax
3. **Linear Types**: Types marked with `!` that must be used exactly once
4. **Category Theory Mappings**: Explicit mappings to category theory concepts
5. **Type Annotations**: Function type signatures using `=>` notation
6. **Resource Management**: Explicit tracking of resource usage

#### Implementation Gaps:
- Need to add support for linear type checking
- Need to implement category theory structures (functor, applicative, monad)
- Need to add runtime resource tracking
- Need to expand the type system

### 2.2 syntax.borf Analysis

The `syntax.borf` file defines metaprogramming tools and code manipulation capabilities.

#### Key Components:
1. **Code as Data**: Operations to treat code as manipulatable data
2. **Quotation/Unquotation**: Operators for code generation and execution
3. **Pattern Matching**: Advanced pattern matching for code transformation
4. **Hygienic Macros**: Macro system that respects lexical scoping
5. **AST Manipulation**: Tools for inspecting and modifying code structures

#### Implementation Gaps:
- Need to implement quotation/unquotation operators
- Need to support pattern matching on code structures
- Need to add AST traversal and transformation
- Need to implement hygienic macro expansion

## 3. Implementation Roadmap

### Phase 1: Core Enhancements
1. Expand the AST representation to support all required expression types
2. Add support for linear type annotations and checking
3. Implement advanced pattern matching
4. Add module system with imports

### Phase 2: Metaprogramming Support
1. Implement quotation/unquotation operators
2. Add AST manipulation functions
3. Support code traversal and transformation
4. Implement hygienic macro expansion

### Phase 3: Categorical Foundations
1. Implement ACSet (Attributed C-Set) for graph representation
2. Add WiringDiagram structures for compositional systems
3. Create Lafont-style interaction net representation
4. Implement categorical morphisms and functors

### Phase 4: Interaction Net Evaluation
1. Implement pattern matching for interaction nets
2. Add algebraic rewriting system for net reduction
3. Create optimization strategies for net evaluation
4. Implement resource tracking for linear types

### Phase 5: Standard Library
1. Implement the core primitives module
2. Add syntax manipulation module
3. Create categorical abstractions library (catlab)
4. Build utility functions for common operations

## 4. Revised Prelude Files

Below are proposed revisions to the prelude files to ensure consistency and implementability.

### 4.1 Revised prim.borf

```
-- Core primitives module
module prim

--[[ 
  Basic types and operations for Borf
  This module defines the foundation of the standard library.
]]--

-- Type definitions
-- ! indicates a linear type (must be used exactly once)
type !File => { path: String, handle: Int }
type Maybe[T] => { Some: T } | { None }
type Either[L, R] => { Left: L } | { Right: R }

-- Basic arithmetic operations
add :: Num, Num => Num
[x, y -> x + y] -> add

sub :: Num, Num => Num
[x, y -> x - y] -> sub

mul :: Num, Num => Num
[x, y -> x * y] -> mul

div :: Num, Num => !Maybe[Num]
[x, y -> 
  0 == y |> [-> None] 
        |> [-> x / y |> Some]
] -> div

-- String operations
concat :: String, String => String
[x, y -> x ++ y] -> concat

-- File operations (using linear types for resource management)
open_file :: String => !Maybe[!File]
[path -> 
  -- Implementation will check if file exists
  -- and return Some with file or None if not found
] -> open_file

close_file :: !File => Unit
[file -> 
  -- Implementation will ensure the file is closed
  -- and can no longer be used (enforced by linear type)
] -> close_file

-- Functorial mapping (category theory structure)
map :: [a -> b], List[a] => List[b]
[f, xs -> 
  -- Apply f to each element of xs
  xs |> [x -> f x] |> collect
] -> map

-- Monadic operations
bind :: [a -> Maybe[b]], Maybe[a] => Maybe[b]
[f, mx -> 
  mx |> [
    Some x -> f x,
    None -> None
  ]
] -> bind

-- Resource handling with linear types
with_file :: String, [!File -> a] => Maybe[a]
[path, f -> 
  open_file path |> [
    Some file -> 
      -- Linear type ensures file is used exactly once
      f file,
    None -> None
  ]
] -> with_file
```

### 4.2 Revised syntax.borf

```
-- Code as data tools for metaprogramming
module syntax

import prim

--[[ 
  Metaprogramming operations for code manipulation
  This module provides tools for treating code as data.
]]--

-- Basic quotation and unquotation
-- 'expr creates a quoted expression (code as data)
-- $expr unquotes an expression (evaluates code data)

quote :: a => 'a
[x -> 'x] -> quote

unquote :: 'a => a
[q -> $q] -> unquote

-- Code inspection
is_quotation :: Any => Bool
[x -> 
  x |> type |> [
    Quotation -> true,
    _ -> false
  ]
] -> is_quotation

is_symbol :: Any => Bool
[x -> 
  x |> type |> [
    Symbol -> true,
    _ -> false
  ]
] -> is_symbol

-- Pattern matching on code structures
match_expr :: 'a => Maybe['a]
[expr -> 
  expr |> [
    'Symbol name -> 'Symbol name |> Some,
    'Number n -> 'Number n |> Some,
    'Quotation q -> 'Quotation q |> Some,
    _ -> None
  ]
] -> match_expr

-- AST transformations
transform :: 'a, ['a -> 'b] => 'b
[expr, transformer -> 
  -- Apply transformer to expression
  transformer expr
] -> transform

-- Hygienic macro definition
defmacro :: Symbol, 'a => Unit
[name, body -> 
  -- Define a macro that will be expanded at compile time
  -- with proper hygiene for variable names
  -- Implementation ensures lexical scoping is respected
] -> defmacro

-- Code generation helpers
gen_function :: Symbol, List[Symbol], 'a => 'a
[name, params, body -> 
  -- Generate a function definition with the given name,
  -- parameters, and body
  '[params -> body] -> name
] -> gen_function

-- Compile-time evaluation
eval_at_compile :: 'a => a
[expr -> 
  -- Evaluate expression at compile time
  -- Implementation will execute during compilation
  $expr
] -> eval_at_compile
```

## 5. Design Considerations

### 5.1 Homoiconicity
The language design maintains homoiconicity by representing code as manipulatable data structures. This is essential for the metacircular evaluator and metaprogramming capabilities.

### 5.2 Concatenative Style
Operations compose by concatenation, with values preceding operations. The pipeline operator (`|>`) makes data flow explicit and improves readability.

### 5.3 Linear Types
Linear types provide resource safety by ensuring resources are used exactly once. This is crucial for managing external resources like files and network connections.

### 5.4 Interaction Net Reduction
The interaction net model provides a clean semantics for reduction and evaluation. This formal model makes reasoning about program behavior more tractable.

### 5.5 Categorical Foundations (Catlab-inspired)
Borf incorporates categorical structures inspired by Catlab.jl:

#### 5.5.1 ACSet (Attributed C-Sets)
ACSet provides a natural computational categorical representation of graphs and networks, with attributes attached to nodes and edges.

#### 5.5.2 Wiring Diagrams
Compositional structure of systems is represented as diagrams with boxes and wires, allowing formal reasoning about system composition.

#### 5.5.3 Lafont Interaction Nets
The core computational model uses Lafont-style interaction nets, where:
- Cells/agents have one principal port and multiple auxiliary ports
- Computation occurs when principal ports connect (creating active pairs)
- Reduction follows algebraic rewrite rules
- The system is inherently parallel and confluent

#### 5.5.4 Algebraic Rewriting
Pattern-based transformation system provides a formal way to specify reduction rules for interaction nets.

## 6. Implementation Strategy

1. **Incremental Development**: Implement features in order of dependency
2. **Test-Driven Approach**: Write examples first to guide implementation
3. **Separation of Concerns**: Clearly separate parsing, type checking, and evaluation
4. **Extensibility**: Design with extension points for future features
5. **Performance**: Pay attention to performance characteristics early

This design document serves as a roadmap for implementing the full vision of the Borf language, building on the existing implementation while adding the advanced features required by the standard library.