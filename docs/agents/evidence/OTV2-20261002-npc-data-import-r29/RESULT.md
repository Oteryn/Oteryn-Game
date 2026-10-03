# NPC data-only server import R29

Owner explicitly requests importing all prepared NPC data while accepting incomplete runtime subsystems. The native server now has an immutable source-pinned NPC catalogue importer and an explicit CLI/opt-in serving-process reader. The existing1282 NPC,836 Dialogue and380 NPC-linked Service records, source history and approximation flags remain unchanged. Existing parser/capture/types are reused; no source format, protocol, placement, active-world binding or service execution is added.

See docs/migration/NPC_DATA_ONLY_IMPORT.md for commands and boundaries. Actual checks/import counts and independent source/code review are recorded in validation.json. No owner-PC project actions or production deployment.

Actual server-binary import PASS:1282 NPC,836 Dialogue,380 NPC-linked Service and2564 linked profiles; imported source tree `3fcab27b778849cb2453d34161fd16c9cbd58c0bff0d00c54fc12e5583c78148`. Ten selected checks passed, including four catalogue tests, strict format/Clippy, normal bootstrap smoke, rejection of an incorrect pin and missing serve pin, and governance/diff checks. Independent code/source preservation review passed. No full live-node startup or production deployment was run.
