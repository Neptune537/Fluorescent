# Cargo NDK Build Recipe

To ship the `fluoderpod_render` library for Android via `cargo-ndk` and potentially prebuild it in CI, follow this recipe.

## Prerequisites
1. Install `cargo-ndk`:
   ```bash
   cargo install cargo-ndk
   ```
2. Add the Android target toolchains:
   ```bash
   rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android i686-linux-android
   ```

## Local Build Script
Create a script (e.g., `build_android.sh`) to build the minimal render pipeline without the heavy networking/XR features:

```bash
#!/bin/bash
set -e

# Compile for 64-bit ARM (most modern Android devices)
# Note: We use --no-default-features and just enable android-render
cargo ndk -t arm64-v8a -t armeabi-v7a -t x86_64 \
    -o ../fluorescent/packages/fluorescent_vulkan/android/src/main/jniLibs \
    build --release \
    --no-default-features \
    --features "android-render"

echo "Android libraries built and copied to jniLibs folder!"
```

## CI Prebuilt `.so` GitHub Actions Recipe
If you want to build this automatically in CI and upload the `.so` files as artifacts:

```yaml
name: Build Android Renderer

on:
  push:
    branches: [ "main" ]
  release:
    types: [ "created" ]

jobs:
  build-android:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: aarch64-linux-android, armv7-linux-androideabi, x86_64-linux-android
          
      - name: Setup Android NDK
        uses: nttld/setup-ndk@v1
        with:
          ndk-version: r26b

      - name: Install cargo-ndk
        run: cargo install cargo-ndk

      - name: Build with cargo-ndk
        working-directory: ./fluoderpod_render
        run: |
          cargo ndk -t arm64-v8a -t armeabi-v7a -t x86_64 \
            -o ./jniLibs \
            build --release --no-default-features --features "android-render"

      - name: Upload .so Artifacts
        uses: actions/upload-artifact@v4
        with:
          name: fluoderpod-android-libs
          path: ./fluoderpod_render/jniLibs/
```
