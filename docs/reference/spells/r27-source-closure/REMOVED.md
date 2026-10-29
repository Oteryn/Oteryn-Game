# r27 source-closure removed

The r27 source-closure reference data (about 36 MB: monster source package tarball, player source projections, proofs and checksums) was removed on 2026-10-29 by owner decision. No code, CI, validator or content referenced it, and `../r28-source-closure/` supersedes it (regenerated at later pins; the files are not byte-identical).

Recover any file from the last commit that held it:

```
git show 329f78f8:docs/reference/spells/r27-source-closure/<path>
```
