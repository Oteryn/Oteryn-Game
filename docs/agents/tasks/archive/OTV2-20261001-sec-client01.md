# OTV2-20261001-sec-client01

```yaml
task_id: OTV2-20261001-sec-client01
title: "SEC-CLIENT-01 client integrity and anti-bot layers"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/arch-sec-client-01
pr: "assigned at PR creation; recorded in the #162 FREEZE_SHA entry"
base_sha: 92eaf21b
head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_frozen_at: null
owner: claude-code-session_016c5MQoe5CoMk9fxmuuMcFJ (Sol Supervising Architect)
created_at: 2026-10-01
updated_at: 2026-10-01
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_SEC_CLIENT01_CLIENT_INTEGRITY_AND_ANTI_BOT_DECISION_2026-10-01.md
  - docs/agents/tasks/archive/OTV2-20261001-sec-client01.md
  - docs/architecture/FND-02_PROTOCOL_OTERYN_V1_CONTRACT.md
  - docs/architecture/reviews/OTERYN_GAME_ADMIT0_WORLD_REQUIRED_CAPABILITIES_DECISION_2026-10-01.md
public_contracts:
  - docs/architecture/FND-02_PROTOCOL_OTERYN_V1_CONTRACT.md
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

SEC-CLIENT-01 closes the horizon gate `SEC-CLIENT-01` (`REQUIRED_FOR_ALPHA`) with owner answer 5b,
layered version (#162 5929226991).

- **Layer 1:** a signed release manifest and a Game-side trust store with an ordered, root-signed
  trust record chain anchored in an append-only trust log outside the Game database and verified
  against a fresh root-signed checkpoint (no challenges while unverified), and emergency revocation; `CLIENT_INTEGRITY_V1`
  with a nonce-salted code-region challenge at login (alpha) and in game (before open beta). A
  correct answer proves only access to official release bytes, never integrity; outcomes are never
  sanction evidence on their own; one terminal outcome per challenge, unsolicited replies
  coalesced; retiring builds keep their corpus until their sessions end.
- **Layer 2:** `INPUT_TELEMETRY_V1` with a closed, statistical summary; server-side timing weighs
  more; no opt-out, disclosed in the privacy policy; summaries accepted by server-owned windows,
  silence recorded once per silent period; a closed list of nine artifact kinds, each with every
  ANL-01 §16 field stated, anything else prohibited (case evidence, and with it case-based identity resolution, until a case profile exists; review stays at signal level and pseudonymous; only a system-run PRIVACY_DSR resolution serves deletion and export requests), and a
  production collection gate on their binding and the DATA-PRIVACY-01 disclosure.
- **Layer 3:** outcomes and telemetry are ANL-01 security events; ANL-03 hypothesis signals; GM
  review and ban waves through OPS-GM-01 before open beta; nothing automatic.
- **Admission:** native transport profile 1 requires both capabilities through a transport profile
  set added to ADMIT-0's predicate.
- **Attestation:** `ClientBootstrap` field 8 set aside.
- **Owner questions:** none; R1-R2 are architect rulings.

No code, migration or content change is made.

## Architecture and source of truth

- `PROVEN`: the horizon gate; FND-02 §9, §11, §20; FND-04 and FND-04A-C; ANL-03 §2, §4, §14; the
  crash diagnostics privacy baseline; owner answer 5b.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: docs only. Children need security, privacy and protocol review.

## Acceptance criteria

- [ ] Decision on an exact frozen head with passing validators.
- [ ] Independent exact-head review (security, privacy, protocol).
- [ ] Protected Merge Queue integration.

## Excluded scope

- Code; attestation; vendors; sanctions (OPS-GM-01).

## Validation

- `python3 tools/agents/validate_governance.py`: PASS on the authoring tree.
- `python3 tools/repository/validate_repository_policy.py`: PASS on the authoring tree.
- `git diff --cached --check`: clean.

## Closeout

- PR: recorded in the #162 FREEZE_SHA entry. Merge commit/result: its squash merge.
- Amendments, each pending on acceptance of SEC-CLIENT-01: FND-02 §11; ADMIT-0 §3.1.
- This record was archived in the PR's final authoring commit (`docs/agents/tasks/archive/README.md`).

```yaml
last_progress: final authoring commit; archived before freeze
status: completed
branch: claude/arch-sec-client-01
owner_action_required: null
blocker: null
next_action: null
```
