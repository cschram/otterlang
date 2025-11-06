# Transparent Rust Crate Bridge

OtterLang's transparent FFI system automatically exposes the entire public API of any Rust crate, allowing you to use Rust libraries directly from OtterLang code without manual configuration.

## Overview

The transparent bridge system:

1. **Automatically extracts** the public API from Rust crates using `rustdoc` JSON
2. **Generates bridge shims** that convert between OtterLang and Rust types
3. **Handles memory management** automatically (no manual `free()` calls needed)
4. **Supports async/await** natively for Rust `Future` types
5. **Caches bridges** by crate+version+features hash for fast rebuilds

## Basic Usage

### Importing a Rust Crate

```otter
use rust:rand

fn main:
    let value = rand.random_f64()
    print("Random value: {}", value)
```

The `use rust:crate` syntax automatically:
- Builds a bridge crate if needed
- Exposes all public functions, types, and constants
- Registers them in the symbol registry for type checking

### Bridge Configuration (Optional)

**No configuration needed!** The transparent bridge automatically extracts all public APIs from Rust crates using `rustdoc` JSON. You can use any Rust crate without creating a `bridge.yaml` file.

If you need to customize behavior (e.g., specific crate version, features, or override function signatures), you can create a `ffi/<crate>/bridge.yaml` file:

```yaml
dependency:
  name: rand
  version: "0.8"
  features: ["std"]

# Functions are auto-extracted, but you can override specific ones:
functions:
  - name: "rand:custom_function"
    rust_path: "rand::custom_path"
    params: ["I32"]
    result: "F64"
```

**Note:** When `bridge.yaml` exists, its functions override transparently-extracted ones with the same name. If no `bridge.yaml` exists, all functions are automatically extracted from rustdoc.

### Module Path Resolution

Functions are accessed using dot notation matching Rust's module structure:

```otter
use rust:chrono

fn main:
    // chrono::Utc::now() becomes:
    let now = chrono.Utc.now()
    print("Current time: {}", now)
```

### With Aliases

You can alias imported crates:

```otter
use rust:serde_json as json

fn main:
    let data = json.from_str("{\"key\": \"value\"}")
    print("Parsed: {}", data)
```

## Type Mapping

### Primitive Types

| Rust Type | OtterLang Type | Notes |
|-----------|----------------|-------|
| `()` | `unit` | No value |
| `bool` | `bool` | Boolean |
| `i32` | `i32` | 32-bit integer |
| `i64` | `i64` | 64-bit integer |
| `f64` | `f64` | 64-bit float |
| `&str` / `String` | `str` | String (copied) |

### Complex Types

| Rust Type | OtterLang Representation | Notes |
|-----------|--------------------------|-------|
| `Option<T>` | `str` (JSON) or `nil` | Use `_optjson` helper for JSON |
| `Result<T, E>` | `str` (JSON) or exception | Use `_try` helper for JSON |
| `Vec<T>` | `Opaque` (handle) | Refcounted, auto-dropped |
| `&[T]` | `Opaque` (handle) | Copy-in/copy-out |
| Custom types | `Opaque` (handle) | Refcounted, auto-dropped |

## Async Support

For async Rust functions, the bridge automatically generates two helpers:

### Spawn Pattern

```otter
use rust:tokio

fn main:
    // Spawn returns an opaque handle (i64)
    let handle = spawn(tokio.time.sleep(1000))
    
    // Do other work...
    
    // Await the result
    let result = await(handle)
```

### Direct Await

```otter
use rust:tokio

fn main:
    // Direct await (spawns internally)
    let result = await(tokio.time.sleep(1000))
```

The generated bridge exports:
- `<function>_spawn(...)` → returns `Opaque` handle
- `<function>_await(handle)` → returns the result type

## Error Handling

### Result Types

For functions returning `Result<T, E>`, use the `_try` helper:

```otter
use rust:reqwest

fn main:
    // Returns JSON string: {"ok": true, "value": ...} or {"ok": false, "error": "..."}
    let response = reqwest.get_try("https://api.example.com/data")
    
    // Parse and handle
    // (In real code, you'd parse the JSON and check "ok" field)
    print("Response: {}", response)
```

### Option Types

For functions returning `Option<T>`, use the `_optjson` helper:

```otter
use rust:some_crate

fn main:
    // Returns JSON string: {"some": true, "value": ...} or {"some": false}
    let result = some_crate.find_optjson("key")
    
    // Parse and handle
    print("Result: {}", result)
```

## Memory Management

### Automatic Cleanup

Opaque handles (returned for complex types) are automatically reference-counted and cleaned up when no longer referenced:

```otter
use rust:some_crate

fn main:
    let handle = some_crate.create_thing()
    // Use handle...
    // Automatically dropped when handle goes out of scope
```

### Manual Release (Advanced)

If you need to manually release a handle:

```otter
// Handles are automatically released when variables go out of scope
// Manual release is rarely needed, but available via:
// otter_handle_release(handle)
```

## Configuration

### Bridge Metadata

For fine-grained control, you can create `ffi/<crate>/bridge.yaml`:

