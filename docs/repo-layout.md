# Repository Layout

This document defines the four-language repository layout for chimera-nim.

## Top-Level Ownership

| Directory | Language | Purpose |
| --------- | -------- |---------|
| `crates/` | Rust | Primary product path - compiler frontend, middle-end, and C/JS backends |
| `ocaml/` | OCaml | Prototype labs for parser, type inference, macro, and concept oracle comparison |
| `zig/` | Zig | Runtime helpers, allocators, platform probes, native backend experiments |
| `cpp/` | C++ | LLVM backend bridge, optimizer integration, ABI verification helpers |
| `tests/fixtures/` | Cross-language | Shared test fixtures for all language components |
| `docs/` | — | Design documents, specifications, and user-facing documentation |

## Artifact Flow

```
Source (Nim)
    │
    ▼
┌─────────────────┐
│  Rust Compiler  │ (crates/rnim_*)
│  Frontend/MIR   │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  C Backend      │ (crates/rnim_codegen_c)
│  (Primary)      │
└────────┬────────┘
         │
         ▼
    C Compiler ──────► Executable

Alternative backends:
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│  JS Backend     │     │  Zig Helpers    │     │  C++ LLVM       │
│  (rnim_codegen_js)    │  (zig/)          │     │  (cpp/)         │
└─────────────────┘     └─────────────────┘     └─────────────────┘
```

## Ownership Boundaries

### Rust (crates/)
- **Owns**: All compiler frontend semantics, HIR, MIR, type inference, template/macro expansion, C/JS backends
- **Prohibited**: Must not delegate frontend semantics to other languages

### OCaml (ocaml/)
- **Owns**: Parser oracle, type inference oracle, macro oracle, concept oracle prototypes
- **Prohibited**: Must not own production compiler semantics; prototypes are comparison references only

### Zig (zig/)
- **Owns**: Low-level runtime helpers, allocators, OS/platform probes, native backend experiments
- **Prohibited**: Must not own frontend or semantic passes; narrow C ABI only

### C++ (cpp/)
- **Owns**: LLVM bridge, optimizer integration, target-specific ABI helpers
- **Prohibited**: Must not own frontend or semantic passes; narrow C ABI only

### Shared ABI Schema

Located in `docs/` if needed. Version contract managed via CI compatibility checks.

## CI Verification

Each language root must configure independently:
- `crates/` - `cargo build --workspace`
- `ocaml/` - `dune build`
- `zig/` - `zig build`
- `cpp/` - `cmake .. && make`

CI runs language-specific builds on every PR to verify independent configurability.