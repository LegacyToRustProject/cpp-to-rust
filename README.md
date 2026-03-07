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

## Version Compatibility

C and C++ are treated as separate tracks due to vast differences in complexity.

### C Track

| C Standard | Priority | Notes |
|------------|----------|-------|
| C99 | **First** | Most common in production. Linux kernel baseline. |
| C11 | **First** | Threads, atomics. Modern systems code. |
| C17 | Second | Bug fixes over C11. Minor changes. |
| C89/ANSI C | Third | Oldest codebases. Simpler = easier to convert. |
| C23 | Fourth | Latest. Limited adoption so far. |

### C++ Track

| C++ Standard | Priority | Notes |
|--------------|----------|-------|
| C++17 | **First** | Sweet spot: modern features, wide adoption. |
| C++11/14 | **First** | Lambdas, move semantics. Large existing codebase. |
| C++20 | Second | Concepts, ranges, coroutines. Growing adoption. |
| C++03 | Third | Pre-modern C++. Enterprise legacy. |
| C++23 | Fourth | Latest. Limited adoption. |

Older C is simpler to convert. Older C++ (pre-11) is verbose but predictable. Modern C++ templates and metaprogramming are the hardest challenge.

Auto-detection: `cpp-to-rust analyze` detects the standard used via compiler flags and feature usage.

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
