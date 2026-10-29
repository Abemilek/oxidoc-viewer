#!/usr/bin/env bash
# Compila y empaqueta el APK. Requiere: rustup, JDK, Android SDK/NDK y cargo-apk (ver README.md)
set -euo pipefail

: "${ANDROID_HOME:?Define ANDROID_HOME (carpeta del Android SDK)}"
: "${ANDROID_NDK_ROOT:=${NDK_HOME:-}}"
: "${ANDROID_NDK_ROOT:?Define ANDROID_NDK_ROOT (o NDK_HOME) con la carpeta del NDK}"
export ANDROID_NDK_ROOT NDK_HOME="$ANDROID_NDK_ROOT" ANDROID_NDK_HOME="$ANDROID_NDK_ROOT"

command -v cargo-apk >/dev/null || cargo install cargo-apk

MODE="${1:-debug}"
if [ "$MODE" = "release" ]; then
  : "${CARGO_APK_RELEASE_KEYSTORE:?Para release define CARGO_APK_RELEASE_KEYSTORE y CARGO_APK_RELEASE_KEYSTORE_PASSWORD (ver README)}"
  cargo apk build --lib --release
else
  cargo apk build --lib
fi

echo
echo "APK generado:"
find target -name '*.apk' -newer Cargo.toml -print
echo
echo "Instalar:  adb install -r <ruta-al-apk>"
