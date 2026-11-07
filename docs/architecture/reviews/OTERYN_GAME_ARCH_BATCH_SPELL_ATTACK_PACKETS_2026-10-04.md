# Architect batch: spell book activation, damage spells and the rune order

- Batch: `ARCH-SPELL-ATTACK-PACKETS-1` (owner D577 answer `b`; control plane priority list)
- Status: **ACCEPTED WHEN THIS DECISION MERGES** for the rulings, leases and packets below.
  ATTACK-0 and RUNE-USE-0 stay what they are; this batch implements accepted ATTACK-0 semantics
  only and does not accept RUNE-USE-0.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers:
  1. D577 `b`: the full imported spell book on the default node now (SPELL-BOOK-ACTIVATE-1).
  2. Damage spells: what already works and what SPELL-TARGET-1 adds.
  3. Where RUNE-USE-0 sits in the order.
- Runtime, migration, deployment and production authority: NONE. Each packet needs its #162
  allocation. Setting the node's configuration and issuing content activation are owner actions (§1.3).
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## 0. Leases and order

### 0.1 Shared files

| File | Owner | Others |
|---|---|---|
| `apps/game-server/src/spell/cast.rs` | SPELL-LOCK-1 (in flight); SPELL-STARTER-1 owns `V1_BUNDLES` | SPELL-BOOK-ACTIVATE-1: none. ATTACK-1b: the `ATTACK_TARGET` seam only. SPELL-TARGET-1: the `SpellTarget::AttackTarget` arm and the `needs_target` refusal, after SPELL-LOCK-1 and ATTACK-1b merge |
| `apps/game-server/src/gameplay_transport/ordinary_combat.rs` | SPELL-LOCK-1 (in flight) | SPELL-TARGET-1: the `SpellTarget::AttackTarget` arm (today `NotAvailable`), after SPELL-LOCK-1 merges |
| `apps/game-server/src/gameplay_transport/qualification.rs` | SPELL-BOOK-ACTIVATE-1: one `#[cfg(test)]` module declaration | none |
| `apps/game-server/src/gameplay_transport/mod.rs` | ATTACK-1b | SPELL-BOOK-ACTIVATE-1 does not touch it |
| `apps/game-server/src/node/serve.rs`, `src/bin/oteryn-game-ops.rs` | none | none of these packets touch them (§1.1) |

No packet here needs a migration, a protocol registry row or a resource row.

### 0.2 Order

1. **SPELL-BOOK-ACTIVATE-1** now, in parallel with SPELL-LOCK-1 and SPELL-STARTER-1. Their paths do
   not overlap.
2. **Owner-side activation** (§1.3) any time after SPELL-BOOK-ACTIVATE-1 merges. It needs no
   other packet.
3. **SPELL-TARGET-1** after ATTACK-1b and SPELL-LOCK-1 merge. ATTACK-1b still waits for VIS-3
   (core-loop batch §0.2).
4. **RUNE-USE-0 acceptance**, after ITEM-USE-1 is packeted (§1.5), then its children in their own
   order.

## 1. Rulings

### 1.1 The book is selected by configuration, not by a code default

D577 asks for the 246-spell book on the default node. The selection code already exists and
works:

- `node/serve.rs` reads `OTERYN_NATIVE_GAMEPLAY_MANIFEST`, decodes it with
  `NativeGameplayInput::from_manifest` and calls `activate_native_entry_room_with_gameplay`.
- The spell book is then `native.spell_book()`.
- `oteryn-game-ops --config <ops-config> content activate` reads the same variable. It computes
  the server, client and frame-binding digests of the room *with* the gameplay artifact.
- `activate_qualified_native_entry_room` refuses boot (`DigestMismatch`,
  `FrameBindingMismatch`) unless the issued digests match that room.

The default node gets the book through its configuration plus a matching issuance. No code
default is added:

- **No implicit path fallback.** The node binary has no reliable repository path. A
  working-directory-relative default would make readiness depend on the directory the node runs
  in. It would also turn an absent file into either a silent baseline boot or a refused boot,
  which breaks the rule in `tools/content-schema/native-gameplay/README.md`.
- **No embedded catalogue.** The same README forbids taking the catalogue from a global
  `include_bytes!`. It is also 88 MiB of artifact bound into every build.
- **The variable stays the single selector, for both node and ops.** The issuance is always
  computed over the room the node will actually boot.

The canonical manifest is `content/spells.manifest.json` (schema v5, `accepted-entry-r1`).
Its README pins the eleven providers, and `content/abilities/SPELL-IMPORT.md` describes the import.
`content/test-packs/spells/r25/manifest.json` stays a frozen input snapshot. It is not the activation default.

