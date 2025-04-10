# Borf Language - Formal Specification

## 1. Introduction

Borf is a stack-based, homoiconic, quotational programming language with support for metaprogramming and category theory-inspired abstractions. This specification defines the syntax and semantics for Borf version 1.0.

## 2. Language Philosophy

Borf follows these key design principles:

1. **Stack-Based Core**: At its heart, Borf is a pure stack-based concatenative language with explicit stack operations.

2. **Syntactic Sugar Through Metaprogramming**: Named parameters and pipelines are syntactic sugar that gets expanded to core stack operations.

3. **Single Execution Model**: All Borf code, regardless of syntax used, compiles down to the same stack-based operations.

4. **Progressive Disclosure**: Users can start with convenient syntax and learn the underlying stack operations as they progress.

5. **Transparent Transformations**: Tools allow users to see how high-level syntax gets rewritten to core stack operations.

This design provides both the ergonomics of modern programming languages and the performance and conceptual simplicity of stack-based languages.

## 3. Lexical Structure

### 3.1 Character Set
Borf programs are written in UTF-8 encoded text. The language is case-sensitive.

### 3.2 Tokens

#### 3.2.1 Whitespace
Whitespace characters include commas, space (`U+0020`), tab (`U+0009`), carriage return (`U+000D`), and newline (`U+000A`). Whitespace is used to separate tokens but is otherwise insignificant.

#### 3.2.2 Comments
- Line comments start with `--` and continue to the end of the line.
- Block comments start with `--[[` and end with `]]--`.
- Comments are ignored during parsing and do not affect the semantics of a program.

#### 3.2.3 Identifiers
Identifiers consist of a sequence of alphanumeric characters, underscores, and may include symbols from the set `?`, `!`, `'`, and `$`, except as the first character. The first character must be a letter or underscore.

```
identifier ::= [a-zA-Z_][a-zA-Z0-9_?!'$]*
```

Reserved words cannot be used as identifiers:
```
module, import, if, then, else, match, case, of, where, let, in, true, false
```

#### 3.2.4 Literals
- **Integer literals**: A sequence of decimal digits, optionally preceded by a minus sign.
  ```
  integer_literal ::= ['-']?[0-9]+
  ```

- **Float literals**: A sequence of decimal digits containing a decimal point, optionally preceded by a minus sign.
  ```
  float_literal ::= ['-']?[0-9]+'.'[0-9]+
  ```

- **String literals**: Text enclosed within double quotes. Escape sequences are supported.
  ```
  string_literal ::= '"' ([^"\\\n] | escape_sequence)* '"'
  escape_sequence ::= '\\' ['"\\nrt]
  ```

#### 3.2.5 Punctuation
Punctuation tokens include: `[`, `]`, `{`, `}`, `(`, `)`, `:`, `,`, `->`, `|>`, `=>`, `'`, `$`, `#`, `|`.

## 4. Formal Grammar (EBNF)

> **Note on Operator Positioning:** As Borf is a concatenative language, all operators are conceptually postfix (they follow their operands). For readability, some operators like quoting (`'`), unquoting (`$`), and type operators (`#`) may appear in prefix position in examples, but they are conceptually postfix in the language design.

Using Extended Backus-Naur Form (EBNF) for the grammar definition:

```ebnf
program ::= module_decl? import_decl* top_level_decl*

module_decl ::= string_literal 'module'

import_decl ::= string_literal 'import'

top_level_decl ::= expression

expression ::= literal
             | identifier
             | quotation
             | assignment
             | pipeline
             | match_expr
             | if_expr
             | binary_expr
             | record_expr
             | tuple_expr
             | list_expr
             | quote_expr
             | unquote_expr
             | quasiquote_expr
             | module_expr

literal ::= integer_literal | float_literal | string_literal

quotation ::= '[' param_list? '->' expression* ']'

param_list ::= param (param)*

param ::= identifier

assignment ::= expression ':' identifier  // Only this form is valid

pipeline ::= expression '|>' expression

match_expr ::= expression '{' pattern_case* '}' 'match'

pattern_case ::= '|' pattern '=>' expression

pattern ::= '_'                           // wildcard
          | literal                       // literal pattern
          | identifier                    // variable binding
          | '{' field_pattern* '}'        // record pattern
          | '\'' pattern                  // quoted pattern

field_pattern ::= pattern ':' identifier

// Factor-style if expressions
if_expr ::= expression '[' expression* ']' '[' expression* ']' 'if'

binary_expr ::= expression expression binary_op  // Standard operators

binary_op ::= '+' | '-' | '*' | '/' | '^' | '%' | '==' | '===' | '<' | '>' | '<=' | '>='

record_expr ::= '{' field_expr* '}'

field_expr ::= expression ':' identifier

tuple_expr ::= '(' expression* ')'

expression_list ::= expression (expression)*

quote_expr ::= '\'' expression

unquote_expr ::= '$' expression

quasiquote_expr ::= '`' expression

