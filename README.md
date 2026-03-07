# cpp-to-rust

**AI-powered C/C++ → Rust conversion agent.**

## Why C/C++

- Memory safety vulnerabilities account for ~70% of all security bugs (Microsoft, Google, NSA)
- Billions of lines of C/C++ in operating systems, browsers, databases, embedded systems
- The White House, NSA, and DARPA are all calling for memory-safe language adoption
- DARPA's TRACTOR program targets C→Rust, but focuses on formal verification. We focus on AI-driven practical conversion at scale.

## How It Works

```
C/C++ project (source + test suite)
    ↓ 1. Parse & analyze (headers, macros, templates, memory patterns)
    ↓ 2. AI converts each module to Rust
    ↓ 3. cargo check (must compile)
    ↓ 4. Run both C/C++ & Rust with same inputs, compare outputs
    ↓ 5. Diff? → AI fixes → goto 3
    ↓ 6. Repeat until all outputs match
Verified Rust binary
```

## Key Challenges

| C/C++ Feature | Conversion Strategy |
|---|---|
| Raw pointers | Ownership + borrowing analysis |
| Manual malloc/free | RAII, Box, Vec, Arc |
| Preprocessor macros | Rust macros or const/inline |
| C++ templates | Rust generics + traits |
| Undefined behavior | Eliminate entirely (Rust's guarantee) |
| Union types | Rust enums |
| Goto statements | Restructured control flow |

## Differentiation from DARPA TRACTOR

| | DARPA TRACTOR | cpp-to-rust |
|---|---|---|
| Approach | Formal verification + AI | AI agent + output comparison |
| Speed | Provably correct, slower | Practically correct, faster |
| Scope | Research | Production use |

## Status

**Concept.** Architecture design in progress.

## Part of [LegacyToRust Project](https://github.com/LegacyToRustProject)

## License

MIT
