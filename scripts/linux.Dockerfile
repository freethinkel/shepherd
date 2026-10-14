# `make linux`: the CI runners' Ubuntu (its glibc is the oldest the packages run on), GPUI's Linux libraries, Zig for
# libghostty-vt and the packagers.
FROM ubuntu:22.04
ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates curl build-essential clang cmake pkg-config xz-utils dpkg-dev \
    libxkbcommon-dev libxkbcommon-x11-dev libwayland-dev libx11-xcb-dev libxcb1-dev \
    libvulkan-dev libfontconfig-dev libfreetype-dev libssl-dev libzstd-dev \
    && rm -rf /var/lib/apt/lists/*
RUN curl -fsSL https://sh.rustup.rs | sh -s -- -y --profile minimal --component clippy
ENV PATH=/root/.cargo/bin:$PATH
RUN cargo install --locked cargo-deb cargo-generate-rpm
RUN arch=$(uname -m) && curl -fsSL https://ziglang.org/download/0.16.0/zig-$arch-linux-0.16.0.tar.xz | tar -xJ -C /opt \
    && ln -s /opt/zig-$arch-linux-0.16.0/zig /usr/local/bin/zig
ENV ZIG=zig