module_expr ::= '{' module_field* '}'

module_field ::= expression ':' identifier
```

## 5. Syntactic Analysis

### 5.1 Expressions

#### 5.1.1 Literal Expressions
Literal expressions represent constant values:
- Integer literals: `42`, `-7`
- Float literals: `3.14`, `-0.5`
- String literals: `"Hello, world!"`

Syntactic constraints:
- Integers and floats cannot have extraneous leading zeros.
- Strings must be properly terminated with closing quotes.

#### 5.1.2 Identifier Expressions
Identifiers reference named values in the current scope. The identifier must be defined before it can be used (with the exception of recursive definitions).

#### 5.1.3 Quotation Expressions
Quotations define blocks of concatenative code to be evaluated 
```
2 3 [x y -> x y +]   // Quotation taking two parameters x and y and adding them
```

To pass a Quotation around as a lambda, simply quote the Quotation:

'[x y -> x y +] :my_lambda // Quotation taking two parameters x and y and adding them

or bind the quotation to a name:

[x y -> x y +] :my_add

Syntactic constraints:
- Parameter list (if any) must be followed by the arrow (`->`) token.
- Empty parameter list is allowed: `[-> expr]`

#### 5.1.4 Assignment Expressions
Assignment binds a value to an identifier:
```
42 :answer       // Binds 42 to identifier "answer"
```

Syntactic constraints:
- The value expression is always on the left.
- The identifier is always on the right.
- The ONLY valid form is `value :name`, NOT `value -> name`.
- Assignment does not return a value.

#### 5.1.5 Pipeline Expressions
Pipeline expressions pass the result of one expression to another:
```
'[1 2 3] |> sum    // Passes the list [1, 2, 3] to the sum Quotation
```

Syntactic constraints:
- The left-hand side must evaluate to a value.
- The right-hand side must evaluate to a Quotation or be a Quotation call.

#### 5.1.6 Match Expressions
Match expressions perform pattern matching:
```
value {
  | 0 => "zero"
  | n => "non-zero: " n to_string |> concat
} match
```

Syntactic constraints:
- Each pattern case must start with the `|` symbol.
- Patterns must be followed by the arrow (`=>`) token.
- At least one pattern case must be provided.
- The expression being matched comes first, followed by the match block.

#### 5.1.7 If Expressions
If expressions in Borf follow Factor's style:
```
x 0 > [
  "positive"
] [
  "non-positive"
] if
```

Syntactic constraints:
- Condition is evaluated first.
- Two quotations follow: true branch and false branch.
- The expression ends with an `if` token.
- Each branch must be enclosed in square brackets.
- There is NO initial `if` token - only a trailing one.

#### 5.1.8 Binary Expressions
Binary expressions use standard mathematical operators:
```
a b +     // Addition
a b -     // Subtraction
a b *     // Multiplication
a b /     // Division
a b ^     // Exponentiation
a b %     // Modulo
a b ==    // Value equality
a b ===   // Structural/categorical equivalence
a b <     // Less than
a b >     // Greater than
a b <=    // Less than or equal
a b >=    // Greater than or equal
```

Syntactic constraints:
- Both operands must precede the operator (postfix notation).
- Standard mathematical operators are used, not Quotation names.
- Equality uses `==` for value equality and `===` for structural/categorical equivalence.

#### 5.1.9 Record Expressions
Record expressions create structured data with named fields:
```
{ "Alice" :name
       30 :age }
