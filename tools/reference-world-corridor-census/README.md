# Reference world corridor census

This is a thin Phase-A consumer of the protected
`tools/game-atlas-fullworld-source/producer.py` API. It streams the exact pinned
migration source once, retains only the two independent 32x32 floor `-7`
starting shards, and writes one canonical JSON summary. It does not parse OTBM,
select Global geometry, publish assets, or define production resource maxima.

```bash
python tools/reference-world-corridor-census/census.py \
  --legacy-root /path/to/Otheryn \
  --map /path/to/world.otbm \
  --asset-zip /path/to/15.32.zip \
  --assets /path/to/extracted/assets \
  --output /tmp/reference-world-corridor-census.json
```

The command fails before counting unless the legacy checkout and all four input
digests match the pinned producer contract. Edge occupancy is reported as
`AMBIGUOUS_EXPANSION`; it never causes an unproved adjacent-shard expansion.
Output classifications remain `MIGRATION_EVIDENCE / OTS_HYPOTHESIS_ONLY`.
