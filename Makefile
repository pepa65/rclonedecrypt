# Makefile for rclone-decrypt

.PHONY: build release test clean install cross-compile help

# Default target
all: build

# Build in debug mode
build:
	cargo build

# Build in release mode
release:
	cargo build --release

# Run tests
test:
	cargo test

# Integration test (requires rclone)
integration-test: release
	./test.sh

# Clean build artifacts
clean:
	cargo clean
	rm -rf dist/

# Install locally
install: release
	cargo install --path .

# Cross-compile for multiple platforms
cross-compile:
	./build.sh

# Check code for errors
check:
	cargo check
	cargo clippy -- -D warnings
	cargo fmt --check

# Format code
fmt:
	cargo fmt

# Show help
help:
	@echo "Available targets:"
	@echo "  build           - Build in debug mode"
	@echo "  release         - Build in release mode"  
	@echo "  test            - Run unit tests"
	@echo "  integration-test - Run integration tests (requires rclone)"
	@echo "  clean           - Clean build artifacts"
	@echo "  install         - Install locally with cargo"
	@echo "  cross-compile   - Build for multiple platforms"
	@echo "  check           - Check code with clippy and formatting"
	@echo "  fmt             - Format code"
	@echo "  help            - Show this help message"