```

Syntactic constraints:
- Fields are identified by name, followed by a colon and the value.
- Field order is not significant.

#### 5.1.10 Tuple Expressions
Tuple expressions create ordered, heterogeneous collections of values:
```
(1 "hello" true)   // A tuple with three elements of different types
```

Syntactic constraints:
- Elements are enclosed in parentheses.
- Elements can be of any type.
- Order is significant, unlike in records.
- Tuples are immutable.

#### 5.1.11 List Expressions
List expressions create ordered collections of values from an unevaluated Quotation
```
'[1 2 3 4]
```

Syntactic constraints:
- Commas are whitespace. Add them if you wish, the parser will ignore them.
- Empty lists are denoted by `[]`.

#### 5.1.12 Quotation-related Expressions
Quotation-related expressions implement metaprogramming:

- Quote: Treats code as data: `'expression` (conceptually postfix: `expression '`)
- Unquote: Evaluates quoted code: `$expression` (conceptually postfix: `expression $`)
- Quasiquote: Templating with unquoted parts: `` `expression ``

Syntactic constraints:
- Quoting can nest, with the innermost unquote resolving in its enclosing quote.
- While conceptually postfix operators, these are typically written in prefix position for readability.

### 5.2 Patterns

#### 5.2.1 Wildcard Pattern
The wildcard pattern `_` matches any value without binding it.

#### 5.2.2 Literal Pattern
Literal patterns match specific constant values:
```
| 0 => "zero"
| "hello" => "greeting"
```

#### 5.2.3 Variable Pattern
Variable patterns match any value and bind it to an identifier:
```
| x => "Value: " x to_string |> concat
```

#### 5.2.4 Record Pattern
Record patterns match record values with specific structures:
```
| { n:name a:age } => "Name: " n ", Age: " a to_string |> concat |> concat
```

#### 5.2.5 Quoted Pattern
Quoted patterns match quoted expressions:
```
| '[x y +] => "Addition expression"
```

## 6. Semantic Analysis

### 6.1 Scoping Rules

#### 6.1.1 Lexical Scoping
Borf uses lexical scoping, meaning identifiers are resolved in the scope where they are defined.

#### 6.1.2 Environments
Each scope is represented by an environment that maps identifiers to values. Environments form a chain, with each inner scope having access to outer scopes.

#### 6.1.3 Shadowing
Identifiers in inner scopes can shadow identifiers with the same name from outer scopes.

### 6.2 Evaluation Order

#### 6.2.1 Strict Evaluation
Borf uses strict (eager) evaluation. Expressions are evaluated from left to right.

#### 6.2.2 Stack-based Model
The evaluation stack maintains intermediate values during execution:
1. Values are pushed onto the stack.
2. Quotations consume values from the top of the stack.
3. Quotation results are pushed back onto the stack.

#### 6.2.3 Quotation Application
For Quotation application:
1. Arguments are evaluated from left to right and pushed onto the stack.
2. The Quotation is evaluated and applied to the arguments on the stack.
3. The result is pushed onto the stack.

### 6.3 Assignment Semantics

#### 6.3.1 Value Binding
Assignment binds a value to an identifier in the current environment using only the `:` operator:
```
42 :answer   // Binds 42 to "answer"
```

The form `value -> name` is NOT an alternative syntax for assignment and has a different meaning in the language (used for named Quotation parameters).

### 6.4 Execution Model and Syntactic Sugar

#### 6.4.1 Core Language vs Syntactic Sugar

Borf has a clear separation between the core language and syntactic sugar:

##### Core Language: Explicit Stack Manipulation
At its core, Borf is a pure stack-based concatenative language with explicit stack manipulation:

```
3 4 +         // Push 3, push 4, add them; result: 7
5 dup *       // Push 5, duplicate it, multiply; result: 25
1 2 3 rot     // Rotates 3rd item to top: 2 3 1
4 5 swap      // Swaps top two items: 4 5 -> 5 4
6 drop        // Removes top element from stack
```

Core stack manipulation primitives:
- `dup`: Duplicate the top stack item
- `drop`: Remove the top stack item
- `swap`: Exchange the top two stack items
- `rot`: Rotate third item to the top
- `over`: Copy the second item to the top
- `tuck`: Copy the top item to the third position
- `pick`: Copy the nth item to the top

##### Syntactic Sugar via the STACKER Algorithm
Named parameters and pipelines are implemented as syntactic sugar that gets rewritten to explicit stack operations via the STACKER translation algorithm.

