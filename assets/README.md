# Fonts

Inter by the Inter Project Authors, under the SIL Open Font License 1.1
(`Inter-LICENSE.txt`). Source: https://github.com/google/fonts/tree/main/ofl/inter

The bundled TrueType files are static instances of `Inter[opsz,wght].ttf`, with
optical size 18 and weights 500 (Medium) and 600 (SemiBold), generated using
fontTools. They are embedded in the Rust binary, so there are no font downloads
at runtime. TrueType outlines are used for consistent small-text rasterization.

## Game artwork

Fih uses original Rust character geometry and hand-authored SVG rooms; see
[fih/README.md](fih/README.md). Coupe uses original Rust vector portraits in
`src/card_art.rs`. Dicksit has 84 original generated paintings; see
[reverie/README.md](reverie/README.md) for production assets and prompts.
