# Spell and DEATH-0 control-plane handoff — 2026-09-29

```yaml
status: RETAINED_EVIDENCE
repository: Oteryn/Oteryn-Game
coordination_issue: 162
protected_main_at_handoff: 3d595bc818112779b5ffebba8b18bc425344ac50
implementation_authority: NONE
merge_authority: REPOSITORY_CONTROL_PLANE_ONLY
jira: "pending: no mapped spell Story (KAN-16 is the content census)"
```

## Purpose

This handoff lets the next agent continue the spell-cast and DEATH-0 lanes without re-reading the
session. Live GitHub state (#162, the PRs below, their checks) is authoritative. Where this file and
GitHub disagree, trust GitHub.

## 1. Delivered

| Task | PR | Merge on `main` | What it gives |
|---|---|---|---|
| `OTV2-20260929-spell-harmony-runtime` (SPELL-D8 H-2) | #1243 | `b18ccc4f` | Monk Harmony (0..5) and Serene engine rules in `apps/game-server/src/spell/harmony.rs`. Harmony spells (`harmony_role`, `monk_focus`) stay fail-closed in the spell reader, pinned by a test. |
| `OTV2-20260929-spell-cast-wire-w1` (§9 step 1, SPELL-D8 H-3) | #1254 | `c91055f6` | `docs/contracts/protocol-oteryn/v1/actor_spell_v1.proto`, command type 3 `WORLD_ACTOR_SPELL_CAST_INTENT`, state domain 3 `ACTOR_VITALS` (delta 1, snapshot 1), `SPELL-RL-01..03`, strict codecs in `crates/protocol-oteryn/src/actor_spell.rs`. |

Owner decision recorded during the session: SPELL-D8 Q1 = b. The remaining forced-Serene time is
durable (as in Canary); the `serene` flag itself is re-evaluated at every initialization. The
contract text was reconciled in #1243.

## 2. In flight at handoff

### #1264 — DEATH-0 (`OTV2-20260929-death0-character-death-receipts`)

- Branch `claude/death0-character-death-receipts`, frozen head `4e11f494fa6e6a9fac8dade22d984426075ddec1`.
- Migration `0016_character_death_receipts.sql`: death receipts, blessings, pending respawns, and a
  rewrite of the 0009 state and consistency guards (two receipt kinds, one chain). No writer.
- Independent review (separate non-authoring agent): KEEP, 0 material findings.
- Required CI green; auto-merge enabled; entered the Merge Queue at 19:11 UTC.
- **Next:** verify the merge on `main` and post the `integrated` record on #162.
- Control-plane rulings (#162):
  - one `#[path]` include was added to `apps/game-server/tests/character_authority_postgres.rs`,
    the D3-6 pattern;
  - the extra guard stays: every state update must match its receipt's `before` values;
  - DEATH-1 carry-over, recorded in the task record: `verify_character_integrity` must count death
    receipts before any death is written; the writer inserts the receipt before deleting blessings;
    the blessing set is capped at 32.

### #1263 — spell cast composition W2a (`OTV2-20260929-spell-cast-composition-w2a`)

- Branch `claude/spell-cast-composition-w2a`. Head `8c52192a` was frozen, then released.
- Wires §9 step 2 on the server:
  - command type 3 → `resolve_cast` → `effect_plan`, then one compare-commit that pays mana and
    soul, heals and sets cooldowns;
  - the result disposition and the `ACTOR_VITALS` snapshot and delta;
  - the spell book loads at boot.
- **Production cast gate stays closed.** Every live cast returns `REJECTED`, because no Character
  vocation or magic level exists yet (SPELL-D4).
- **Blocking CI failure on `8c52192a`**, the PR's own:
  - `server_seam_real_owners_over_tcp_tls` (job 109560699570) and
    `node_boot_seam_against_running_node` (job 109560699769) both fail with
    `same-session resume diverged` at `SEAM_EVIDENCE stage=resume`;
  - the likely cause is that vitals or spell state are not replayed consistently on a same-GameSession
    resume.
- A W2a worker was fixing it at handoff (one push to the same branch, no amend or force). If
  no new head exists, re-allocate the fix within the same lease.
- **Next:** freeze the new head, run a fresh independent review focused on the resume path, then
  auto-merge on green.
- The first independent review (KEEP on `8c52192a`) is **superseded**: it missed the resume
  divergence.
- Size is about 690 non-test lines, accepted as one slice that cannot be split.

## 3. Open decisions

- **`SPELL-CASTER-FACTS`, an architecture escalation on #162.** Where do a Character's vocation and
  magic level (and ML progress) come from for `CasterState`?
  - It belongs to the architect (`Oteryn: sol supervising architect`), not the owner or the control
    plane.
  - W2b (opening the cast gate) is not allocated until it is decided.
  - A durable field probably touches the same guard functions as DEATH-0, STANCE-0 and H-1.

## 4. Blocked or queued lanes

- **H-1:** durable Harmony and forced-Serene time. It needs a Harmony receipt kind, decided after
  the DEATH-0 and STANCE-0 migrations, because each of them replaces the same guard functions and the
  later migration carries every kind.
- **STANCE-0:** its decision (D145, #1225) is merged; there is no allocation yet.
- **DEATH-1:** the writer and reconcile. Its dependency DEATH-0 is in the Merge Queue.
- **§9 step 3** (native client spell bar, result and vitals bar) and **step 4** (qualification in the
  native entry room). Step 3 does not depend on `SPELL-CASTER-FACTS`.
- **ITEM-ID-1:** it holds a lease on `apps/game-server/**` and was blocked on the pinned 15.30
  appearances file. `main` now has `d9ecaaf9` ("B3 census reads the in-repo 15.30 appearances"),
  so the blocker may be gone. The owner chose (1a) to proceed with W2a despite the overlap: whichever
  change merges second rebases.

## 5. Working notes

- **Codex review quota was exhausted** on #1263 (comment 5896345388). Independent review then used a
  separate non-authoring read-only agent, as `DELIVERY_COMPLETENESS_AND_CLOSEOUT.md` allows.
- **Local PostgreSQL:** the CI-pinned 17.6 image is available from `mirror.gcr.io` when Docker Hub
  returns 429.
- **Disk:** the container disk filled once. Finished worktrees under `.claude/worktrees/`, and the
  scratchpad worktrees `harmony` and `h1243`, can be removed. Removal needs owner permission in this
  environment.
- **Self check-in:** `trig_01QnbnujZf7axGSyCShwZe3k` was scheduled for 19:48 UTC to re-check #1263
  and #1264. It belongs to this session; delete it or let it lapse.
