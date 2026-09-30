#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
if [[ "$(uname -s)" != Darwin ]]; then
  echo 'iOS linking requires macOS and Xcode.' >&2
  exit 1
fi
ios_target="${1:-aarch64-apple-ios}"
case "$ios_target" in
  aarch64-apple-ios|aarch64-apple-ios-sim|x86_64-apple-ios) ;;
  *) echo "Unsupported iOS target: $ios_target" >&2; exit 1 ;;
esac
rustup target add "$ios_target"
cargo build --locked --release --target "$ios_target"
mkdir -p dist/Jarcade.app
cp "target/$ios_target/release/jarcade" dist/Jarcade.app/
cp ios/Info.plist dist/Jarcade.app/
echo 'Built unsigned dist/Jarcade.app. Device installation requires signing.'
