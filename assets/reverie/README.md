# Dicksit artwork

84 original surreal storytelling illustrations generated with the built-in
`image_gen` model for Jarcade. No existing Dixit card art was supplied or copied.
The full production prompt and all 84 scene descriptions are in
[`prompts.json`](prompts.json).

Each `atlas-01.jpg` through `atlas-21.jpg` is a 1024×1536 texture with four
512×768 full-bleed portraits, ordered top-left, top-right, bottom-left,
bottom-right. Card IDs 0–83 address those quadrants directly by UV rectangle.
The model's original PNGs were converted to JPEG at quality 90, with no resize
or artistic editing. Native-resolution PNG originals remain in the generator's
local artifact directory; the compact production assets are committed here.

The first atlas is embedded for the arcade preview. Web loads other atlases
only when a hand/table needs them; native builds bundle the same JPEGs. Texture
caches retain only the current pictures and preview. Failed web loads can be
retried. These images are exclusive to Dicksit; Fih remains code and SVG art.
