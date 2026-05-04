# Zig Support for Chimera-Nim

This directory contains Zig modules for low-level runtime and ABI helpers.

## Purpose

Zig is used only for specific cases where it provides clear advantages over Rust:
- Compact allocators
- Path/OS probes
- Narrow backend/runtime helpers
- Target-layout helpers

## Modules

### rnim_zig_support
Core support library with:
- RnimAllocator: Simple allocator wrapper
- Platform probes
- FFI helpers for C ABI

## Building

```bash
zig build
zig build test
```

## Ownership

Zig helpers are backend support only. They do not own semantic or MIR pipeline.

## Prohibited

- Zig must NOT own frontend semantics
- Zig must NOT implement type inference
- Zig must NOT implement macro/template expansion