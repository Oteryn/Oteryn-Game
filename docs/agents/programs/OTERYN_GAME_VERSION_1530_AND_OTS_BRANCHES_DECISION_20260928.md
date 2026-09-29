# Oteryn game version 15.30 and OTS source branches

- Date: 2026-09-28
- Status: ACCEPTED (owner decision)
- Programme: recorded from #162; Jira `KAN-16`
- Supersedes: §2 "Server-side Item data stays pinned at 15.25" of
  `docs/architecture/OTERYN_CLIENT_ASSET_VERSION_OWNER_DECISION_2026-09-27.md`
- Amends: the `CANARY_OTS` / `CRYSTAL_OTS` section of `OTERYN_REFERENCE_INVESTIGATION_SOURCE_REGISTRY_20260910.md`
- Related: `OTERYN_TARGET_DATE_20260927_DECISION.md` (target 2026-09-27)

## 1. Game version

The owner set the game version to the newest Tibia client, **15.30**. Before this decision, the client graphics
were pinned to 15.30 and the server-side data to the 15.25 engines.

- **Scope.** 15.30 applies to the whole game: client graphics (already pinned by the 15.30 asset manifest),
  server-side content data, mechanics and protocol behaviour. This matches the target date 2026-09-27, which is
  after the 15.30 Summer Update.
- **Server-side data is 15.30.** Items, creatures, loot, NPCs, spells, quests and every other server-side family
  target 15.30. Each family re-pins its Canary or Crystal source to a 15.30-capable revision and does not wait for
  upstream `main` to move past `CLIENT_VERSION` 1525. The revision may be on a branch (§2).
- **Existing 15.25 data is transitional.** Data already imported from the 15.25 pins keeps its recorded provenance,
  but it is not the target. Every family must be re-pinned to 15.30 and reconciled; this is required work, not an
  option. In the re-pin, a value that differs between 15.25 and 15.30 is a `CONFLICT`, and target-date evidence
  resolves it (tibia.com, then the wikis). The 15.25 value never wins silently. New content, including the 1,409
  object ids new in 15.30, uses 15.30 sources from the start.
- **Scheduling.** The work coordinator (#162) allocates one bounded re-pin task per family. Active batches may finish
  on their current pin. The next batch of each family starts from a 15.30 revision.
- **Precedent.** The spell source already follows this rule: S14 uses the Canary 15.30 branch
  `dudantas/fix-tibia-15-30-regressions` at `99902524` (`OTERYN_SPELL_AUTHORING_SCHEMA_V1.md`).

## 2. OTS source branches

Agents may use any branch of the two OTS repositories, not only `main`:

- https://github.com/opentibiabr/canary/branches/active
- https://github.com/zimbadev/crystalserver/branches/active

Rules:

- **Same evidence class.** A branch is `OTS_HYPOTHESIS_ONLY`, like `main`. It never outranks tibia.com or the wikis,
  and it never silently fills `UNKNOWN` or resolves `CONFLICT`.
- **Pin the commit.** Record the repository, the branch name, the exact commit SHA and the read date. Branches move,
  are force-pushed and get deleted, so the commit SHA is the pin and the branch name is only context. Reproduction
  must work from the commit alone.
- **Prefer `main` when it is equal.** Use a branch when it carries something `main` lacks, for example 15.30 support
  or a fix. Say why the branch was chosen.
- **One source revision per batch and repository.** Do not mix commits from several branches of the same repository
  in one batch without recording each value's commit.
- **Reference only.** Engine code on a branch is evidence to read, as on `main`. It is not copied into Oteryn (see
  the playable-first and upstream-first policy).

## Non-claims

- No runtime, protocol, persistence or content change is made by this record.
- No existing batch, binding or identity is re-pinned by this record itself. The required re-pins run as separate tasks per family.
- Branch availability grants no authority over the upstream repositories.
