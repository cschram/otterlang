# OtterLang Language Specification

This document provides a complete specification of the OtterLang programming language syntax, semantics, and features.

## Table of Contents

1. [Lexical Structure](#lexical-structure)
2. [Types](#types)
3. [Expressions](#expressions)
4. [Statements](#statements)
5. [Functions](#functions)
6. [Structs and Classes](#structs-and-classes)
7. [Enums](#enums)
8. [Pattern Matching](#pattern-matching)
9. [Control Flow](#control-flow)
10. [Error Handling](#error-handling)
11. [Concurrency](#concurrency)
12. [Modules](#modules)
13. [Standard Library](#standard-library)

## Lexical Structure

### Comments

```otter
# Single-line comment

# Multi-line comments
# are written as multiple
# single-line comments
```

### Identifiers

Identifiers start with a letter or underscore, followed by letters, digits, or underscores:

```otter
valid_name
_valid_name
name123
```

### Keywords

Reserved keywords: `def`, `let`, `return`, `if`, `elif`, `else`, `for`, `while`, `break`, `continue`, `pass`, `class`, `struct`, `enum`, `match`, `case`, `use`, `pub`, `spawn`, `await`, `try`, `except`, `finally`, `raise`, `as`, `type`

### Literals

**Numbers:**
```otter
42          # Integer
3.14        # Float
1e10        # Scientific notation
```

**Strings:**
```otter
"hello"     # String literal
'world'     # String literal (single quotes)
f"value: {x}"  # F-string (interpolated)
```

**Booleans:**
```otter
true
false
```

**None/Unit:**
```otter
nil         # None value
```

## Types

### Primitive Types

- `int` - 64-bit signed integer
- `float` - 64-bit floating point
- `bool` - Boolean
- `string` - String
- `unit` - Unit type (no value)

### Type Annotations

```otter
let x: int = 42
let name: string = "Otter"
let pi: float = 3.14
```

### Generic Types

```otter
let numbers: [int] = [1, 2, 3]
let mapping: {string: int} = {"a": 1, "b": 2}
let maybe: Option<int> = Option.Some(42)
```

### Type Aliases

```otter
type ID = int
type Name = string
type UserID = ID
```

## Expressions

### Arithmetic

```otter
1 + 2       # Addition
3 - 1       # Subtraction
2 * 4       # Multiplication
8 / 2       # Division
5 % 3       # Modulo
2 ** 3      # Exponentiation
```

### Comparison

```otter
a == b      # Equality
a != b      # Inequality
a < b       # Less than
a > b       # Greater than
a <= b      # Less than or equal
a >= b      # Greater than or equal
```

### Logical

```otter
a and b     # Logical AND
a or b      # Logical OR
not a       # Logical NOT
```

### Member Access

```otter
obj.field
module.function
Option.Some
```

### Function Calls

```otter
function(arg1, arg2)
obj.method(arg)
```

### F-Strings

```otter
f"Hello, {name}!"
f"Value: {x}, Sum: {a + b}"
```

## Statements

### Variable Declaration

```otter
let x = 42
let name: string = "Otter"
let y: int  # Uninitialized (must be assigned before use)
```

### Assignment

```otter
x = 10
name = "New Name"
```

### Return

```otter
return value
return  # Returns unit
```

### Expression Statements

Any expression can be used as a statement:

```otter
print("Hello")
x + 1  # Evaluated but result discarded
```

## Functions

### Function Definition

```otter
def greet(name: string) -> string:
    return f"Hello, {name}!"
```

### Function with Default Parameters

```otter
def greet(name: string, greeting: string = "Hello") -> string:
    return f"{greeting}, {name}!"
```

### Generic Functions

```otter
def first<T>(items: [T]) -> T:
    return items[0]
```

### Public Functions

```otter
pub def exported_function():
    # Can be imported by other modules
    pass
```

## Structs and Classes

### Definition

```otter
class Point:
    x: float
    y: float

# 'class' and 'struct' are aliases
struct Point:
    x: float
    y: float
```

### Instantiation

```otter
let p = Point{x: 1.0, y: 2.0}
let origin = Point{x: 0.0, y: 0.0}
```

### Methods

```otter
class Point:
    x: float
    y: float
    
    def distance(self) -> float:
        return math.sqrt(self.x * self.x + self.y * self.y)
```

### Generic Structs

```otter
class Pair<T, U>:
    first: T
    second: U
```

## Enums

### Definition

```otter
enum Option<T>:
    Some: (T)
    None

enum Result<T, E>:
    Ok: (T)
    Err: (E)
```

### Enum Constructors

```otter
let some_value = Option.Some(42)
let none_value = Option.None
let ok_result = Result.Ok(3.14)
let err_result = Result.Err("error message")
```

### Generic Enums

```otter
enum Maybe<T>:
    Just: (T)
    Nothing
```

## Pattern Matching

### Match Expression

```otter
let result = match value:
    case Option.Some(x):
        f"Got value: {x}"
    case Option.None:
        "No value"
```

### Pattern Types

**Identifier Pattern:**
```otter
case x:
    # Binds entire value to x
```

**Wildcard Pattern:**
```otter
case _:
    # Matches anything, binds nothing
```

**Enum Variant Pattern:**
```otter
case Option.Some(value):
    # Matches Some variant, binds payload to value
case Option.None:
    # Matches None variant
```

**Literal Pattern:**
```otter
case 42:
    # Matches specific literal value
case "hello":
    # Matches string literal
```

### Match Guards

```otter
match x:
    case n if n > 0:
        "positive"
    case n if n < 0:
        "negative"
    case _:
        "zero"
```

## Control Flow

### If Statements

```otter
if condition:
    # then branch
elif other_condition:
    # elif branch
else:
    # else branch
```

### For Loops

```otter
for i in 0..10:
    print(i)

for item in items:
    print(item)
```

### While Loops

```otter
while condition:
    # loop body
    if should_break:
        break
    if should_continue:
        continue
```

### Break and Continue

```otter
while True:
    if condition:
        break  # Exit loop
    if skip:
        continue  # Skip to next iteration
```

## Error Handling

### Try-Except-Finally

```otter
try:
    # code that might raise
    result = risky_operation()
except Error as e:
    # handle error
    print(f"Error: {e}")
finally:
    # cleanup code (always executes)
    cleanup()
```

### Raise

```otter
raise "Error message"
raise ValueError("Invalid input")
```

### Exception Types

```otter
try:
    # code
except ValueError:
    # handle ValueError
except TypeError:
    # handle TypeError
except Error:
    # handle any error
```

## Concurrency

### Spawn

```otter
let task = spawn:
    return compute_heavy_task()

let result = await task
```

### Await

```otter
let task1 = spawn: compute_a()
let task2 = spawn: compute_b()

let result1 = await task1
let result2 = await task2
```

## Modules

### Import

```otter
use math
use std.io as io
use core
```

### Module Definition

```otter
# mymodule.ot
pub def public_function():
    return "accessible"

def private_function():
    return "not accessible"
```

### Re-exports

Re-exports allow modules to re-export items from other modules, enabling facade patterns and cleaner public APIs.

```otter
# Re-export specific items
pub use math.sqrt
pub use math.sin as sine

# Re-export all public items from a module
pub use math
```

**Re-exporting specific items:**
```otter
# math.ot
pub def sqrt(x: float) -> float:
    # ... implementation

# mymodule.ot
pub use math.sqrt  # Re-export sqrt from math module
```

**Re-exporting with rename:**
```otter
# mymodule.ot
pub use math.sin as sine  # Re-export sin as sine
```

**Re-exporting all items:**
```otter
# mymodule.ot
pub use math  # Re-export all public items from math
```

Re-exports must reference items that are actually public in the source module. Re-export chains are supported.

### Standard Library Modules

- `core` - Core types and functions (Option, Result)
- `math` - Mathematical functions
- `std.io` - Input/output functions
- `std.time` - Time functions

## Standard Library

### Built-in Functions

**`print(message: string) -> unit`**
Prints a message to standard output.

**`println(message: string) -> unit`**
Prints a message followed by a newline.

**`eprintln(message: string) -> unit`**
Prints a message to standard error.

**`str(value: any) -> string`**
Converts any value to its string representation.

**`len(collection) -> int`**
Returns the length of a collection.

### Collections

**Lists:**
```otter
let items = [1, 2, 3]
let first = items[0]
items.append(4)
```

**Dictionaries:**
```otter
let map = {"key": "value"}
let value = map["key"]
map["new"] = "entry"
```

### Core Enums

**Option<T>:**
```otter
enum Option<T>:
    Some: (T)
    None
```

**Result<T, E>:**
```otter
enum Result<T, E>:
    Ok: (T)
    Err: (E)
```

## Grammar Summary

### Program Structure

```
program := (statement | function | struct | enum | type_alias)*
```

### Function Definition

```
function := [pub] def identifier [<type_params>] (params) [-> type] : block
params := param ("," param)*
param := identifier : type [= default_expr]
```

### Statement

```
statement := let identifier [: type] [= expr]
           | assignment
           | return [expr]
           | if_stmt
           | for_stmt
           | while_stmt
           | try_stmt
           | expr
```

### Expression

```
expr := literal
      | identifier
      | expr binop expr
      | unop expr
      | expr.member
      | expr(args)
      | match expr : (case pattern [if guard] : expr)+
      | f_string
```

### Pattern

```
pattern := identifier
         | _
         | literal
         | enum_name.variant (pattern*)
         | struct_name {field: pattern*}
         | [pattern*]
```

## Semantics

### Type System

OtterLang uses a static type system with type inference. Types are checked at compile time.

### Evaluation Order

Expressions are evaluated left-to-right. Function arguments are evaluated before the function is called.

### Scoping

Variables are scoped to the block in which they are declared. Inner scopes can shadow outer scopes.

### Memory Management

OtterLang uses automatic memory management. Values are reference-counted, and memory is freed when no longer needed.

## Implementation Notes

- OtterLang compiles to LLVM IR, which is then compiled to native binaries
- The language is indentation-sensitive (Python-style)
- All numbers are represented as 64-bit floats internally, but can be annotated as integers
- Enums are encoded with variant tags in the upper 32 bits and payloads in the lower 32 bits of an i64 value

## See Also

- [Tutorials](./TUTORIALS.md) - Step-by-step guides
- [API Reference](./API_REFERENCE.md) - Standard library documentation

