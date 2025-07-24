!#/bin/bash

echo "Starting build..."

source .env

# Build docker image if it doesn't exist
docker build -t bot-builder -f Dockerfile.build . || exit 1

# Run cross-compilation
docker run -v "$(pwd):/app" -w /app bot-builder \
cargo build --target aarch64-unknown-linux-gnu || exit 1

echo "Copying binary to Raspberry Pi..."
scp target/aarch64-unknown-linux-gnu/debug/robo_rs $PI_HOST:$PI_PATH || exit 1

echo "Done! Binary copied to Pi"
