# MAGIC-ITEMS-1

```yaml
task_id: MAGIC-ITEMS-1
title: "MAGIC-ITEMS-1: rune use and conjure; first slice RUNE-CONTENT-1 rune carrier audit"
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
issue: 1622
base_branch: main
branch: claude/magic-items-1-20261014
pr: 2116
base_sha: 75ec7985
head_sha: null
final_head_sha: null
owner: claude-code-session-01Puou2Pm8SECtLFcjHYAuEC
control_plane: session_01XYC239givrMonrYoBwwECV
created_at: 2026-10-14
updated_at: 2026-10-14
owned_paths:
  - tools/content-schema/spell-authoring/audit_rune_carriers.py
  - tools/content-schema/spell-authoring/test_audit_rune_carriers.py
  - tools/content-schema/spell-authoring/samples/rune-carrier-audit.json
  - tools/content-schema/spell-authoring/README.md
  - docs/agents/tasks/active/MAGIC-ITEMS-1.md
public_contracts: []
external_repositories: []
```

## Outcome

Plan row MAGIC-ITEMS-1 (`docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md`,
appendix "MAGIC-ITEMS-1"), under RUNE-USE-0 (accepted, #2107). This PR is the one child whose
prerequisites are all on `main`: **RUNE-CONTENT-1** (depends on ITEM-SEM-USE, merged).

- `audit_rune_carriers.py` audits the 36 rune carriers of `content/abilities/definitions/player-spells.json`
  and writes `samples/rune-carrier-audit.json`; `--check` fails when the committed audit is stale. Checks
  per carrier: rune item ID (wikis, Canary and Crystal registrars), vocations (wikis, S3; the agreeing
  registrars where no wiki states them), cooldown group, far use and every targeting flag the converter
  maps (`aggressive`, `self_target`, `needs_target`, `needs_direction`, `target_or_direction`, `block_walls`,
  `allow_on_self`, `check_floor`; registrars), the PZ-lock and weapon flags (`pz_locks_caster`,
  `needs_weapon`; registrars), range (wikis and registrars), and the target restriction
  `allowed_targets` against the accepted S27 onCastSpell guard (`guard-behaviours.json`), and the
  `parameter` mode, `cast_at_position` (`optionalTarget`) and `aim_at_target` as the converter derives
  them; any other targeting field fails as unaudited (`targeting_fields`). Where the references
  conflict, our value must be that of the source the catalogue manifest records as the winner
  (S3/S11/S13/S14); a recorded native resolution (S27, Disintegrate non-aggressive) requires a value
  that differs from the references;
  the dated `rune-use-source-resolutions.json` facts (they replace the field they name), `rune.charges` =
  the conjuring spell's `count` where both exist (§3), the native Item profile stack facts (stackable,
  100, `StackCapable`), and the authored area against `RUNEUSE0-RL-08` (≤ 37 tiles, so the victim cap
  is ≤ 74; only hit cells `x`/`C` count, as in the runtime geometry) and `RUNEUSE0-RL-07` (≤ 12 tiles for a rune that creates fields). The joins and the 5 aliases
  are those of `verify_spells.py`.
- Result: 34 of 36 pass. The largest area is 37 tiles and the largest field area 12 (Energy Wall
  diagonal), which reproduces the decision's ceilings. All 33 joinable charges equal the conjure count;
  Lightest Missile, Light Stone Shower and Lightest Magic Missile have no conjuring spell (S25).
- **Fail, stay blocked (2):** Intense Healing Rune (the lowered catalogue lacks Monk and Exalted Monk,
  per the 2026-10-09 ledger) and Paralyze Rune (catalogue group `support`, ledger `attack`). The
  converter already applies the ledger; the lowered catalogue and the r21 candidate bundles predate it.
  A catalogue regeneration (pinned Canary/Crystal checkouts) closes both.
- Stack facts: the live conjure path (`world_item_cast.rs`) qualifies items from the native Item
  profiles, where all 36 runes and the blank rune are already admitted stack-capable, 100. No content
  regeneration, materializer step, migration or registry is needed for this slice.
- A pass is not runtime admission: each carrier stays subject to its §9 group's needs.

## Blocked children (not implemented)

