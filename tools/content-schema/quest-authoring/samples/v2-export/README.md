Quest v2 Source-authoring overlay (offline candidate)

The existing accepted ordinary v2 seam is role `declarative-definitions` at `definitions/declarations.json`, schema `OTERYN_WORLD_PROJECT_DECLARATIONS/v2` (server `content/project/v2.rs:18,33,1376,1497`). Quest declarations carry identity and scalar CandidateFields. QuestAuthoring profiles are optional, tagged `{"kind":"Quest","profile":...}`; absent requirement/reward/reference fields remain absent. CandidateField is explicitly Source observation without executable interpretation (`v2.rs:536`). Current content/world declaration file demonstrates the same document and Text metadata field pattern.

Run:
  python quest_v2_export.py --repo-root REPO --out declarations.json --report report.json
  python -m unittest -q test_quest_v2_export
  python quest_v2_export.py --check

Installed authoring module uses repo-root=Path(__file__).parents[3] and defaults outputs to samples/v2-export/{declarations,report}.json. --check compares deterministic bytes and never writes. Recommended run_checks invocation: [sys.executable, authoring / "quest_v2_export.py", "--check"]. Final canonical regeneration requires export regeneration, then --check. Full canonical reference pointer/hash covers optional chosen_journal_corrections as well; these are Source evidence only.

Export reads all 352 canonical Quest definitions. It emits 351 declarations, 309 chosen profiles (241 donor completion recipes plus 68 authored recipes), and 42 Source-catalogue-only declarations. Existing ProductionKey rejects the `test` segment in `oteryn:quest.barbarian_test`; this record remains an explicit identity hold, with no invented alias. Coordinator/architect must authorize its canonical identity repair or an accepted identity mapping.

Only chosen recipe requirement min_level→required_level (u16), premium→premium (bool), and repeat.kind once/daily→repeatable (bool) are projected. Unknowns, wrong types, overflows and unsupported repeat kinds are omitted. Zero and false are retained when explicitly chosen. Daily cycle details remain Source Text, not implemented cooldown behavior. No free-text Item/Achievement/Encounter/Quest name becomes a production reference. Stages/repeat/reward intents are retained as bounded Source Text annotations. Full canonical Source data stays at original path/hash/pointer references. No existing readiness, runtime_enabled, native admission or executable effect is promoted.

Output is an overlay, not a replacement of the existing declarations file or a complete loadable project. Integration must merge exact identities into the existing package, preserve other families, regenerate accepted manifest/lock, and qualify the complete parser snapshot. Server does not dereference or execute Source metadata strings.

Qualification is source-backed Python closed-shape validation plus meaningful negative fixtures; no compiled server parser was available at the inspected workspace target locations, so no Rust parser run is claimed. Actual parser rejects unknown fields and candidate nested object values. `validate_v2_state` checks sorted unique declarations/profiles and reference existence (`v2.rs:2610,2685`). All output profiles target output declarations.

Ordinary filesystem capture accepts caller-selected ProjectFilesystemLimits/ProjectEvidenceLimits (`project_fs.rs:135`); these are evidence limits, not production maxima. Repository full-family test uses document96MB/total160MB/decodedfields2,120,000/stringbytes43MB/reference records finite full-family count (`tests/content_world_project_repository.rs:125,145`). The standalone overlay is about1.32MB and does not establish the merged package budget. Native-entry capture uses fixed first-slice64KB/4096 fields, requires its overlay and separate admission path; this export exceeds that byte budget and is NOT native-entry admitted. Do not weaken either limit or claim production load.
