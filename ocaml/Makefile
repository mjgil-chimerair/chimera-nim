# OCaml Labs Makefile for Chimera-Nim

.PHONY: all test build clean install-deps

# Install OCaml dependencies
install-deps:
	@if command -v opam >/dev/null 2>&1; then \
		opam install --yes dune menhir; \
	else \
		echo "opam not found. Please install OCaml toolchain."; \
	fi

# Build all OCaml labs
build:
	cd chimera_nim_parser_lab && dune build
	cd chimera_nim_type_lab && dune build
	cd chimera_nim_macro_lab && dune build
	cd chimera_nim_concept_lab && dune build

# Run tests
test:
	cd chimera_nim_parser_lab && dune exec ./test.exe
	cd chimera_nim_type_lab && dune exec type_lab.ml
	cd chimera_nim_macro_lab && dune exec macro_lab.ml
	cd chimera_nim_concept_lab && dune exec concept_lab.ml

# Clean build artifacts
clean:
	dune clean

# Run all tests
all: install-deps build test