```yaml
dependency:
  name: "my_crate"
  version: "1.0"
  features: ["feature1", "feature2"]
  default_features: false

# Optional: manual function overrides
# functions:
#   - name: "custom_function"
#     rust_path: "my_crate::custom::function"
#     params: ["F64", "F64"]
#     result: "F64"
```

### Cache Management

Bridges are cached under `~/.otter_cache/ffi/<crate>-<hash>/` where the hash includes:
- Crate name
- Version
- Features
- Path (if using local path dependency)

To clear cache:
```bash
rm -rf ~/.otter_cache/ffi
```

## Examples

### Example 1: Random Numbers

```otter
use rust:rand

fn main:
    let random1 = rand.random_f64()
    let random2 = rand.uniform(1.0, 10.0)
    
    print("Random [0,1): {}", random1)
    print("Uniform [1,10): {}", random2)
```

### Example 2: JSON Parsing

```otter
use rust:serde_json

fn main:
    let json_str = "{\"name\": \"Alice\", \"age\": 30}"
    let parsed = serde_json.from_str_try(json_str)
    
    print("Parsed JSON: {}", parsed)
```

### Example 3: Date/Time

```otter
use rust:chrono

fn main:
    let now = chrono.Utc.now()
    print("Current UTC time: {}", now)
```

### Example 4: Async HTTP Request

```otter
use rust:reqwest

fn main:
    // Spawn async request
    let handle = spawn(reqwest.get("https://api.example.com"))
    
    print("Request sent, doing other work...")
    
    // Await result
    let response = await(handle)
    print("Response received: {}", response)
```

### Example 5: Complex Types

```otter
use rust:nalgebra

fn main:
    // Create a vector (returns opaque handle)
    let vec = nalgebra.Vector3.new(1.0, 2.0, 3.0)
    
    // Use the vector
    let length = nalgebra.Vector3.norm(vec)
    
    print("Vector length: {}", length)
    // vec is automatically dropped when it goes out of scope
```

## Diagnostics

### Type Checking

The type checker uses registry signatures when available, providing accurate type errors:

```otter
use rust:rand

fn main:
    // Error: function expects 2 arguments, got 1
    let x = rand.uniform(1.0)  // ❌ Type error
```

### Common Errors

#### "failed to create bridge crate directory"
- **Cause**: Permission issues or disk full
- **Fix**: Check `~/.otter_cache/ffi` permissions and disk space

#### "rustdoc JSON not found"
- **Cause**: Cargo doc failed or rustdoc JSON format unavailable
- **Fix**: Ensure Rust toolchain is installed and up-to-date

#### "opaque handle type mismatch"
- **Cause**: Using a handle from a different crate or wrong type
- **Fix**: Ensure handle variable matches the function that created it

#### "function expects N arguments, got M"
- **Cause**: Argument count mismatch
- **Fix**: Check function signature in Rust docs or use `_try`/`_optjson` helpers

### Debug Mode

Enable debug logging to see bridge generation:

```bash
OTTER_LOG=debug otter run my_program.ot
```

This shows:
- Bridge crate generation
- Cache hits/misses
- Symbol registration
- Type conversions

## Limitations

### v1 Limitations

1. **Macros**: Rust macros are not exposed (only functions/types)
2. **Proc Macros**: Not supported
3. **Complex Generics**: Trait methods with complex generic parameters may not bridge correctly
4. **Lifetimes**: Borrowed references (`&T`) are restricted to call scope
5. **Unsafe Code**: Unsafe Rust functions require explicit allowlist in bridge.yaml

### Type Limitations

- **Slices**: Converted to opaque handles (copy-in/copy-out)
- **Tuples**: Converted to opaque handles
- **Enums**: Converted to opaque handles
- **Structs**: Converted to opaque handles (no field access yet)

## Future Enhancements

Planned improvements:
- Field access for struct types
- Better enum pattern matching
- Zero-copy slices for large data
- Stream support (async iterators)
- Macro exposure (experimental)

## Troubleshooting

### Bridge Not Found

If a crate isn't found:
1. Check crate name spelling
2. Ensure crate is in `Cargo.toml` or available via crates.io
3. Check `ffi/<crate>/bridge.yaml` exists if using custom config

### Build Failures

If bridge compilation fails:
1. Check Rust toolchain: `rustc --version`
2. Verify crate dependencies compile: `cargo check --manifest-path ffi/<crate>/Cargo.toml`
3. Clear cache and rebuild: `rm -rf ~/.otter_cache/ffi`

### Runtime Errors

If runtime errors occur:
1. Check function signature matches Rust docs
2. Verify argument types match (use `_try` helpers for Result/Option)
3. Enable debug logging to see FFI calls

## Performance Considerations

- **First Build**: Slow (compiles bridge crate + dependencies)
- **Cache Hits**: Fast (uses precompiled library)
- **Memory**: Opaque handles use reference counting (minimal overhead)
- **Async**: Uses Tokio runtime (shared across all async calls)

## Security

- **Sandboxing**: FFI functions run in the same process (no isolation)
- **Unsafe Code**: Explicitly allowlisted in bridge.yaml only
- **Input Validation**: Validate inputs before passing to FFI functions
- **Error Handling**: Always use `_try` helpers for Result types to handle errors gracefully

