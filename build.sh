#!/bin/bash

echo "Starting build..."

PI_HOST="ryan@10.0.0.36"
PI_PATH="~/jumo_2/"

# Build docker image if it doesn't exist
docker build -t bot-builder -f Dockerfile.build . || exit 1

# Run cross-compilation
docker run -v "$(pwd):/app" -w /app bot-builder \
cargo build --target aarch64-unknown-linux-gnu || exit 1

echo "Copying binary and .env to Raspberry Pi..."
scp target/aarch64-unknown-linux-gnu/debug/robo_rs $PI_HOST:$PI_PATH || exit 1

echo "Done! Binary and .env copied to Pi"
