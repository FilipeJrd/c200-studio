# The official Rust toolchain is installed on the supported glibc baseline.
FROM ubuntu:22.04 AS build
ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl clang libclang-dev build-essential pkg-config libx11-dev libxi-dev libxrandr-dev libxcursor-dev libxkbcommon-dev libwayland-dev libgl1-mesa-dev libegl1-mesa-dev libudev-dev libdbus-1-dev file patchelf desktop-file-utils && rm -rf /var/lib/apt/lists/*
ENV CARGO_HOME=/opt/cargo RUSTUP_HOME=/opt/rustup
ENV PATH=/opt/cargo/bin:${PATH}
RUN curl --fail --location https://sh.rustup.rs -o /tmp/rustup.sh && sh /tmp/rustup.sh -y --profile minimal --default-toolchain 1.99.0 --no-modify-path
RUN cargo install cargo-about --version 0.9.2 --locked --features cli
RUN apt-get update && apt-get install -y --no-install-recommends libxkbcommon-x11-dev && rm -rf /var/lib/apt/lists/*
WORKDIR /src
COPY . .
RUN cargo build --release --locked -p c200-app
RUN bash scripts/appimage.sh

FROM build AS verify
RUN cargo test --workspace --locked
RUN apt-get update && apt-get install -y --no-install-recommends xvfb xauth && rm -rf /var/lib/apt/lists/*
RUN LIBGL_ALWAYS_SOFTWARE=1 WINIT_UNIX_BACKEND=x11 timeout 25s xvfb-run -a dist/C200-Studio-x86_64.AppImage --appimage-extract-and-run --demo --smoke-test --screenshot dist/ubuntu22-demo.png

FROM scratch AS artifacts
COPY --from=verify /src/dist/C200-Studio-x86_64.AppImage /
COPY --from=verify /src/dist/C200-Studio-x86_64.AppImage.sha256 /
COPY --from=verify /src/dist/ubuntu22-demo.png /
