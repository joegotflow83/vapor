#!/bin/bash

# Continuous Deployment script for Vapor Rust application
# This script builds the application for multiple platforms based on the tag

set -e  # Exit on any error

echo "Starting CD process for tagged version..."

# Get the tag from environment or git
TAG="${GITHUB_REF#refs/tags/}"
if [ -z "$TAG" ]; then
    echo "Error: No tag found"
    exit 1
fi

echo "Deploying tag: $TAG"

# Function to build for a specific platform
build_for_platform() {
    local platform=$1
    local target=$2
    local binary_name="vapor-${platform}"

    echo "Building for $platform ($target)..."

    # Build the release version for the target platform
    cargo build --release --target "$target"

    # Create a zip file with the binary
    if [ "$platform" = "windows" ]; then
        # Windows binaries have .exe extension
        binary_name="${binary_name}.exe"
        zip "${binary_name}.zip" "target/${target}/release/${binary_name}"
    else
        # Unix-like systems (Linux, macOS)
        tar -czf "${binary_name}.tar.gz" -C "target/${target}/release" "${binary_name}"
    fi

    echo "Built $platform package successfully"
}

# Build for all platforms
echo "Building for all platforms..."

# Linux (x86_64)
build_for_platform "linux" "x86_64-unknown-linux-gnu"

# macOS (x86_64)
build_for_platform "macos" "x86_64-apple-darwin"

# Windows (x86_64)
build_for_platform "windows" "x86_64-pc-windows-gnu"

echo "All builds completed successfully!"
echo "Packages created:"
ls -la vapor-*.tar.gz vapor-*.zip

# If this is a release, we would upload to GitHub releases here
# For now, we'll just create the artifacts locally