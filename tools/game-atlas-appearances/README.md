# Game -> Atlas animated appearances

`export.py` is the Game-owned offline conversion boundary for exact 15.32 appearance animation metadata. It validates the immutable source identity, decodes the pinned protobuf semantics without a runtime dependency, produces a deterministic/content-addressed `animated-appearances-v1` catalog and exposes explicit object/outfit resolution helpers.

```bash
python tools/game-atlas-appearances/export.py /path/to/15.32.zip /tmp/animated-appearances
python tools/game-atlas-appearances/verify.py /tmp/animated-appearances
python tools/game-atlas-appearances/self_test.py
```

The output is metadata only. Do not commit the raw archive, appearance file or sprite sheets. Atlas browser runtime must consume the normalized product/publication and never parse those source formats.
