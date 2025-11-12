# OtterLang Tutorial Series

## Getting Started

Welcome to OtterLang! This tutorial series will guide you through learning the language.

## Table of Contents

1. [Installation](#installation)
2. [Your First Program](#your-first-program)
3. [Variables and Types](#variables-and-types)
4. [Functions](#functions)
5. [Control Flow](#control-flow)
6. [Collections](#collections)
7. [Structs and Types](#structs-and-types)
8. [Concurrency](#concurrency)
9. [Error Handling](#error-handling)
10. [Advanced Topics](#advanced-topics)

## Installation

See the main [README](../README.md) for installation instructions.

## Your First Program

Create a file `hello.ot`:

```otter
def main():
    print("Hello, World!")
```

Run it:

```bash
otter run hello.ot
```

## Variables and Types

### Basic Types

```otter
def main():
    # Integers
    x = 42
    
    # Floats
    pi = 3.14
    
    # Strings
    name = "Otter"
    
    # Booleans
    is_active = true
    
    # Arrays
    numbers = [1, 2, 3, 4, 5]
    
    # Dictionaries
    person = {"name": "Otter", "age": 42}
```

### Type Annotations

```otter
def main():
    x: int = 42
    name: string = "Otter"
```

## Functions

### Basic Functions

```otter
def greet(name: string) -> string:
    return f"Hello, {name}!"

def main():
    message = greet("World")
    print(message)
```

### Multiple Parameters

```otter
def add(x: int, y: int) -> int:
    return x + y

def main():
    result = add(10, 20)
    print(f"Result: {result}")
```

## Control Flow

### If Statements

```otter
def check_age(age: int):
    if age >= 18:
        print("Adult")
    elif age >= 13:
        print("Teenager")
    else:
        print("Child")
```

### Match Expressions

```otter
def day_name(day: int) -> string:
    return match day:
        case 1:
            "Monday"
        case 2:
            "Tuesday"
        case 3:
            "Wednesday"
        case _:
            "Unknown"
```

### Loops

```otter
def main():
    # While loop
    mut i = 0
    while i < 10:
        print(f"Count: {i}")
        i = i + 1
    
    # For loop
    for num in [1, 2, 3, 4, 5]:
        print(f"Number: {num}")
```

## Collections

### Arrays

```otter
def main():
    numbers = [1, 2, 3, 4, 5]
    
    # Access elements
    first = numbers[0]
    
    # Iterate
    for num in numbers:
        print(f"{num}")
```

### Dictionaries

```otter
def main():
    person = {"name": "Otter", "age": 42}
    
    # Access values
    name = person["name"]
    age = person["age"]
```

## Structs and Types

### Defining Structs

```otter
struct Point:
    x: float
    y: float

def main():
    p = Point{x: 1.0, y: 2.0}
    print(f"Point: ({p.x}, {p.y})")
```

### Type Aliases

```otter
type ID = int
type Name = string

def create_user(id: ID, name: Name):
    # ...
```

## Concurrency

### Spawning Tasks

```otter
def main():
    task1 = spawn:
        return compute_something()
    
    task2 = spawn:
        return compute_something_else()
    
    result1 = await task1
    result2 = await task2
    
    print(f"Results: {result1}, {result2}")
```

## Error Handling

OtterLang uses `nil` for error handling:

```otter
def safe_divide(a: float, b: float) -> float | nil:
    if b == 0:
        return nil
    return a / b

def main():
    result = safe_divide(10, 2)
    if result != nil:
        print(f"Result: {result}")
    else:
        print("Division by zero!")
```

## Advanced Topics

### Generics

```otter
def first<T>(items: [T]) -> T:
    return items[0]

def main():
    numbers = [1, 2, 3]
    first_num = first(numbers)
    
    strings = ["a", "b", "c"]
    first_str = first(strings)
```

### F-Strings

```otter
def main():
    name = "Otter"
    age = 42
    message = f"Hello, {name}! You are {age} years old."
    print(message)
```

### String Conversion

Convert values to strings with the built-in `str()` helper (`stringify()` is kept only for backward compatibility):

```otter
def main():
    num = 42
    num_str = str(num)  # "42"
    print(f"Number as string: {num_str}")
```

### Modules and Re-exports

Create modules to organize your code:

```otter
# math_utils.ot
pub def add(a: float, b: float) -> float:
    return a + b

pub def multiply(a: float, b: float) -> float:
    return a * b

# main.ot
use math_utils

def main():
    result = math_utils.add(5, 3)
    print(result)
```

**Re-exports** allow you to create facade modules that aggregate functionality:

```otter
# math.ot
pub def sqrt(x: float) -> float:
    # ... implementation
    pass

pub def sin(x: float) -> float:
    # ... implementation
    pass

# my_math.ot - Facade module
pub use math.sqrt
pub use math.sin as sine

# Now users can import from my_math instead of math
# main.ot
use my_math

def main():
    result = my_math.sqrt(16)  # Re-exported from math
    sine_val = my_math.sine(0.5)  # Re-exported with rename
```

You can also re-export all items from a module:

```otter
# api.ot - Facade that re-exports everything
pub use math
pub use io
pub use time

# Users can import everything from api
use api

def main():
    api.sqrt(16)
    api.print("Hello")
```

For more information, see the [Language Specification](./LANGUAGE_SPEC.md) and [API Reference](./API_REFERENCE.md).
