# OTV2-20260930-starter-backpack0

```yaml
task_id: OTV2-20260930-starter-backpack0
title: "STARTER-BACKPACK-0 starter backpack"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/arch-starter-backpack-0
pr: "exact PR in the #162 FREEZE_SHA entry"
base_sha: 370fc338
head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_frozen_at: null
owner: claude-code-session-01KbqAgmFfAYDSKHKkFmWKWW (Sol Supervising Architect)
created_at: 2026-09-30
updated_at: 2026-09-30
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_STARTER_BACKPACK0_STARTER_GRANT_DECISION_2026-09-30.md
  - docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md
  - docs/architecture/reviews/OTERYN_GAME_CHARACTER_REVISION_ITEM_TRANSACTION_COMPOSITION_DECISION_2026-09-27.md
  - docs/agents/tasks/archive/OTV2-20260930-starter-backpack0.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

STARTER-BACKPACK-0 decides who provisions the main backpack (#162 5914960502, answering the C2
escalation 5914818272).

- **Ruling:** a separate item-only writer, STARTER-1, after fresh admission; not the Character
  bootstrap and not CHAR-BUILD-1.
- **Template:** `StarterKit` records resolved from the root's immutable `starter_template_revision`
  (GAME-CHAR-01 Stage B §6); V1 has one record, the backpack `oteryn:item.tibia.i2854`.
- **Grant:** once per (Character, template key), forever; `MINTED` or `SKIPPED_SLOT_OCCUPIED`;
  server-originated with no CommandRef; guard branches, a deferred consistency trigger and a new
  audit operation.
- **Amendments (pending):** DUR-03 (a MINT shape into the container slot) and composition rule 1
  with a server-originated fence variant.

No code, migration or content change is made. No owner question: Stage B §6 already accepts
starter inventory as template content.

## Architecture and source of truth

- `PROVEN`: `character_authority.rs`, migrations `0005`, `0009`, `0011`-`0013`, `0016`, `0022`;
  `item_transfer.rs`, `item_mint.rs`, `reward_claim_mint.rs`; GAME-CHAR-01 Stage B; B3; DUR-03;
  the composition decision; the content record.
- `DERIVED`: A13; Tibia's starting backpack (`OTS_HYPOTHESIS_ONLY`, verified by STARTER-CONTENT-1).

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: docs only. STARTER-1 needs persistence and security review.

## Acceptance criteria

- [ ] Decision on an exact frozen head with passing validators.
- [ ] Independent exact-head review (persistence, security).
- [ ] Protected Merge Queue integration.

## Excluded scope

- Code, migrations, content; the rest of the starting kit; vocation items; Dawnport.

## Validation

- `python3 tools/agents/validate_governance.py`: PASS on the final authoring tree.
- `python3 tools/repository/validate_repository_policy.py`: PASS on the final authoring tree.
- `git diff --check`: clean.

## Closeout

- PR: the one named in the #162 FREEZE_SHA entry. Merge commit/result: its squash merge.
- Self-review (`oteryn-hard-worker`, read-only): 6 material and 6 minor findings, all fixed before
  the first push: the `0011`-`0013` guards need slot and mint branches and a grant consistency
  trigger; the grant row as receipt with audit columns and a `starter_grant` audit operation; the
  template from the root's `starter_template_revision` (Stage B §6); a server-originated fence
  variant with no CommandRef; attempt order, in-session retry and the TRANSFER race; the grant-key
  re-read and 23505 as replay; wording on MINT causes, A13, item loss today, the owner question
  (dropped: Stage B covers it), rows and the §39.1 parenthetical.
- Review of `ea8f4455` (#1387 5915953626: 2 MEDIUM, 1 LOW), answered in one push: STARTER-1 owns
  the chest hand-off (§4.1: the chest's runtime content carries i2854 in place of C2's injected
  backpack, with an end-to-end test); a Stage B §6.2 clarification (§4.2: an empty bound revision
  gets its first records, then records are fixed; later kits go into a new revision); resume
  (§5.4); deletion and erasure (§5.6).
- This record was archived in the PR's final authoring commit (`docs/agents/tasks/archive/README.md`).

```yaml
last_progress: final authoring commit; archived before freeze
status: completed
branch: claude/arch-starter-backpack-0
owner_action_required: null
blocker: null
next_action: null
```