### 1.2 Non-castable shapes refuse with a typed disposition

The book is safe to switch on because every cast already ends in one of the ten
`SpellCastDisposition` values. A shape with no owner refuses; nothing is accepted without effect.

- `cast_spell_completion` dispatches familiar, then world items, then native combat, then the
  V1 path.
- Ordinary combat (`Effects`, `AbilityVariants`, `PartyBuff`) refuses `SpellTarget::AttackTarget`
  with `NotAvailable`.
- It refuses a missing facing with `TargetRequired` and a cross-floor origin with `TargetIllegal`.
- Unresolvable abilities, elements or geometry refuse with `Rejected`.
- Native keys without a child refuse (SPELL-NPC-MAP-0: only `party_buff` is implemented).

SPELL-BOOK-ACTIVATE-1 proves this for the whole book. It changes no disposition. It fixes a shape
that panics, or that returns `Cast` while writing no effect, by making it refuse with the
disposition the existing code uses for its family. A finding that needs a new owner is a
`BLOCKER` to the control plane; it is not built in that packet.

### 1.3 Owner-side steps (not repository work)

These need production or test-server authority. The repository grants neither.

1. **Deploy the manifest tree with its layout.** The manifest's paths are relative to its own
   directory, and two of them leave `content/`:
   - `source_world.path` is `../imports/spells/r25/source-world.json`;
   - `wheel_profile.path` is `../rulesets/progression/wheel-of-destiny/spell-profile.json`.

   So deploy, from the merged commit, `content/`, `imports/spells/r25/source-world.json` and
   `rulesets/progression/wheel-of-destiny/spell-profile.json` under one root `<R>`, with their
   repository-relative paths kept. Set `OTERYN_NATIVE_GAMEPLAY_MANIFEST=<R>/content/spells.manifest.json`
   for the node service **and** for the `oteryn-game-ops` run of step 2. A missing or unreadable
   referenced file refuses the decode (`BootError::ContentActivation`), so the node does not boot.
2. **Issue the activation** with the real command, under that variable:

   ```text
   oteryn-game-ops --config <ops-config> content activate --world <W> --channel <C> \
     --sequence <N+1> --previous <N> --request content-activation-<W>-<C>-<N+1>.json
   ```

   - `--config` is required. It names the root-owned operator configuration, which holds the
     operator state directory and the control database connection. The command runs as root and
     refuses the service user.
   - `<N>` is the scope's current activation sequence (`empty` if none).
   - The request file is created in the operator state directory with the computed server, client
     and frame-binding digests of the room *with* the gameplay artifact, and is then recorded.
   - The operator needs the Content-activation grant for the scope. If the outcome is unknown,
     re-run with the same `--request` file: it replays exactly.
3. **Restart the node** with the variable set. If the digests or the frame binding do not match
   the newest activation, the node refuses readiness (`DigestMismatch`, `FrameBindingMismatch`).
   This batch specifies no rollout mechanism and promises no availability during the switch: a
   refused node stays down until its configuration and the activation match.
   - To go back, unset the variable and issue the baseline digests at the next sequence with a new
     request file (`--sequence <N+2> --previous <N+1>`), then restart.
   - Run all three steps on a test node first.

The content is `baseline_test` magnitude with source approximation flags. It is test and
preproduction content, not production numerical parity (`content/test-packs/spells/r25/README.md`).

### 1.4 Damage spells: area works now, targeted needs SPELL-TARGET-1

With the book active, ordinary combat already resolves damage that has no attack target:

- self-centred areas;
- facing-directed waves and beams (`SpellTarget::None` with `needs_direction` or
  `target_or_direction`);
- position origins.

Each of these goes through the chain world (sight, path and hit checks) and the `OwnerCombatBatch`. Single-target spells and runes that aim
at the attack target refuse `NotAvailable` until the server holds an attack target.

- **SPELL-TARGET-1** (ATTACK-0 child) replaces exactly those two `AttackTarget` arms (ordinary combat
  and `spell/cast.rs`) with the ATTACK-1b seam. It resolves the §4 target, applies the same
  sight and range checks, and refuses `TargetRequired` with no target and `TargetIllegal` when the target is out of sight, out of range or on another floor.
- A separate area packet is not needed. If the SPELL-BOOK-ACTIVATE-1 sweep (§2.1) shows an
  area family that refuses for a reason other than an attack target, that is a finding for the
  control plane, not a reason for a speculative area child.

