# Independent data-only Weapon Proficiency import review

Reviewed `/workspace/Oteryn-WP-Import` native importer and CLI read-only on 2026-10-02. No Cargo, repository edits, remote actions or paid review were performed.

No remaining material blocker found in the bounded full-data import of committed definitions/bindings and the complete accepted progression ruleset. The initial P1 closed-schema omissions (`Shard.shard`, `Bindings.source`) are repaired with explicit typed fields; shard ranges/counts and the compile-time binding source digest are checked. Negative cases now corrupt the actual binding source digest and shard count, targeting these repaired checks.

The current catalogue contains 443 definitions and 665 bindings. An independent Python check against all 69 committed Item shards (34,032 definitions) found zero missing Item references or revision mismatches. Proficiency targets are resolved against the imported definition map with exact target revision, family, canonical Item key and duplicate checks. Levels/perks retain source order and use the owning closed perk model and validator. Root reports its actual CLI export and independent Python Decimal/Fraction comparison passed for all 443 definitions, 3,671 perks and 665 bindings; this is reported evidence, not a test rerun by this reviewer.

The earlier ruleset-preservation qualification is resolved: `progression_ruleset: Value` retains the complete parsed committed rules document, including identity/revision/provenance, track, credit and points, and exposes only a borrowed read-only accessor. It is also included in the serialized CLI output. Accepted thresholds/mastery still receive the owning domain cross-check; inert retention of the remaining fields grants no execution authority. No gameplay activation, Character writes or new authority admission occurs in the importer/CLI.

Reviewed importer SHA-256: `6e4289ef0ca634a7d62e04ed71f68a0544b63ca2c623eb7e7a07bc5e849abf49`.
Reviewed CLI SHA-256: `6ea557e9fc1beae69fa9b294930569318def6cdbd56bb7585ac0fcd7954abd8d`.

Final supplementary full-rules equality, native tests and Clippy are deliberately left to the root's already-running qualification; this report does not claim those pending checks passed.
