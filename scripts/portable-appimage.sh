#!/usr/bin/env bash
set -euo pipefail
PROJECT_ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$PROJECT_ROOT"
mkdir -p dist
if docker buildx version >/dev/null 2>&1; then
  docker build --output type=local,dest=dist --target artifacts .
else
  # Support systems that only have the legacy Docker builder installed.
  docker build --target verify --tag c200-studio-appimage-build .
  CONTAINER_ID="$(docker create c200-studio-appimage-build)"
  trap 'docker rm "$CONTAINER_ID" >/dev/null' EXIT
  docker cp "$CONTAINER_ID:/src/dist/C200-Studio-x86_64.AppImage" dist/
  docker cp "$CONTAINER_ID:/src/dist/C200-Studio-x86_64.AppImage.sha256" dist/
  docker cp "$CONTAINER_ID:/src/dist/ubuntu22-demo.png" dist/
fi