### 1.5 RUNE-USE-0 stays a candidate here

Runes need USE on an item (cast-wire contract §6), not attack spells. RUNE-USE-0 is accepted in
its own architect batch, once ITEM-USE-1 is packeted, because RUNE-1 depends on ITEM-USE-1. Its
children keep RUNE-USE-0's order:

1. RUNE-WIRE-1. It can start after acceptance, since ITEM-USE-WIRE-1 has merged.
2. RUNE-CONTENT-1, after ITEM-SEM-USE.
3. RUNE-1, after ITEM-USE-1 and RUNE-WIRE-1.
4. RUNE-CAST-1, after RUNE-1, SPELL-TARGET-1, VIS-2 and CHAR-BUILD-1.

Nothing in this batch waits for runes.

## 2. Packets

### 2.1 SPELL-BOOK-ACTIVATE-1

```yaml
task_id: OTV2-20261004-spell-book-activate-1
decision: this batch §1.1-§1.3; SPELL-NPC-MAP-0; cast-wire contract §6
worker: oteryn-impl-worker
review: spell review (Codex, final frozen head)
branch: claude/spell-book-activate-1-20261004
base: main
owned_paths:
  - apps/game-server/src/gameplay_transport/spell_book_sweep_tests.rs   # new
  - apps/game-server/src/gameplay_transport/qualification.rs           # one #[cfg(test)] module line
  - apps/game-server/src/gameplay_transport/native_combat_cast.rs      # only a §1.2 refusal fix the sweep finds
  - apps/game-server/src/gameplay_transport/stance_cast.rs             # only a §1.2 refusal fix the sweep finds
  - apps/game-server/src/gameplay_transport/familiar_cast_dispatch.rs  # only a §1.2 refusal fix the sweep finds
  - apps/game-server/src/gameplay_transport/native_companion_item_cast.rs  # only a §1.2 refusal fix the sweep finds
  - apps/game-server/src/gameplay_transport/native_world_item_cast.rs # only a §1.2 refusal fix the sweep finds
  - tools/qualification/node_boot/run.sh                               # default spell manifest and repository-root staging
  - tools/qualification/spells/README.md
  - tools/content-schema/native-gameplay/README.md
  - content/abilities/SPELL-IMPORT.md
  - docs/agents/tasks/archive/OTV2-20261004-spell-book-activate-1.md
validation:
  - cargo fmt --all -- --check
  - cargo clippy -p oteryn-game-server --all-targets -- -D warnings
  - bash tools/qualification/spells/run.sh runtime gameplay_transport::spell_book_sweep_tests
  - bash tools/qualification/spells/run.sh runtime
  - bash tools/qualification/spells/run.sh map content/spells.manifest.json
  - python tools/agents/validate_governance.py
  - git diff --check
```

Builds:

- **The whole-book sweep.** A test reuses the qualification fixture that activates
  `content/spells.manifest.json` through `NativeGameplayInput::from_manifest` (as
  `qualification.rs` does). For every one of the 246 book indices it runs
  `cast_spell_completion` for an admitted caster with enough level, magic level, mana and soul. It
  runs once per target form the protocol admits: `None`, `AttackTarget`, and a `Position` on the
  caster's floor.

  It asserts:
  - no panic and one of the ten dispositions;
  - `AttackTarget` on a spell that needs a target gives `NotAvailable`;
  - every `Cast` wrote at least one owner effect or vitals change;
  - an exact golden count of dispositions per family. A later change that moves a spell between
    `Cast` and a refusal then shows up in review.

  Spells marked `needs_target` are swept with an empty target and must give `TargetRequired` or
  `NotAvailable`.
- **Damage evidence.** One self-centred area damage spell and one directional wave, each cast
  next to a fixture creature, reduce that creature's health through the existing
  ordinary-combat path. If the creature does not take damage, report `BLOCKER`; do not build a
  damage owner.
- **Refusal fixes**, only for what the sweep finds (§1.2), within the dispatch files listed above.
  `spell/cast.rs` and `ordinary_combat.rs` are SPELL-LOCK-1's. A finding there is reported to
  the control plane for SPELL-LOCK-1 or a later batch.
