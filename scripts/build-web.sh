#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
cargo build --locked --release --target wasm32-unknown-unknown
mkdir -p dist/vendor
cp web/navigation.js web/platform.js web/multiplayer.js dist/
mkdir -p dist/assets/reverie
cp assets/reverie/atlas-*.jpg dist/assets/reverie/
cp web/vendor/* dist/vendor/
cp target/wasm32-unknown-unknown/release/jarcade.wasm dist/
python3 - <<'PY'
import hashlib
from pathlib import Path
paths = [Path('dist/jarcade.wasm'), Path('web/platform.js'), Path('web/navigation.js'), Path('web/multiplayer.js'), *sorted(Path('assets/reverie').glob('atlas-*.jpg')), Path('web/vendor/mq_js_bundle.js'), Path('web/index.html')]
version = hashlib.sha256(b''.join(path.read_bytes() for path in paths)).hexdigest()[:12]
Path('dist/index.html').write_text(Path('web/index.html').read_text().replace('__BUILD__', version))
print(f'Web build: {version}')
PY
echo 'Built dist/. Preview with routes: cargo run --release --features server --bin jarcade-server'
