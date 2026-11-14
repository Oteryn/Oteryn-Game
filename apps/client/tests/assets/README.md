# Native text input test font

`FiraMono-subset.ttf` is an unmodified 18,848-byte test fixture extracted from
`src/FiraMono-subset.ttf` in the existing, locked `bevy_text` 0.20.1 crate.
It supplies real glyph layout for native keyboard selection tests; an empty
font collection cannot meaningfully exercise Ctrl+A. Tests embed it with
`include_bytes!`; production UI continues to use its externally supplied font.
Tests do not search Cargo caches or require proprietary fonts.

- Crate archive: https://crates.io/api/v1/crates/bevy_text/0.20.1/download
- Crate SHA-256 (also pinned in `Cargo.lock`):
  `acee0f4cd88beed48dd1bce01759bc00fc68f9294d34fae4f5f63d881e50fea6`
- Extracted font SHA-256:
  `45131c5e913a5c7b4311101303e3c320a97b93a27a64b7262b533ee0d31ae03c`
- Upstream revision from the crate's `.cargo_vcs_info.json`:
  `a0577b5c6a7b208719dd761c5c6a7a539e472a97`
- Source: https://github.com/bevyengine/bevy/blob/a0577b5c6a7b208719dd761c5c6a7a539e472a97/crates/bevy_text/src/FiraMono-subset.ttf

Digitized data copyright (c) 2012-2015, The Mozilla Foundation and Telefonica S.A.
The font is distributed under the SIL Open Font License 1.1; see `OFL.txt`.
The license and attribution were copied from the same upstream revision's
[assets/fonts/FiraMono-LICENSE](https://github.com/bevyengine/bevy/blob/a0577b5c6a7b208719dd761c5c6a7a539e472a97/assets/fonts/FiraMono-LICENSE)
(Git blob `5e4608f24a4b050232daa61b2d0d13cfe10317b6`), with trailing
whitespace removed. No font data or font names were changed.
