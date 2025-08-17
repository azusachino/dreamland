#!/bin/bash

# Build script for Dreamland Image Viewer
echo "Building Dreamland Image Viewer..."

# Check if rust is installed
if ! command -v cargo &> /dev/null; then
    echo "Error: Cargo/Rust not found. Please install Rust first."
    exit 1
fi

# Run tests first
echo "Running tests..."
cargo test --lib

# Check code compilation
echo "Checking compilation..."
cargo check

echo "Build complete! You can run the application with:"
echo "  cargo run"