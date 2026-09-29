# content/assets/files (temporary staging)

This is the owner-requested temporary folder for Oteryn asset files, such as sprites, images and sounds. The folder is a stopgap. Its final layout and the storage for large binaries (plain Git or Git LFS) still need an architecture decision. Files may move when that decision lands.

- **Allowed:** files that Oteryn owns or for which Oteryn holds sufficient rights or a licence to redistribute. This includes third-party client assets only when those redistribution rights are confirmed.
- **Current owner-approved snapshot (2026-09-29):** 6,248 files / 129,713,346 bytes copied from the owner's current local Tibia package `assets` folder after the owner confirmed redistribution rights. All 6,248 files match their SHA-256 entries in `imports/official/client-assets/15.30/manifest.json`. That older manifest has one additional 122,882,530-byte `.bmp.zip` entry which is absent from the current source folder and therefore is not committed here.
- When `content/assets/catalog/` is populated, record each file there with its identity, SHA-256 and licence/provenance information.
- **Without confirmed redistribution rights:** do not commit original third-party binary or media assets; retain only checksums/reference evidence.
