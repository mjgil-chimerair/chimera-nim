# OCaml Labs for Chimera-Nim

This directory contains OCaml prototype implementations used as reference oracles for the Rust implementation.

## Packages

### chimera_nim_parser_lab
Parser prototype for Nim. Used to verify parser correctness by comparing outputs.

### chimera_nim_type_lab
Type inference prototype. Used to validate Rust type inference against OCaml oracle results.

### chimera_nim_macro_lab
Macro expansion prototype. Models template/macro phase boundaries for Rust implementation de-risking.

### chimera_nim_concept_lab
Concept solving prototype. Validates Rust concept constraint resolution.

## Building

```bash
cd ocaml
opam install --yes dune menhir
dune build
dune test
```

## Testing Protocol

1. Run OCaml oracle on shared fixture
2. Run Rust implementation on same fixture
3. Compare outputs
4. If mismatch, investigate which is correct and update accordingly

## Ownership

OCaml prototypes are research artifacts. When stable, their lessons are incorporated into Rust. OCaml code is not shipped as part of the product.