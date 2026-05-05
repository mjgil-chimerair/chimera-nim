# Workspace Design

## Crate Boundaries

The chimera-nim workspace is organized into the following crates:

### Core Compiler Infrastructure

| Crate | Type | Purpose |
|-------|------|---------|
| `chimera` | binary | Main compiler driver |
| `rnim_allocator` | library | Arena allocator for AST/HIR |
| `rnim_session` | library | Global session, config, target triples |
| `rnim_span` | library | Source spans, file mapping |
| `rnim_diag` | library | Diagnostics emission |
| `rnim_lexer` | library | Tokenization |
| `rnim_parser` | library | Lossless parsing |
| `rnim_ast` | library | Typed AST wrappers |

### Symbol Resolution and HIR

| Crate | Type | Purpose |
|-------|------|---------|
| `rnim_symbols` | library | Symbol interning, module graph |
| `rnim_hir` | library | High-level IR |
| `rnim_types` | library | Type representation |
| `rnim_sema` | library | Semantic analysis |

### Analysis and Transformation

| Crate | Type | Purpose |
|-------|------|---------|
| `rnim_effects` | library | Effect tracking |
| `rnim_macro` | library | Macro expansion |
| `rnim_vm` | library | Compile-time VM |
| `rnim_mir` | library | Mid-level IR |
| `rnim_lifetime` | library | ARC/ORC lowering |

### Code Generation

| Crate | Type | Purpose |
|-------|------|---------|
| `rnim_codegen_c` | library | C backend |
| `rnim_codegen_js` | library | JavaScript backend |
| `rnim_build` | library | Build orchestration, compile/link plans |

### Runtime and Standard Library

| Crate | Type | Purpose |
|-------|------|---------|
| `rnim_runtime` | library | Runtime support |
| `rnim_stdlib` | library | Standard library |
| `rnim_nimble` | library | Nimble integration |

### Tooling

| Crate | Type | Purpose |
|-------|------|---------|
| `rnim_docgen` | library | Documentation generation |
| `rnim_suggest` | library | Nimsuggest server |
| `rnim_test` | library | Test runner |
| `rnim_fuzz` | library | Fuzzing targets |
| `rnim_query` | library | Query system for IDE features |
| `rnim_tools` | library | Formatter/utilities |

## Dependency Direction Rules

The following dependency rules are enforced:

1. **No cycles**: The dependency graph must be acyclic
2. **No frontend-backend cycles**: Frontend crates (lexer, parser, ast, symbols, hir, types, sema) cannot depend on backend crates (mir, codegen_c, codegen_js, lifetime, build)
3. **Span is foundational**: `rnim_span` has no dependencies on other workspace crates
4. **Runtime is standalone**: `rnim_runtime` has no dependencies on other workspace crates
5. **Stdlib is standalone**: `rnim_stdlib` has no dependencies on other workspace crates

## Allowed Dependency Edges

```
chimera -> rnim_session, rnim_lexer, rnim_parser, rnim_ast, rnim_diag, rnim_symbols, rnim_hir, rnim_types, rnim_sema, rnim_build, rnim_allocator

rnim_session -> rnim_span

rnim_diag -> rnim_span

rnim_lexer -> rnim_span

rnim_parser -> rnim_span, rnim_lexer, rnim_diag

rnim_ast -> rnim_span, rnim_parser, rnim_allocator

rnim_symbols -> rnim_span, rnim_ast, rnim_allocator

rnim_hir -> rnim_span, rnim_ast, rnim_symbols, rnim_allocator

rnim_types -> rnim_span, rnim_ast

rnim_sema -> rnim_span, rnim_ast, rnim_hir, rnim_types, rnim_effects

rnim_effects -> rnim_span, rnim_hir

rnim_macro -> rnim_ast, rnim_vm, rnim_span, rnim_allocator

rnim_query -> rnim_span, rnim_ast, rnim_sema

rnim_vm -> rnim_span, rnim_ast

rnim_mir -> rnim_span, rnim_hir

rnim_lifetime -> rnim_span, rnim_hir, rnim_mir

rnim_codegen_c -> rnim_span, rnim_mir

rnim_codegen_js -> rnim_span, rnim_mir

rnim_build -> rnim_span

rnim_docgen -> rnim_span, rnim_ast

rnim_suggest -> rnim_session, rnim_lexer, rnim_parser, rnim_ast, rnim_sema, rnim_query

rnim_test -> rnim_session, rnim_parser

rnim_fuzz -> rnim_lexer, rnim_parser

rnim_tools -> rnim_span, rnim_parser
```

## Unsafe Code Policy

**Deny-by-default** - Unsafe code is prohibited in all crates except:

- `rnim_runtime` - Memory management, thread primitives, reference counting
- `rnim_codegen_c` - C interop, pointer manipulation, FFI
- `rnim_codegen_js` - JS interop, wasm manipulation

All unsafe code must be documented with:
- Invariants that must be maintained
- Safety justification
- Expected behavior if invariants are violated

This policy is enforced via `Cargo.toml` lints and CI validation.

## Build Configuration

- `profile.release`: opt-level 3, LTO enabled, codegen-units 1
- `profile.dev`: opt-level 0
- `profile.dev.package."*"`: opt-level 1 (dependencies optimized for faster incremental builds)

## Deny-by-Default Lints

The workspace configures deny-by-default lints for:
- `unsafe_code` - Denied except in allowlisted crates
- `dead_code` - Warn to keep codebase clean
- `unused_variables` / `unused_imports` - Warn
- `nonstandard_style` - Warn
- `ragged_comments` - Warn