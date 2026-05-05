# Rust Nim Design Documentation

Canonical entry point for chimera-nim compiler design documentation.

## Crate Architecture

| Crate | Description |
|-------|-------------|
| `chimera` | Main compiler binary |
| `rnim_allocator` | Arena allocator for AST/HIR |
| `rnim_session` | Compiler session, config, target triples |
| `rnim_span` | Source files, byte spans, line/column mapping |
| `rnim_diag` | Structured diagnostics, labels, suggestions |
| `rnim_lexer` | Full Nim lexer |
| `rnim_parser` | Lossless CST parser |
| `rnim_ast` | Typed AST wrappers over CST |
| `rnim_symbols` | Symbol interning, module graph, scopes |
| `rnim_hir` | Resolved high-level IR |
| `rnim_types` | Type representation, inference, conversions |
| `rnim_sema` | Semantic analysis |
| `rnim_effects` | Effect lattice, `raises`, tags |
| `rnim_macro` | Template/macro expansion |
| `rnim_vm` | Compile-time VM |
| `rnim_mir` | Backend-neutral MIR |
| `rnim_lifetime` | ARC/ORC lowering, move analysis |
| `rnim_codegen_c` | C backend |
| `rnim_codegen_js` | JavaScript backend |
| `rnim_build` | Build/link orchestration |
| `rnim_runtime` | Runtime ABI |
| `rnim_stdlib` | Minimal `system` module |
| `rnim_nimble` | Nimble manifest parsing |
| `rnim_docgen` | Documentation generation |
| `rnim_suggest` | Nimsuggest protocol server |
| `rnim_test` | Conformance test runner |
| `rnim_fuzz` | Fuzz targets |
| `rnim_query` | Query system for IDE features |
| `rnim_tools` | Formatter, reducer, snapshot updater |

## Design Documents

### Foundation
- [Workspace and Build Configuration](workspace.md) - Crate boundaries, dependency direction, unsafe policy
- [Build and CI](../build.md) - Build configuration, CI quality gates
- [Repository Layout](../repo-layout.md) - Multi-language ownership and artifact flow

### Core Infrastructure
- [Diagnostics](diagnostics.md) - Structured diagnostics, JSON/human output

### Backends
- [C Backend](codegen-c.md) - C code generation
- [JS Backend](codegen-js.md) - JavaScript code generation