###### The STACKER Algorithm

The STACKER algorithm systematically translates human-friendly named parameter syntax into core stack operations:

1. **Named parameters in quotations:**
```
// User writes this with named parameters
[x y -> x y +]

// Gets expanded internally to these stack operations
[1 pick 1 pick + ]
```

2. **Pipeline operator:**
```
// User writes this with pipeline
value |> function

// Gets expanded to these stack operations
value function
```

###### How STACKER Works

The STACKER algorithm follows these steps to translate named parameters:

1. **Parameter Mapping**: Maps parameters to their initial stack depths
   - Last parameter (rightmost) is at depth 0 (top of stack)
   - Second-to-last at depth 1, and so on

2. **Dynamic Depth Tracking**: For each operation in the body:
   - Tracks how the stack depth changes as operations are performed
   - Updates the actual depth needed for each parameter reference

3. **Parameter Translation**: When a parameter is referenced:
   - Calculates its current depth: `actual_depth = initial_depth + current_stack_depth_increase`
   - Generates `actual_depth pick` operation
   - Increments stack depth (since `pick` adds an item)

4. **Operation Tracking**: For each operation:
   - Tracks how it affects the stack (inputs consumed, outputs produced)
   - Updates the running stack depth accordingly

5. **Pipeline Handling**: Treats pipeline operators (`|>`) as no-ops
   - They're purely structural syntax with no runtime overhead
   - Simply appends the pipeline target to the generated code

Example walkthrough for `[x y -> x y +]`:
1. Map parameters: `y` at depth 0, `x` at depth 1
2. For token `x`:
   - `actual_depth = initial_depth(1) + current_increase(0) = 1`
   - Generate `1 pick`, stack increase becomes 1
3. For token `y`:
   - `actual_depth = initial_depth(0) + current_increase(1) = 1`
   - Generate `1 pick`, stack increase becomes 2
4. For token `+`:
   - Add `+` to output
   - Update stack increase: `2 - 2 + 1 = 1` (consumes 2, produces 1)
5. Result: `1 pick 1 pick +`

The algorithm correctly handles complex cases:
- Parameter reuse (accessing a parameter multiple times)
- Multi-stage pipelines
- Nested quotations
- Operations with varying stack effects

###### Stack Effect Declarations for STACKER

The STACKER algorithm requires knowing the stack effect of every operation in the body to calculate correct depths:

```
// Stack effect declaration shows inputs and outputs
+ ( a b -- a+b )         // Addition takes 2 items, produces 1
dup ( a -- a a )         // Duplicate takes 1 item, produces 2
map ( list quot -- list ) // Map takes a list and quotation, returns a list
```

Without these stack effect declarations, the algorithm couldn't correctly track how operations affect the stack depth, which would lead to incorrect parameter access. This is why stack effect declarations are a fundamental part of Borf, ensuring that named parameters are correctly translated.

By translating to explicit stack operations, Borf maintains a single, consistent execution model while allowing developers to use more expressive syntax.

##### Examples Showing Equivalence

1. String manipulation:
   ```
   // With syntactic sugar
   "hello" |> uppercase |> reverse
   
   // Expands to core stack operations
   "hello" uppercase reverse
   ```

2. List processing with named parameters:
   ```
   // With syntactic sugar
   [x -> x 2 *] :double
   [1 2 3] |> map double
   
   // Expands to core stack operations (map expects a quotation on top of stack)
   [0 pick 2 *] :double
   [1 2 3] double map
   ```

3. Complex expression with named parameters:
   ```
   // With syntactic sugar
   [user -> 
     user |> name |> uppercase |> find_in_database |> extract_details
   ]
   
   // Expands to core operations
   [0 pick name uppercase find_in_database extract_details]
   ```

#### 6.4.2 Benefits of This Approach
- **Single Mental Model**: Only one execution model to understand (explicit stack)
- **Transparency**: Users can learn what's happening under the hood
- **Predictable Performance**: All code compiles to the same core operations
- **Metaprogramming Flexibility**: Language extensions can be built on this foundation
- **Pedagogy**: Learn high-level syntax first, then understand the stack

#### 6.4.3 Working with Multiple Values

