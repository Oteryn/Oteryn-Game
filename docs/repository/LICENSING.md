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

## Questions and exceptions

Licensing exceptions are governance changes. They require an explicit file- or directory-level notice, provenance evidence, compatibility review and a pull request updating this policy when the exception affects repository-wide expectations.

This policy explains the intended repository licensing boundary. It is not legal advice.
