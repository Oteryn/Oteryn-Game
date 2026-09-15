# OTV2-20260811-first-reference-baseline-dossier — archived

```yaml
task_id: OTV2-20260811-first-reference-baseline-dossier
title: Prepare exact first Reference baseline owner decision dossier
mode: COORDINATE
status: completed
repository: blakinio/Oteryn-v2
delivery_branch: docs/OTV2-20260811-first-reference-baseline-dossier
delivery_pr: 179
base_sha: 3853127dfccf7df2421dfe0a6c63714f19e828ff
final_head_sha: d8b2e54439b6fa711bad5f687fbe281a32c6cac8
delivery_merge_sha: 8f126ef0c54fbfa1098cb6cc549570daa13e736a
lifecycle_closeout_pr: 180
owner: released
completed_at: 2026-08-11T19:20:00+02:00
implementation_status: NOT_APPLICABLE
```

## Outcome

Delivered and merged the nonbinding first Reference baseline decision dossier:

- `docs/architecture/GAME-VISION-01_FIRST_REFERENCE_BASELINE_DECISION_DOSSIER.md`.

The dossier remains `PRE-DECISION ANALYSIS / NOT ACCEPTED` and recommends, without accepting, **Global Tibia production behavior after the 2026-07-28 server-save/maintenance change boundary** as the first Oteryn Reference target.

## Key result

- target date and evidence completeness are separate;
- unknown mechanics remain `UNKNOWN` rather than guessed;
- official public evidence is primary but not assumed exhaustive;
- controlled black-box observation is permitted evidence with provenance;
- community sources are corroborative/discovery inputs;
- Canary/crystalserver/other OTS are hypothesis/inventory inputs, not proof of Global behavior or production authority;
- Reference revisions remain immutable and later Global changes require explicit later revision promotion;
- security/integrity/legal/provenance overrides remain binding;
- accepting a target would only unblock evidence-backed GAME-CHAR Stage B; it would not accept Stage B or authorize implementation.

## Delivery evidence

- final head: `d8b2e54439b6fa711bad5f687fbe281a32c6cac8`;
- self-review: `4908860655` — PASS, 0 material findings;
- Agent Governance `31516361255` / #826 — success;
- Dependency Review `31516361296` / #590 — success;
- CodeQL `31516361250` / #714 — success;
- unresolved review threads at merge: 0;
- branch behind main at merge: 0;
- squash merge: `8f126ef0c54fbfa1098cb6cc549570daa13e736a`;
- runtime/schema/content/Platform/production authority: NONE.

## External evidence boundary

Official Tibia chronology reviewed on 2026-08-11 established the July 13 Summer Update and subsequent July 14, July 16, July 21 and July 28 changes. It did not establish complete proof that no later behavior-changing production change occurred through August 11. The dossier therefore preserves post-July-28 chronology completeness as `UNKNOWN`; absence of a found patch note is not evidence of absence.

## Next owner decision

Accept, replace or modify the recommended package in dossier section 21. Until owner acceptance, no exact first Reference target is canonical and GAME-CHAR Stage B remains hard-blocked.
