# chimera-nim

`chimera-nim` is a Rust-first reimplementation of the Nim compiler and toolchain, with OCaml, Zig, and C++ kept in bounded support roles for experiments and backend integration.

## Repository layout

- `crates/` - Rust compiler crates and the `chimera` CLI
- `xtask/` - unified build and test entrypoints
- `docs/` - design notes and repository documentation
- `ocaml/` - prototype labs used as comparison oracles
- `zig/` - narrow runtime and ABI helper experiments
- `cpp/` - narrow C++ bridge for backend integration

## Workspace highlights

- `chimera` - command-line compiler driver
- `rnim_session` - compiler session and configuration handling
- `rnim_span` - source files, spans, and line mapping
- `rnim_diag` - structured diagnostics
- `rnim_lexer` - Nim tokenization and indentation handling
- `rnim_parser` - lossless parsing
- `rnim_ast` - typed AST wrappers
- `rnim_symbols` - symbol interning and name resolution support
- `rnim_hir`, `rnim_mir` - intermediate representations
- `rnim_types`, `rnim_sema`, `rnim_effects` - typing and semantic analysis
- `rnim_macro`, `rnim_vm` - compile-time expansion and evaluation
- `rnim_codegen_c`, `rnim_codegen_js` - backend code generation
- `rnim_runtime`, `rnim_stdlib` - runtime and standard-library support
- `rnim_nimble`, `rnim_docgen`, `rnim_suggest`, `rnim_test`, `rnim_fuzz`, `rnim_tools`, `rnim_build`, `rnim_query` - package, tooling, testing, and build support crates

## Build

```bash
cargo build --workspace
```

```bash
cargo run -p xtask -- build
```

## Test

```bash
cargo test --workspace
```

```bash
cargo run -p xtask -- test
```

## Documentation

- Repository and build notes: `docs/repo-layout.md`, `docs/build.md`
- Architecture overview: `docs/design/index.md`
- Detailed subsystem notes: `docs/design/`

## ChimeraIR Binary Builds

This repo does not currently carry the ChimeraIR manifests for the three binary
variants. The current build surface for those binaries lives in
`../chimera-beam`, using the shared `chimera` CLI from
`../chimerair/tools`.

Build the CLI first:

```bash
cd ../chimerair/tools
cargo build --release -p chimera-cli
```

Then build the three binary variants from `chimera-beam`:

```bash
HOST_TRIPLE=x86_64-unknown-linux-gnu
CHIMERA=../chimerair/tools/target/release/chimera

cd ../chimera-beam
"$CHIMERA" build --manifest Chimera.toml --target "$HOST_TRIPLE" --output ./build-abi
"$CHIMERA" build --manifest Chimera.adapter.toml --target "$HOST_TRIPLE" --output ./build-adapter
"$CHIMERA" build --manifest Chimera.separate.toml --target "$HOST_TRIPLE" --output ./build-semantic
```

The resulting executables are emitted as `build-abi/chimera_binary`,
`build-adapter/chimera_binary`, and `build-semantic/chimera_binary`.

## License

This project is licensed under the 0BSD license. See `LICENSE`.
