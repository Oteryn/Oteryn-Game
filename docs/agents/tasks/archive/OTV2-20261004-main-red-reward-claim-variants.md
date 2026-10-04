# OTV2-20261004-main-red-reward-claim-variants

```yaml
task_id: OTV2-20261004-main-red-reward-claim-variants
title: "Main red: re-seal the RewardClaim variant migration packet after #1706"
mode: REPAIR
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/main-red-reward-claim-variants
pr: 1709
base_sha: 2a5ce70c
owner: claude-code-session-01QFRdKvFbNsNiCzMyR75FrC (control-plane P0 allocation)
created_at: 2026-10-04
updated_at: 2026-10-04
owned_paths:
  - tools/content-schema/reward-claim-authoring/samples/migration/reward-claim-variants.json
  - docs/agents/tasks/archive/OTV2-20261004-main-red-reward-claim-variants.md
public_contracts: []
```

## Outcome

#1706 (ITEM-SEM-2b-2) regenerated Item definition shards under `content/items/definitions/`. The
RewardClaim variant migration packet seals the digest of each authoring source it reads, so
`reward_claim_variant_migration.py --check` failed on `main` with "variant migration packet is
stale". The packet is regenerated with `reward_claim_variant_migration.py`. Only
`authoring_sources[*].sha256` values change (61 of 84 entries); the variant set (105 variants, eligible
plain missing 0) is unchanged. No other file pins the packet's digest.

## Validation

- `reward_claim_variant_migration.py --check`: pass after regeneration.
- `reward-claim-authoring` workflow steps run locally: variant tests, typed source variant packet,
  RewardClaim authoring tests and deterministic content: pass. (`Verify exact head` needs the CI
  event context.)
- `git diff --check`: clean.
- `python tools/agents/validate_governance.py`: pass.
- `python -m unittest discover -s tools/agents/tests`: pass.
