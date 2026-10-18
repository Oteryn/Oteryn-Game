# Oteryn-v2 licensing policy

## Default software license

Unless a file or directory contains a different explicit license notice, the following material in this repository is licensed under the Mozilla Public License 2.0 (`MPL-2.0`):

- source code, scripts and build tooling;
- configuration files, schemas and machine-readable contracts;
- tests, fixtures created for testing and examples;
- technical architecture, protocol and developer documentation;
- code-like gameplay definitions and data that are intended to be edited as source.

The complete license text is in [`LICENSE`](../../LICENSE).

The standard license text includes Exhibit B, but Oteryn-v2 does not attach or apply the separate Exhibit B incompatibility notice to covered source. The project is therefore not declared "Incompatible With Secondary Licenses".

New source files should use the SPDX identifier where the file format supports comments:

```text
SPDX-License-Identifier: MPL-2.0
```

A central `LICENSE` file may be used for formats where an inline notice is impractical or undesirable.

## Material outside the MPL grant

The MPL-2.0 grant does not automatically cover creative game assets or project branding. See:

- [`LICENSE-ASSETS.md`](../../LICENSE-ASSETS.md) for art, audio, maps, narrative and other creative assets;
- [`TRADEMARKS.md`](../../TRADEMARKS.md) for names, logos and product identity.

`LICENSE-ASSETS.md` is the repository-wide explicit notice for qualifying creative assets, including assets added outside a dedicated assets directory. A file-specific license or a nearer directory-level license overrides the repository default for the material it governs. Third-party material must retain its original license, attribution and provenance.

## Contributions

Unless an accepted contribution explicitly identifies different terms that the project is legally able and willing to accept, contributions to MPL-covered material are submitted under MPL-2.0.

By submitting a contribution, the contributor represents that they created it or have sufficient rights to provide it under the stated terms. Contributions must not include proprietary or incompatibly licensed third-party code, assets, data, fonts, media, maps or documentation.

The project does not currently require copyright assignment or a Contributor License Agreement. Consequently, accepting an external contribution under MPL-2.0 does not by itself grant the project owner a separate right to relicense that contributor's work under a proprietary license. A future dual-licensing programme would require a separately reviewed contributor agreement or contributor consent.

## Dependencies and imported material

The license of this repository does not replace dependency licenses. Every dependency, imported component and converted data source must be reviewed for compatibility and documented with its provenance and applicable notices before distribution.

Generated output inherits licensing according to the material and tools from which it is produced; generation does not erase upstream obligations.

### Embedded client fonts

The unmodified fonts embedded by `epaint_default_fonts` **0.36.2** remain under
their upstream licenses, including SIL Open Font License 1.1 and Ubuntu Font
Licence 1.0. Both permit embedding and redistribution with software while
retaining the fonts' licenses and copyright notices. These terms govern the
fonts; they do not replace the MPL-2.0 license of Oteryn source code. Font files
are not renamed, relicensed or sold separately.

`deny.toml` permits those two additional licenses only for that exact package
version. [`FONT-PROVENANCE.json`](../../apps/client/FONT-PROVENANCE.json) records
the verified crates.io archive and original notice/font hashes.
[`THIRD-PARTY-FONTS.txt`](../../apps/client/THIRD-PARTY-FONTS.txt) preserves the
upstream copyright and full license texts for Hack, Noto Emoji, Ubuntu Light
and emoji-icon-font, together with the separately licensed Cinzel brand font.
The Windows installer installs this notice alongside the launcher; its checks
verify the installed notice matches the source. Any future font or dependency
version change requires a fresh compatibility and provenance review.

### Windows clipboard adapters

The unmodified, locked `clipboard-win@5.4.1` and `error-code@3.4.0` packages
are used by `arboard@3.6.1` through `egui-winit@0.35.0` for the real Windows
clipboard. Their original manifests declare **Boost Software License 1.0**
(`BSL-1.0`), not the Business Source License. Boost permits use, modification
and distribution with MPL-covered software; it does not relicense these crates.
Source redistribution must retain upstream copyright notices and the complete
license. The license explicitly exempts solely machine-executable object code
from that notice requirement. No upstream source is vendored or modified here.

`deny.toml` grants exceptions only to these two exact package versions, without
adding BSL to the global allow-list or weakening advisory/source/bans checks.
The crates.io archive SHA-256 values were verified against `Cargo.lock`:

| Package | Archive SHA-256 | Original manifest SHA-256 |
| --- | --- | --- |
| clipboard-win 5.4.1 | `bde03770d3df201d4fb868f2c9c59e66a3e4e2bd06692a0fe701e7103c7e84d4` | `039fe75560600788fffd9b1b9a771e9e1ef8a9b651b629de11cbfae56f38f7ba` |
| error-code 3.4.0 | `0b5343afd4a8365a643ac588dab4cf234a190c7f6c88c9f6dd6ffe00837661b7` | `a01638c3f4cba4714c706444cfb337546495bdb4be2b26193c2b27e3403ca347` |

The packaged error-code LICENSE and the [clipboard-win LICENSE at its recorded
VCS revision](https://github.com/DoumanAsh/clipboard-win/blob/3b27cf2bfd1adcfa6e0264eb51c1025ddaf0f342/LICENSE)
have SHA-256 `c9bff75738922193e67fa726fa225535870d2aa1059f91452c411736284ad566`.
The clipboard archive excludes LICENSE, so its upstream license was checked
separately. error-code records VCS revision
`e4615e514db3ff64f5f1b328b4865b20a3dcdbc3` in
[`DoumanAsh/error-code`](https://github.com/DoumanAsh/error-code/tree/e4615e514db3ff64f5f1b328b4865b20a3dcdbc3).
Both manifests identify the upstream author as Douman. A version change requires
a new provenance/compatibility check, not a widened exception.

## Questions and exceptions

Licensing exceptions are governance changes. They require an explicit file- or directory-level notice, provenance evidence, compatibility review and a pull request updating this policy when the exception affects repository-wide expectations.

This policy explains the intended repository licensing boundary. It is not legal advice.
