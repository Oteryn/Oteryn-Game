# StarterKit authoring

Builds the `StarterKit` family in `content/starter/` from `templates.json`. It also keeps the
family registered in `content/project.json`, `content/manifest.json` and
`content/content.lock.json`.

This is STARTER-CONTENT-1 (#162), under STARTER-BACKPACK-0 §4 and §4.2
(`docs/architecture/reviews/OTERYN_GAME_STARTER_BACKPACK0_STARTER_GRANT_DECISION_2026-09-30.md`)
and the architect's family ruling on #162. It is tree-first, like RewardClaim. The family is data
only: STARTER-1 binds each configured `starter_template_revision` to a template label and fails
closed on an unknown label.

| File | Purpose |
|---|---|
| `templates.json` | The authored templates: one label per template, each with its records. |
| `sealed-templates.json` | Append-only seals: one record-set digest per label that has records. |
| `starter_kit_authoring.py` | `content` writes the family and its registration; `content --check` verifies them byte for byte. `seal --label` seals a new label. Validation runs on every build. |
| `test_starter_kit_authoring.py` | No-network tests: record shape, admission, sealing and the committed content. `tools/content-migration/test_world_project_v2_to_tree.py` runs it, so the Content Tree Migration workflow does too. |

## Rules

- **Record (§4).** A key (`oteryn:starter.<name>`), an Item definition key and revision, a
  quantity and a destination, under one template label (`oteryn:starter-template.<name>`).
  A key appears once per template.
- **Admission (§4).** The Item is an A12 key that resolves in `content/items` at the named
  revision (a record naming another revision fails), is materializable
  with a known stack class, and fits the quantity. `container_slot` takes one non-stackable
  Item with a known capacity and a container-slot equipment pattern (D114), at most once per
  template. The quantity is a JSON integer, never a boolean. Anything else fails the build.
- **Immutable labels (§4.2).** Each label with records must match its seal, and a sealed label
  never loses its records. A later kit (for example the verified Dawnport kit) goes into a new
  label for new creations only; seal it with `seal --label`. Never edit or remove a seal.
- **Append-only across revisions.** Within one head, editing a record together with its seal
  passes. `content --check --seal-base <rev>` therefore also compares the ledger with the ledger
  at a base revision (default: the merge base with `origin/main`): a seal there must be kept
  unchanged. When no base resolves (an explicit one, or `origin/main` in a shallow checkout) the
  check fails. The Content Tree Migration workflow checks out full history and passes the PR's
  base SHA (D250).

```sh
cd tools/content-schema/starter-kit-authoring
python starter_kit_authoring.py content          # rebuild after templates.json or content/items changes
python starter_kit_authoring.py content --check  # base: merge base with origin/main
python starter_kit_authoring.py content --check --seal-base "$BASE_SHA"  # CI
python starter_kit_authoring.py seal --label oteryn:starter-template.<name>
python test_starter_kit_authoring.py
```

Today the family holds one label, `oteryn:starter-template.main`, with one record:
`oteryn:starter.main_backpack`, one `oteryn:item.tibia.i2854` into the container slot.
