# Fih artwork

The five room backgrounds are original, hand-authored SVGs. They use a small
pastel palette, flat fills, rounded furniture and open space around Fih.
No generated images, external assets or textures are used.

Edit `kitchen.svg`, `bathroom.svg`, `bedroom.svg`, `playroom.svg` or `clinic.svg`.
The shared viewBox is `0 0 390 844`; the wall and floor expand to the viewport
while furniture keeps its horizontal proportions. Supported elements are
`rect` (including `rx`), `circle`, `ellipse`, and `polygon`, with six-digit
hexadecimal fills. The Rust reader validates and caches the shapes once, then
Macroquad draws them directly as geometry on native and web. Tests parse every
room and reject malformed geometry or unsupported elements.

Fih is also original code-native artwork, using curved meshes in
`src/fih_character.rs`. Clothes, hats, food and icons are in `src/fih_art.rs`.
Its body silhouette and care/animation geometry are shared with the pure
interaction rules. The user's fish reference is not bundled.
