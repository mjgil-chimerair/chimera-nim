# Build Orchestration

Chimera-Nim uses a unified build system via `cargo xtask`.

## Prerequisites

| Component | Version | Install |
|-----------|---------|---------|
| Rust | 1.75+ | `rustup install 1.75` |
| OCaml | 4.14+ | `opam install ocaml` |
| Zig | 0.12+ | `ziglang.org` |
| CMake | 3.16+ | `apt install cmake` |
| Dune | 3.0+ | `opam install dune` |
| Clang/LLVM | 14+ | `apt install clang` |

## Using xtask

Build all components:
```bash
cargo xtask build
```

Build specific language:
```bash
cargo xtask build --lang rust
cargo xtask build --lang ocaml
cargo xtask build --lang zig
cargo xtask build --lang cpp
```

Run all tests:
```bash
cargo xtask test
```

Run tests for specific language:
```bash
cargo xtask test --lang rust
cargo xtask test --lang ocaml
```

Check independent configurability:
```bash
cargo xtask check-independent
cargo xtask check-independent --lang rust
```

Verbose output:
```bash
cargo xtask build -v
cargo xtask test -v
```

## Language-Specific Commands

### Rust
```bash
cargo build --workspace
cargo test --workspace
```

### OCaml
```bash
cd ocaml && make build
cd ocaml && make test
```

### Zig
```bash
cd zig && zig build
```

### C++
```bash
mkdir -p cpp/build && cd cpp/build && cmake .. && make
```

## CI Verification

Each language root is verified independently in CI:
1. Rust: `cargo build --workspace`
2. OCaml: `dune build --root ocaml`
3. Zig: `zig build --root zig`
4. C++: `cmake .. && make` in cpp/build