#!/usr/bin/env bash
# Build and package the APK. Requires rustup, a JDK, the Android SDK/NDK, and cargo-apk (see README.md).
set -euo pipefail

: "${ANDROID_HOME:?Define ANDROID_HOME (Android SDK directory)}"
: "${ANDROID_NDK_ROOT:=${NDK_HOME:-}}"
: "${ANDROID_NDK_ROOT:?Define ANDROID_NDK_ROOT (or NDK_HOME) with the NDK directory}"
export ANDROID_NDK_ROOT NDK_HOME="$ANDROID_NDK_ROOT" ANDROID_NDK_HOME="$ANDROID_NDK_ROOT"

command -v cargo-apk >/dev/null || cargo install cargo-apk

MODE="${1:-debug}"
if [ "$MODE" = "release" ]; then
  : "${CARGO_APK_RELEASE_KEYSTORE:?For release builds, set CARGO_APK_RELEASE_KEYSTORE and CARGO_APK_RELEASE_KEYSTORE_PASSWORD (see README)}"
  cargo apk build --lib --release
else
  cargo apk build --lib
fi

echo
echo "APK generated:"
find target -name '*.apk' -newer Cargo.toml -print
echo
echo "Install:  adb install -r <path-to-apk>"
