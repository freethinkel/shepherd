# `make linux-test`, `linux-lint` and `linux-shot`: the Linux build image (linux.Dockerfile) with git for the tests'
# repos, and Xvfb, xdotool, ImageMagick and Mesa's lavapipe (Vulkan on the CPU) to draw the window without a screen.
FROM shepherd-linux
RUN apt-get update && apt-get install -y --no-install-recommends \
    git xvfb xdotool imagemagick mesa-vulkan-drivers libvulkan1 fonts-dejavu-core fonts-noto-color-emoji \
    && rm -rf /var/lib/apt/lists/*
RUN git config --system user.name shepherd && git config --system user.email shepherd@localhost \
    && git config --system --add safe.directory '*'