- RUNE-WIRE-1: needs a `RUNE_USE_V1` capability number from the registry (#2113/#2114).
- RUNE-1: needs a migration (0083 `cause_kind CHECK IN (1,2)`); #2110 holds 0085.
- RUNE-CAST-1: RUNE-1, SPELL-TARGET-1, VIS-2.
- RUNE-CONJ-1 (and gate `RUNEUSE0-C1`; conjure is ungated today): RUNE-1, SPELL-D4.
- FIELD-1, FIELD-WIRE-1, Soulfire: COND-1 / CONDITIONS-0 (#2106).

## Review

- Codex P2 on 7401dd64 (conflicts passed on any matching reference): repaired; a conflict now passes only
  on the manifest-recorded winning source.
- Codex P2 on 7401dd64 (`aggressive` and `self_target` unchecked): repaired; both are checked against the
  registrars with the converter defaults.
- Codex P2 on 5e0f8fdf (only the last conjure count per item audited): repaired; every conjuring spell
  of the item is kept and each count must equal the charges (item 3174 has three, all 10).
- Codex P2 on 5e0f8fdf (an incomplete carrier inventory passes): repaired; the audited names must equal
  the 36 names of the section 9 groups, else the tool exits 1 without writing.
- Codex P2 on ee280f07 (one registrar record suffices): repaired; a `registrars` check fails a carrier
  unless both the Canary and the Crystal records exist (all 36 have both).
- Codex P2 on e38bbac5 (registrar ranges ignored): repaired; the range is compared with the wikis and the
  registrar `range` (Desintegrate 1 and Destroy Field 5 now agree instead of `no_source`).
- Codex P2 on e38bbac5 (a native override reverted to the reference value passes): repaired; a recorded
  native resolution now requires a value that differs from the references.
- Codex P2 on b83699ad (remaining converter targeting fields unaudited): repaired; `needs_direction`,
  `target_or_direction`, `block_walls`, `allow_on_self` and `check_floor` are checked against the registrars
  with the converter defaults (all 36 agree).
- Codex P2 on b83699ad (duplicate carriers pass the inventory): repaired; the inventory reports duplicate
  names and the tool exits 1 without writing when any exist.
- Codex P2 on d861cb74 (`allowed_targets` unaudited): repaired; a `target_guard` check requires the S27
  guard's restriction (Intense and Ultimate Healing `self_or_own_summons`) and none on an unguarded rune.
- Codex P2 on d861cb74 (several recorded agreeing sources let a new divergence pass): repaired; several
  recorded sources are agreement provenance, so a new divergence follows the converter's S4 rule (Canary
  decides between the registrars) and any other such conflict fails.
- Codex P2 on 98782c90 (`parameter` unaudited): repaired; a `parameter` check requires the converter's
  mode from the registrar flags (`player_name`, `text` or `none`; all 36 carriers `none`).
- Codex P2 on 98782c90 (non-hit centre `c` counted as an area tile): repaired; only `x` and `C` count.
- Codex P2 on 87cc5d66 (`cast_at_position` unaudited): repaired at the root; `cast_at_position` and
  `aim_at_target` are checked, and a closed `targeting_fields` check fails any targeting field the audit
  does not cover, so a new converter field cannot pass unaudited.
- Codex P2 on 34edef0b (`pz_locks_caster` unaudited): repaired; `pz_locks_caster` and `needs_weapon`,
  the converter's registrar-only spell flags, are checked against both registrars (all 36 agree). The
  other mapped spell facts are wiki-stated and already compared by `verify_spells.py`.

## Validation

- `python3 tools/content-schema/spell-authoring/audit_rune_carriers.py --check`: up to date
- `python3 -m unittest test_audit_rune_carriers` (spell-authoring): 15 tests OK
- spell-authoring `unittest discover`: 337 run; the 8 errors need the pinned engine checkouts
  (`_sources/canary`) and are identical on `main`
- `python3 verify_spells.py self-test`: ok
- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: OK
- `cargo +1.95.0 fmt --all --check`: pass; `cargo +1.95.0 clippy --locked -p oteryn-game-server --all-targets -- -D warnings`: pass
- No Rust or `content/` file changes, so the Rust test suites are not affected and were not re-run
- `git diff --check`: clean