For functions that need to operate on multiple values, Borf uses the quotation approach rather than allowing functions to arbitrarily consume the stack:

```
// Using a quoted list
'[2 3 4 5] sum              // Creates a list and passes it to sum

// Using pipeline syntax
'[2 3 4 5] |> sum           // Same operation with pipeline syntax 

// Using list creation with map
'[1 2 3] |> map [x -> x * 2]  // Maps over the list elements
```

This approach ensures:
- Clear function boundaries and stack effects
- Deterministic behavior
- Preservation of lexical scope
- Predictable composition of functions

All operations on multiple values expect either a single list value or explicit parameters, rather than consuming "everything on the stack."

#### 6.4.4 Debugging Support
Borf provides tools to see how syntactic sugar gets expanded to stack operations:

```
expand([x y -> x y +])
// Output: [1 pick 1 pick +]

trace(data |> process |> transform)
// Output: 
// 1. data process transform
// 2. Stack: [ data ]
// 3. Stack: [ processed_data ]
// 4. Stack: [ transformed_data ]
```

### 6.5 Error Handling

#### 6.5.1 Runtime Errors
Runtime errors interrupt execution and propagate up the call stack.

#### 6.5.2 Error Values
Error values can be created and handled explicitly:
```
"Error message" error   // Creates an error value in concatenative style
```

## 7. Stack Effects, Linear Effect System, and Gradual Typing

Borf uses three complementary systems to provide safety and expressiveness:
1. Stack Effects - Tracking data flow on the stack
2. Linear Effect System - Controlling usage of resources
3. Gradual Type System - Optional type safety with both static and dynamic checking

### 7.1 Stack Effect Notation

Stack effects describe how functions consume inputs from and produce outputs to the stack. They are written using the following notation:

```
( inputs -- outputs )
```

Where:
- `inputs` is the list of items consumed from the stack (from left to right, leftmost is deepest)
- `--` separates inputs from outputs
- `outputs` is the list of items pushed onto the stack (from left to right, leftmost is pushed first)

Examples:
```
( n -- n n )          // dup: duplicates the top item
( a b -- b a )        // swap: swaps the top two items
( a b c -- b c a )    // rot: rotates the third item to the top
( a -- )              // drop: removes the top item
( a b -- a b a )      // over: copies the second item to the top
( a b -- b a b )      // tuck: copies the top item to the third position
( i*1+n ... i+1 i -- i*1+n ... i+1 i i+n ) // pick: copies the nth item to the top
```

### 7.2 Stack Effect Declarations

Every word in Borf must have a stack effect declaration. This is necessary for the compiler to translate named parameter syntax to explicit stack operations correctly.

#### 7.2.1 Declaring Stack Effects for Words

Stack effects are declared for all words in the program:

```
add ( a b -- a+b )
[a b -> a b +]
```

The compiler uses these declarations to:
1. Calculate stack depth changes during execution
2. Generate correct translation for named parameters
3. Verify that quotations use the stack correctly

#### 7.2.2 Stack Effects for Core Operations

Basic operations:
```
dup ( a -- a a )
drop ( a -- )
swap ( a b -- b a )
rot ( a b c -- b c a )
over ( a b -- a b a )
tuck ( a b -- b a b )
pick ( i*1+n ... i+1 i -- i*1+n ... i+1 i i+n )
```

Arithmetic operations:
```
+ ( a b -- a+b )
- ( a b -- a-b )
- ( a -- -a )      // Unary minus
* ( a b -- a*b )
/ ( a b -- a/b )
mod ( a b -- a%b )
```

Logical operations:
```
and ( a b -- c )
or ( a b -- c )
not ( a -- b )
```

Equality operations:
```
== ( a b -- c )
!= ( a b -- c )
< ( a b -- c )
> ( a b -- c )
<= ( a b -- c )
>= ( a b -- c )
```

### 7.3 Stack Effect Calculation

When translating named parameter syntax to explicit stack operations, the compiler calculates the stack depths:

#### 7.3.1 Translation Algorithm

1. Map parameters to initial stack positions (rightmost param at depth 0)
2. For each operation in the body:
   - If it's a parameter reference, calculate its current depth based on:
     * Initial depth
     * Current stack depth increase
   - Generate the appropriate `pick` operation
   - Track how each operation changes the stack depth

