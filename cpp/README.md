# C++ Backend Bridge for Chimera-Nim

This directory contains a narrow C++ bridge for LLVM or external optimizer integration.

## Purpose

C++ is used only for specific backend integration cases:
- LLVM bindings when required
- Optimizer integration
- Target-specific ABI helpers

## Components

### rnim_ffi_cpp
Core FFI bridge library with:
- LlvmBridge: LLVM integration wrapper
- CompilationResult: Result type for compilation operations
- C ABI exports for Rust consumption

## Building

```bash
mkdir build
cd build
cmake ..
make
```

## Ownership

C++ backend is isolated from frontend semantics. It consumes MIR export data and produces object files.

## Prohibited

- C++ must NOT own frontend or semantic passes
- C++ must NOT implement Nim parsing or type inference
- C++ is ONLY for backend integration