- **Qualification default and staging.** `tools/qualification/node_boot/run.sh` defaults
  `NODE_BOOT_SPELL_MANIFEST` to `content/spells.manifest.json` instead of the r21 reference
  artifact. The baseline boot (`NODE_BOOT_SPELLS=0`) stays available.
  - Today the stager (`run.sh`, the `PY_STAGE` block) takes the manifest's directory as its
    source root and rejects any `..` locator. The canonical manifest's `source_world` and
    `wheel_profile` locators leave `content/`, so it must change.
  - The staging root becomes the repository root. Each locator is resolved against the
    manifest's directory, must stay inside the repository root, and must match its pinned
    `sha256`. Absolute and backslash locators stay refused.
  - Only the manifest and its declared, hash-bound inputs are staged, under their
    repository-relative paths (`content/spells.manifest.json`, `content/...`, `imports/...`,
    `rulesets/...`). No unrelated sibling is published.
  - `OTERYN_NATIVE_GAMEPLAY_MANIFEST` and `NODE_BOOT_SPELL_MANIFEST` point at
    `$BASE/gameplay/<manifest path relative to the repository root>` instead of its basename.
  - The r21 reference artifact still stages and boots under an explicit `NODE_BOOT_SPELL_MANIFEST`.
- **Docs.** The two READMEs and `SPELL-IMPORT.md` describe the §1.3 activation and state that the
  variable is the single selector.

Acceptance:

- the sweep passes with its goldens, and the counts are recorded in the task record;
- the damage evidence passes;
- the node-boot qualification boots the native book with an issuance computed by
  `oteryn-game-ops` under the same variable;
- no change to `node/serve.rs`, `oteryn-game-ops`, the manifest bytes, the protocol or any
  disposition other than a §1.2 fix.

Not in scope:

- the attack target (SPELL-TARGET-1);
- runes (RUNE-USE-0);
- conjuring (DUR-03 MINT);
- new native keys (SPELL-NPC-MAP-0 children);
- `V1_BUNDLES` (SPELL-STARTER-1);
- numerical parity;
- deployment and issuance (§1.3);
- removing the `native_gameplay.rs` module `dead_code` allowance.

### 2.2 SPELL-TARGET-1

```yaml
task_id: OTV2-20261004-spell-target-1
decision: ATTACK-0 child table, §4; this batch §1.4
worker: oteryn-impl-worker
review: combat review (Codex, final frozen head)
branch: claude/spell-target-1-20261004
base: main after ATTACK-1b and SPELL-LOCK-1 merge
owned_paths:
  - apps/game-server/src/spell/cast.rs                         # the AttackTarget arm and the needs_target refusal only
  - apps/game-server/src/spell/cast_tests.rs
  - apps/game-server/src/gameplay_transport/ordinary_combat.rs # the AttackTarget arm only
  - apps/game-server/src/gameplay_transport/spell_book_sweep_tests.rs  # goldens move
  - docs/agents/tasks/archive/OTV2-20261004-spell-target-1.md
validation:
  - cargo fmt --all -- --check
  - cargo clippy -p oteryn-game-server --all-targets -- -D warnings
  - bash tools/qualification/spells/run.sh runtime
  - python tools/agents/validate_governance.py
  - git diff --check
```

Builds:

- `SpellTarget::AttackTarget` resolves through the ATTACK-1b seam to the caster's current §4 attack
  target. With no target the cast gives `TargetRequired`. A target that is out of sight, out of
  range, on another floor or not attackable gives `TargetIllegal`.
- The resolved target becomes the origin and the single target of ordinary combat. Damage, death
  and kill credit go through the existing batch and the DEATH-2 path; nothing new is written.
- The sweep goldens move from `NotAvailable` to `Cast` for single-target damage.

Not in scope: changing the attack target from a spell, PvP rules beyond ATTACK-0, runes (RUNE-CAST-1).

## 3. Rejected options

- **A code default to a repository path or an embedded catalogue** (§1.1).
- **A separate area-damage child now.** Area damage already runs through ordinary combat; one is
  packeted only if the sweep finds a gap.
- **Switching the book on in the same PR as SPELL-TARGET-1.** That would hold 200+ working spells
  behind VIS-3 and ATTACK-1b.
- **Accepting RUNE-USE-0 here.** RUNE-1 depends on ITEM-USE-1, which has no packet yet.

## 4. Decision test

- The default test node, configured per §1.3, boots `native.spell_book()`.
- Every index of the 246-spell book gives a typed disposition, with no panic and no effectless `Cast`.
- Area and directional damage spells damage a creature now.
- Single-target damage waits only on SPELL-TARGET-1, which waits only on ATTACK-1b and SPELL-LOCK-1.
- No packet here touches SPELL-STARTER-1's `V1_BUNDLES`, ATTACK-1b's paths or SPELL-LOCK-1's paths
  before those merge.
