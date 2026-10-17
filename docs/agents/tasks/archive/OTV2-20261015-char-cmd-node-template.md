# OTV2-20261015-char-cmd-node-template

```yaml
task_id: OTV2-20261015-char-cmd-node-template
title: "CHAR-CMD-NODE-TEMPLATE: commented [platform.character_commands] example in the Synology node template"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/cc-node-template-20261015
issue: 1622
pr: 2212
head_sha: "the head reported to the control plane"
final_head_sha: "the head reported to the control plane"
owner: claude-code worker for control plane session_01XYC239givrMonrYoBwwECV
created_at: 2026-10-15
updated_at: 2026-10-15
execution_policy: continuous_progress
owned_paths:
  - deploy/synology-game/node.toml.template
  - docs/agents/tasks/archive/OTV2-20261015-char-cmd-node-template.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: GAME-CHAR-CMD-0
external_repositories: [Oteryn/Oteryn-Platform]
```

## Outcome

- `deploy/synology-game/node.toml.template` shows the `[platform.character_commands]` table (contract
  `oteryn-game-character-authority-commands-v1`) as a commented example after
  `[platform.account_characters]`: certificate and key under `<BASE>/node/secrets`, source authority
  `oteryn:character-authority:primary`.
- The table stays commented. The consumer runs only when the table is present, so nothing is enabled.
  The owner uncomments it at §14 step 4, after Platform enables the commands for the certificate's
  subject (Platform runbook `docs/operations/CHARACTER_AUTHORITY_COMMANDS_ENABLEMENT.md`; Platform
  deploy wiring in Oteryn/Oteryn-Platform PR 1633).

## Architecture and source of truth

- PROVEN (code): `apps/game-server/src/node/config.rs` reads `[platform.character_commands]` with
  `client_certificate_file`, `client_key_file` and `source_authority`. It refuses paths shared with the
  `[platform]`, runtime-status or projection identities, and refuses a source authority equal to the
  Platform, readiness, bootstrap-intent or purpose names.
- PROVEN (test): `deploy/synology-game/test-preprod-topology.sh` renders the template and passes.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: a comment in a deploy template. No Rust, persistence, wire or enablement change.

## Acceptance criteria

- [x] Commented example with the three keys the node config reads.
- [x] Nothing enabled; no secret or subject value committed.

## Excluded scope

- No change to the consumer code, `preprod-topology.toml` or the deploy scripts.
- No enablement.

## Validation

### Focused

- `git diff --check`: pass
- `bash deploy/synology-game/test-preprod-topology.sh`: pass
- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass

### Component/integration

- `NOT_APPLICABLE`: template comment only.

### E2E

- `NOT_APPLICABLE`: template comment only; enablement is an owner step.

## Self-review

- method/reviewer: task writer. I checked the keys against `node/config.rs` and its
  `character_commands_section_is_optional_off_by_default_and_separate` test.
- material findings: none open
- verdict: ready for review

## PR and closeout

- changed-file review: two owned paths
- protected auto-merge: control plane
- ownership release: on merge

## Context checkpoint

```yaml
last_progress: commented [platform.character_commands] example added to the node template
status: completed
branch: claude/cc-node-template-20261015
pr: 2212
blocker: null
next_action: control plane reviews and merges; the owner uncomments the table at enablement
```
