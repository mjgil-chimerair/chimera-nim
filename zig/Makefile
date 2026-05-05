# Zig Support Makefile for Chimera-Nim

.PHONY: all test build clean

# Check if zig is available
ZIG := $(shell command -v zig 2>/dev/null || echo "")

# Build all Zig modules
build:
ifneq ($(ZIG),)
	cd rnim_zig_support && zig build
else
	@echo "zig not found. Please install Zig toolchain."
endif

# Run tests
test:
ifneq ($(ZIG),)
	cd rnim_zig_support && zig build test
else
	@echo "zig not found. Please install Zig toolchain."
endif

# Clean build artifacts
clean:
ifneq ($(ZIG),)
	rm -rf zig-cache
	rm -rf rnim_zig_support/zig-cache
endif

# Show Zig version
version:
ifneq ($(ZIG),)
	zig version
else
	@echo "zig not found"
endif

all: build test