# Fonts

Inter by the Inter Project Authors, under the SIL Open Font License 1.1
(`Inter-LICENSE.txt`). Source: https://github.com/google/fonts/tree/main/ofl/inter

The bundled TrueType files are static instances of `Inter[opsz,wght].ttf`, with
optical size 18 and weights 500 (Medium) and 600 (SemiBold), generated using
fontTools. They are embedded in the Rust binary, so there are no font downloads
at runtime. TrueType outlines are used for consistent small-text rasterization.
