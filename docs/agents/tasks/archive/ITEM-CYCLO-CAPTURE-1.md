# ITEM-CYCLO-CAPTURE-1

```yaml
task_id: ITEM-CYCLO-CAPTURE-1
mode: MIGRATE
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/compassionate-newton-oljapn
pr: 3192
owner: owner-authorized item collection session
created_at: 2026-11-14
updated_at: 2026-11-14
owned_paths:
  - docs/reference/content-prep/item-cyclopedia-capture-1/**
  - docs/agents/tasks/active/ITEM-CYCLO-CAPTURE-1.md
  - docs/agents/tasks/archive/ITEM-CYCLO-CAPTURE-1.md
```

## Outcome

Owner request (2026-11-14): publish the verified item data read from the Tibia client, marked
for Tibia Global compatibility. Reference evidence only, under
`docs/reference/content-prep/item-cyclopedia-capture-1/`. `content/` and `rulesets/` are unchanged.

## High-risk authority/recovery qualification

NOT_APPLICABLE: data and documentation only; no protocol, identity, persistence, authority or
production change.

## Acceptance criteria

- One record per item id with Basic, dialog values and a `tibia-global` compatibility marker.
- Each record carries its TibiaWiki per-field cross-check and verdict.
- `manifest.json` SHA-256 matches `items.jsonl`.
- The final push carries the complete native re-collection.

## Closeout

- Final push carries the complete native re-collection: 6369 records, `manifest.json` SHA-256
  matches `items.jsonl`.
- Review: none requested; reference data only.
- merge commit/result: squash merge of #3192 if the owner merges it.

## Validation

- `git diff --check`: pass
- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: OK