#### 7.3.2 Example Translation

```
[x y -> x y +]
```

Translates to:

```
[1 pick 1 pick +]
```

Detailed steps:
1. Map parameters: `y` at depth 0, `x` at depth 1
2. `x`: initial depth = 1, current depth = 1, generate `1 pick`, stack increases by 1
3. `y`: initial depth = 0, stack increase = 1, so actual depth = 1, generate `1 pick`, stack increases by 1 more
4. `+`: consumes 2 items, produces 1, stack decreases by 1

### 7.4 Linear Effect System

The Linear Effect System controls how resources are tracked and used, applying linear logic principles to dynamically typed values. This provides many of the safety benefits of linear types without requiring static type checking.

#### 7.4.1 Effect Annotations

Words that create, consume, or transform resources have effect annotations in addition to stack effects:

```
open_file ( path -- file ) !creates[file]
close_file ( file -- ) !consumes[file]
read_line ( file -- file line ) !uses[file]
```

Effect annotations appear after the stack effect using `!` notation.

#### 7.4.2 Effect Types

Borf defines the following effect types:

1. **`!creates[resource]`** - Indicates a word that creates a new resource that must be properly managed
2. **`!consumes[resource]`** - Indicates a word that consumes (destroys/releases) a resource
3. **`!uses[resource]`** - Indicates a word that uses a resource without consuming it
4. **`!transfers[resource]`** - Indicates a word that transfers ownership of a resource
5. **`!pure`** - Explicitly indicates a word with no side effects

#### 7.4.3 Runtime Tracking of Linear Resources

The Borf runtime associates unique IDs with resources and tracks their state:

```
"data.txt" open_file  // Creates a file resource with a unique ID
dup read_line         // Uses the file resource but doesn't consume it
swap close_file       // Consumes the file resource
```

The runtime enforces linearity constraints:
- Resources created with `!creates` must eventually be consumed with `!consumes`
- Resources cannot be used after they are consumed
- Resources cannot be duplicated unless explicitly allowed

#### 7.4.4 Resource Tags

For dynamic linearity checking, resources are tagged with their resource type:

```
"socket" tag_resource    // Tags a value as a "socket" resource
is_resource? ( value -- bool )  // Checks if a value is a resource
resource_type ( resource -- type )  // Gets the resource type
```

#### 7.4.5 Linear Function Composition

The effect system enables safe composition of functions that use resources:

```
// A function that processes a file
process_file ( file -- file result ) !uses[file]
[file -> 
  file read_line process_line
]

// Two functions that must be called in sequence
!seq[a b] // Ensures a is called before b
```

#### 7.4.6 Borrowing and Regions

For temporary resource access, Borf implements borrowing through delimited regions:

```
file with_borrowed [borrowed_file ->
  // Can use borrowed_file but not consume it
  borrowed_file read_line drop
] // borrowed_file goes out of scope here
```

The `with_borrowed` operation creates a temporary reference that cannot escape the region.

### 7.5 Stack and Effect Safety

The combined systems provide comprehensive safety:

1. **Stack Underflow Prevention**: Verifies words don't consume more items than are available
2. **Stack Balance Verification**: Checks quotations maintain expected stack balance
3. **Resource Leak Prevention**: Ensures resources are properly consumed or released
4. **Use-After-Free Prevention**: Prevents using resources after they're consumed
5. **Named Parameter Reliability**: Ensures correct translation of named parameters

### 7.6 Dynamic Stack and Effect Inspection

For debugging and development, Borf provides tools to inspect the stack and effects:

```
.s                 // Print the current stack
depth              // Push the current stack depth
inspect_word       // Show the stack effect and definition of a word
.resources         // List active resources and their states
effect_trace       // Enable tracing of effect operations
```

### 7.7 "T-Shirt Typing": Borf's Gradual Type System

In addition to stack effects and linear effect tracking, Borf incorporates a gradual type system affectionately known as "T-Shirt Typing" (technically a set-theoretic type system). Borf is proud to be the second t-shirt typed language in the programming world.

This typing system is:

1. **Sound** - The inferred and assigned types align with the behavior of the program
2. **Gradual** - Includes the `dynamic()` type for runtime checking, but behaves statically when explicit types are used
3. **T-Shirt Compatible** - Types are described, implemented, and composed using basic set operations: unions, intersections, and negation, just like how a t-shirt can belong to multiple categories at once

