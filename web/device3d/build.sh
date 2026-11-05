#!/bin/sh
# Bundles the 3D device view (main.ts, t3code's viewer and three.js) into assets/device3d/viewer.js, which Shepherd
# compiles in. Run after changing anything here; the bundle is committed so a Rust build needs no Node.
set -e
cd "$(dirname "$0")"
npm install --no-audit --no-fund --silent
npx esbuild main.ts --bundle --format=iife --minify --target=safari16 --legal-comments=none \
  --outfile=../../assets/device3d/viewer.js
