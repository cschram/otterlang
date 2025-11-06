## OtterLang Roadmap

This roadmap outlines major areas of investment and their intended scope. Items are prioritized by impact and feasibility and grouped by milestone tiers. Timelines are indicative and may evolve based on feedback.

### Guiding Principles
- Simplicity first: syntax and tooling should be easy to learn and hard to misuse
- Performance by default: native codegen, low overhead abstractions
- Safety with explicit control: predictable resource and error handling
- Batteries included: great stdlib, formatter, profiler, and REPL

---

### Milestone 1: Error Handling and Language Ergonomics
- Result/Option return-based errors
  - Add `Result<T, E>` and `Option<T>` as first-class types
  - Standard library helpers for mapping/combining results
  - CLI and formatter awareness
- Syntactic sugar for simplicity
  - `?` operator to early-return on error from `Result<T, E>`
  - `try` expression support for concise error flows
- Fatal internal errors
  - `panic`/`raise` for unrecoverable states with backtrace hooks
- Pattern matching integration
  - Exhaustive `match` on `Result`/`Option` and algebraic data types
  - Destructuring and guards in `match` arms

### Milestone 2: Runtime and JIT Capabilities
- Per-function JIT
  - Julia-style per-function JIT for hot paths
  - Tiered execution (interp/JIT/native) with profiling feedback
- Live module loading
  - Reloadable modules for REPL and long-lived processes
  - Stable ABI for safe hot-swapping within a process

### Milestone 3: Tooling and Developer Experience
- Testing framework
  - Built-in test runner (`otter test`) and assertion library
  - Snapshot testing hooks and timing output
- Diagnostics and observability
  - Rich error messages with spans and suggestions
  - Built-in tracing and structured logs
- Formatter and LSP
  - Incremental formatter improvements
  - First-party LSP features (hover, go-to-def, diagnostics)

### Milestone 4: Libraries and Ecosystem
- Standard library depth
  - Collections, iterators, async utilities
  - IO/FS/Net ergonomics and safety improvements
- FFI bridges
  - Wider crate support and improved metadata extraction
  - Sandboxed loading and permission controls

---

### Acceptance Criteria (Examples)
- Error handling
  - A function returning `Result<T, E>` can be composed with `?`
  - `match` on `Result` is exhaustive and type-checked
- JIT
  - A hot function receives JIT compilation with measurable speedup
  - Live module swapping retains state isolation guarantees
- Testing
  - `otter test` discovers and runs tests, returning non-zero on failure
  - Assertions report clear diffs and spans

### Out of Scope (for now)
- Distributed runtime
- Advanced macro system or compile-time evaluation

### Feedback
Feedback is welcome via issues and discussions. Priorities will be adjusted based on real-world usage.


