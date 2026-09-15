# Game -> Atlas physical-profile readiness spike

Status: **research-only / non-canonical / non-production**.

This directory contains a deterministic, stdlib-only synthetic measurement tool used by `OTV2-20260816-game-atlas-physical-profile-readiness` / issue #291.

It exists to gather bounded evidence before a later contract decides whether a first executable Game -> Atlas physical profile can be frozen. Nothing in this directory is a public schema, runtime parser, production resource limit, canonical coordinate model or selected serializer.

## What it measures

The spike uses only project-owned synthetic records with stable Oteryn-style keys. No OTBM, Crystal/Canary data, Tibia/CipSoft assets or proprietary fixtures are used.

It compares:

- `canonical-json-v0`;
- `canonical-jsonl-v0`;
- `binary-baseline-v0` — a deliberately research-only lower-bound comparator, **not a proposed public schema**;
- `32x32` and `64x64` grid packaging;
- per-floor and all-floors-per-grid packaging.

For each matrix cell it checks/measures:

- deterministic byte identity for identical inputs;
- semantic encode/decode round-trip;
- raw and deterministic-gzip size;
- file/chunk count and p95/max chunk size;
- one-point and representative viewport compressed bytes;
- local single-record edit changed-file granularity;
- textual diff line count **and maximum line byte length for textual encodings**, so compact one-line JSON is not falsely treated as equally reviewable as record-oriented JSONL;
- SHA-256 corruption detection and gzip corruption detection.

Text-diff metrics are deliberately `null` for `binary-baseline-v0`. Binary bytes can accidentally form valid UTF-8 and must not be interpreted as meaningful source-control text simply because decoding happens to succeed.

The script's internal record/string/chunk caps protect the research harness itself. They are **not production limits** and must never be copied into `RESOURCE_LIMITS_REGISTRY.json` without independent owning evidence.

## Run

```bash
python tools/game-atlas-profile-spike/spike.py \
  --self-test \
  --output game-atlas-physical-profile-report.json \
  --summary
```

The dedicated GitHub Actions workflow runs the same command on the exact PR head and prints both the compact summary and machine-readable report into immutable workflow logs.

## Interpretation guardrail

A smaller artifact is not automatically the preferred production format. The later profile decision must also account for canonical World coordinate authority, consumer/browser implementation cost, parser safety, compatibility, evolution, observability and exact production resource ceilings.

Likewise, source-control text-review quality is only one axis. A published Atlas artifact may not be reviewed as ordinary source files, so JSONL's record-oriented diff advantage cannot by itself select the public distribution format.

The binary baseline is intentionally hand-bounded and deterministic solely so it can show a rough binary-size/locality lower bound without adding a new dependency or silently selecting a public encoding.
