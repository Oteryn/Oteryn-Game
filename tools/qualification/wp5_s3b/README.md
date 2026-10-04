# Existing S3-B qualification topology

`run.sh` reuses the S3-A Platform containers and the S3-B Character-intent overlay.
It requires Docker Compose/BuildKit, OpenSSL, Rust 1.94.0, an exact Platform
checkout at the revision pinned in the script, and an isolated PostgreSQL 17.6
service selected through `OTERYN_TEST_POSTGRES_ADMIN_URL`. That database service
is supplied separately; the runner does not create it. Its administration rights
must be scoped to disposable qualification state. Host `sudo` is not required.

Select `WP5_QUALIFICATION=s3b` (default), `seam` (entry-room TCP/TLS), or
`spell-seam` (full spell input over the same real-owner TCP/TLS path). Spell mode
accepts `OTERYN_SEAM_SPELL_MANIFEST`; its default is the preserved full manifest.
The shared entry point is `bash tools/qualification/spells/run.sh server`.
Services are temporary and cleaned up; this runner does not deploy content to an
existing long-lived test or production server.

## Spell scenario scope

The full-content scenario prepares one fresh disposable Sorcerer at level 8
through the normal fenced progression/build writers, disconnects and waits for
ordinary grace release, then rejoins to load its actual resources. It requires
Exura to consume mana and an immediate retry to return `CoolingDown` without a
second payment. It then probes every loaded catalog index with `target=none` and
records the actual disposition.

A free level-8 caster and an untargeted request cannot validate every vocation,
Premium spell, rune reagent, target damage, monster AI or quest encounter. Those
outcomes are diagnostic coverage; only explicit successful assertions qualify
execution. Full-health Exura does not measure wounded healing magnitude.

The native dispatcher selects the existing whole-player self owner before the
broader ordinary-combat route for eligible instant self spells. Targeted spells,
areas, chains and runes retain their original owner paths. This enables baseline
self effects without inventing absent magnitude facts. Advanced numeric modifier
parity is not qualified by this scenario; its absence must not be reported as
complete spell coverage. The independent magnitude owner remains required for
its target and world-combat paths.
