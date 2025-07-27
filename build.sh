#!/bin/bash

echo "Starting build..."

PI_HOST="ryan@10.0.0.36"
PI_PATH="~/"

# Build docker image if it doesn't exist
docker build -t bot-builder -f Dockerfile.build . || exit 1

# Create temp directory and copy source
TEMP_DIR="/tmp/robo_rs_build"
rm -rf "$TEMP_DIR"
cp -r "$(pwd)" "$TEMP_DIR"

# Run cross-compilation
docker run -v "$TEMP_DIR:/app" -w /app bot-builder \
cargo build --target aarch64-unknown-linux-gnu || exit 1

# Copy binary back
cp "$TEMP_DIR/target/aarch64-unknown-linux-gnu/debug/robo_rs" "target/aarch64-unknown-linux-gnu/debug/" 2>/dev/null || mkdir -p "target/aarch64-unknown-linux-gnu/debug/" && cp "$TEMP_DIR/target/aarch64-unknown-linux-gnu/debug/robo_rs" "target/aarch64-unknown-linux-gnu/debug/"

echo "Copying binary to Raspberry Pi..."
scp target/aarch64-unknown-linux-gnu/debug/robo_rs $PI_HOST:$PI_PATH || exit 1

echo "Done! Binary copied to Pi"