#### 7.7.1 Type Notation

Types are written using the type name followed by parentheses:

```
integer()          // Integer values
list(integer())    // Lists containing only integers
(integer() -> boolean())  // Function taking an integer and returning a boolean
```

#### 7.7.2 Basic Types

Borf provides several basic types:

- `symbol()` - Symbolic values
- `binary()` - Binary data
- `integer()` - Integer numbers
- `float()` - Floating-point numbers
- `quotation()` - Quotations (functions)
- `list()` - Lists
- `map()` - Maps/dictionaries
- `tuple()` - Tuples

#### 7.7.3 Special Types

- `none()` - Represents an empty set of values
- `word()` - Represents all types (universal set)
- `dynamic()` - Represents a range of the given types (for gradual typing)
- `boolean()` - Convenience alias for `true or false`

#### 7.7.4 Type Composition

Types can be composed using set operations:

```
symbol() or integer()    // Union: values that are either symbols OR integers
symbol() and integer()   // Intersection: values that are both symbols AND integers
                         // For disjoint base types like these, the intersection is empty
                         // For function types, intersection means the function handles both types
not nil                  // Negation: any value except nil
```

When working with function types, intersection has a special meaning, and this is where the name "T-Shirt Typing" comes from:

```
(integer() -> integer()) and (boolean() -> boolean())
```

This represents a function that can handle both integers and booleans as inputs, returning values of the corresponding type. The intersection for function types expresses that the function belongs to both sets simultaneously.

#### The T-Shirt Analogy

Imagine a t-shirt with both green and yellow stripes:
- It belongs to the set of "t-shirts with green"
- It also belongs to the set of "t-shirts with yellow"
- So its type is the intersection: "t-shirts with green" AND "t-shirts with yellow"

Similarly, a pattern-matching function in Borf can belong to multiple function type sets at once:

```
[x -> 
  | integer() => x * -1    -- Handles integers
  | boolean() => not x     -- Also handles booleans
]
```

This function is like a multi-colored t-shirt - it belongs to multiple categories simultaneously. This powerful typing approach enables expressive pattern matching while maintaining type safety.

#### 7.7.5 The `dynamic()` Type

The `dynamic()` type enables gradual typing in Borf:

- When no type is specified, Borf assumes `dynamic()`
- Can be restricted with intersections: `dynamic() and integer()`
- Shorthand: `dynamic(integer())` equivalent to `dynamic() and integer()`
- Only triggers type errors when all possible types would fail

#### 7.7.5.1 T-Shirt Typing with Pattern Matching

Pattern matching in Borf is powered by t-shirt typing. When a function handles multiple types:

```
negate [x ->
  | integer() => x * -1
  | boolean() => not x
]
```

This function has the type: `(integer() -> integer()) and (boolean() -> boolean())`

The Borf type checker knows:
- When you pass it an integer, it returns an integer
- When you pass it a boolean, it returns a boolean
- If you pass it anything else, it's a type error

Like a t-shirt that's both casual and formal, this function belongs to multiple categories at once.

#### 7.7.6 Relationship with Stack Effects

Stack effects and gradual types work together:

1. **Stack effects** ensure correct stack manipulation and composition
2. **Gradual types** provide additional safety for value manipulation 

This dual system allows programs to:
- Start with minimal typing and focus on stack effects
- Add increasing type safety as code matures
- Get compile-time guarantees where needed while maintaining flexibility

## 8. Evaluation Rules

### 8.1 Literals

Literals evaluate to their corresponding values:
```
42           // The integer 42
"hello"      // The string "hello"
```

### 8.2 Variables

Variables evaluate to their bound values in the current environment:
```
x            // The value bound to x
```

### 8.3 Quotations

Quotations evaluate to Quotation values:
```
[x y -> x y +]   // A Quotation that adds its two arguments
```

The Quotation captures the current environment for closures.

### 8.4 Quotation Application

Quotation application executes the Quotation with the provided arguments:
```
5 3 +      // Applies the '+' operator to 5 and 3
```

Evaluation steps:
1. Push 5 onto the stack.
2. Push 3 onto the stack.
3. Evaluate '+' and apply it to the top two stack values.
4. Push the result (8) onto the stack.

