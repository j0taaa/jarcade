#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
cargo build --locked --release --target wasm32-unknown-unknown
mkdir -p dist/vendor
cp web/navigation.js web/platform.js web/multiplayer.js web/pwa.js web/manifest.webmanifest dist/
cp -r web/icons dist/
mkdir -p dist/assets/reverie
cp assets/reverie/atlas-*.jpg dist/assets/reverie/
cp web/vendor/* dist/vendor/
cp target/wasm32-unknown-unknown/release/jarcade.wasm dist/
python3 - <<'PY'
import hashlib, json, re
from pathlib import Path
paths = [Path('dist/jarcade.wasm'), Path('web/platform.js'), Path('web/navigation.js'), Path('web/multiplayer.js'), *sorted(Path('assets/reverie').glob('atlas-*.jpg')), Path('web/vendor/mq_js_bundle.js'), Path('web/index.html'), Path('web/pwa.js'), Path('web/sw.js'), Path('web/manifest.webmanifest'), *sorted(Path('web/icons').glob('*')), Path('src/navigation.rs')]
version = hashlib.sha256(b''.join(path.read_bytes() for path in paths)).hexdigest()[:12]
Path('dist/index.html').write_text(Path('web/index.html').read_text().replace('__BUILD__', version))
core = ['index.html', 'manifest.webmanifest', 'jarcade.wasm', 'navigation.js', 'platform.js', 'multiplayer.js', 'pwa.js', 'vendor/mq_js_bundle.js', *[p.relative_to('dist').as_posix() for p in sorted(Path('dist/icons').glob('*'))]]
precache = []
for name in core:
    path = Path('dist') / name
    url = '/' + path.relative_to('dist').as_posix()
    if path.suffix in ['.js', '.wasm']:
        url += '?v=' + version
    precache.append({'url': url, 'sha256': hashlib.sha256(path.read_bytes()).hexdigest()})
routes = re.findall(r'=> "(/[^"\n]*)"', Path('src/navigation.rs').read_text())
assert '/' in routes and '/games/sudoku/play' in routes
worker = Path('web/sw.js').read_text().replace('__BUILD__', version).replace('__PRECACHE__', json.dumps(precache)).replace('__ROUTES__', json.dumps(routes))
Path('dist/sw.js').write_text(worker)
print(f'Web build: {version}')
PY
echo 'Built dist/. Preview with routes: cargo run --release --features server --bin jarcade-server'
