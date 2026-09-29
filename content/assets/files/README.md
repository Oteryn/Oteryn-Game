# content/assets/files (temporary staging)

This is the owner-requested temporary folder for Oteryn's own asset files, such as sprites, images and sounds. The
folder is a stopgap. Its final layout and the storage for large binaries (plain Git or Git LFS) still need an
architecture decision. Files may move when that decision lands.

- **Allowed:** files that Oteryn owns or holds a licence for. Record each file in `content/assets/catalog/` when the
  catalog is populated: its identity, SHA-256 and licence.
- **Never allowed:** CipSoft or Tibia client files (`appearances*.dat`, sprite sheets, `.lzma`, sounds, maps and the
  like). Only their checksums live in the repository, in `imports/official/client-assets/15.30/manifest.json`.