### 8.5 Pipelines

Pipelines pass the result of one expression to another:
```
x |> f       // Applies f to x
```

Evaluation steps:
1. Evaluate x and push the result onto the stack.
2. Evaluate f and apply it to the result of x.

### 8.6 Match Expressions

Match expressions evaluate patterns against a value:
```
value {
  | pattern1 => expr1
  | pattern2 => expr2
} match
```

Evaluation steps:
1. Evaluate value.
2. Try each pattern in order.
3. For the first matching pattern, evaluate the corresponding expression.

Match expressions can be written with syntactic sugar (using pipelines) or with direct stack operations:
```
// With syntactic sugar
value |> match [patterns...]

// Expands to core stack operations
value [patterns...] match
```

### 8.7 If Expressions

If expressions in Factor style evaluate one of two quotations based on a condition:
```
condition [ true_branch ] [ false_branch ] if
```

Evaluation steps:
1. Evaluate condition.
2. If true, evaluate the first Quotation; otherwise, evaluate the second Quotation.
3. The `if` token at the end delimits the entire if expression.

## 9. Module System

### 9.1 Module Declarations

Modules are declared using the `module` keyword:
```
"module_name" module
```

### 9.2 Module Imports

Modules are imported using the `import` keyword:
```
"module_name" import
```

### 9.3 Module Exports

All top-level bindings are exported from a module.

## 10. Standard Library

### 10.1 Core Operations

#### 10.1.1 Arithmetic
- `+`: Addition ( a b -- a+b )
- `-`: Subtraction ( a b -- a-b )
- `*`: Multiplication ( a b -- a*b )
- `/`: Division ( a b -- a/b )
- `^`: Exponentiation ( a b -- a^b )
- `%`: Modulo ( a b -- a%b )

#### 10.1.2 Comparison
- `==`: Value equality ( a b -- c )
- `===`: Structural equality ( a b -- c )
- `<`: Less than ( a b -- c )
- `>`: Greater than ( a b -- c )
- `<=`: Less than or equal ( a b -- c )
- `>=`: Greater than or equal ( a b -- c )

#### 10.1.3 Logical
- `and`: Logical AND ( a b -- c )
- `or`: Logical OR ( a b -- c )
- `not`: Logical NOT ( a -- b )

### 10.2 Collection Operations

#### 10.2.1 Lists
- `map`: Apply a Quotation to each element ( list quot -- result )
- `filter`: Select elements satisfying a predicate ( list quot -- result )
- `fold`: Reduce a list to a single value ( list init quot -- result )
- `length`: Get the length of a list ( list -- n )
- `concat`: Concatenate lists ( list1 list2 -- result )

#### 10.2.2 Maps/Records
- `keys`: Get the keys of a map ( map -- keys )
- `values`: Get the values of a map ( map -- values )
- `has_key`: Check if a key exists ( map key -- bool )
- `insert`: Insert a key-value pair ( map key value -- new_map )
- `delete`: Remove a key-value pair ( map key -- new_map )

### 10.3 String Operations

- `concat`: Concatenate strings ( str1 str2 -- result )
- `length`: Get the length of a string ( str -- n )
- `substring`: Extract a substring ( str start end -- result )
- `starts_with`: Check if a string starts with a prefix ( str prefix -- bool )
- `ends_with`: Check if a string ends with a suffix ( str suffix -- bool )

### 10.4 Metaprogramming

- `eval`: Evaluate quoted code ( quoted -- result )
- `quote`: Quote an expression ( expr -- quoted )
- `unquote`: Unquote an expression ( quoted -- expr )

## 11. Error Handling

### 11.1 Error Types

- Syntax errors: Detected during parsing
- Runtime errors: Detected during execution

### 11.2 Error Creation

Errors can be created explicitly:
```
"Error message" error   // Creates an error value in concatenative style
```

### 11.3 Error Handling

Errors can be caught and handled with either direct stack operations or syntactic sugar:

```
// Core stack operations
expr {
  | Ok value => value handle_success
  | Error err => err handle_error
} try

// With syntactic sugar
expr |> try {
  | Ok value => value |> handle_success
  | Error err => err |> handle_error
}

// Both expand to the same core operations
```