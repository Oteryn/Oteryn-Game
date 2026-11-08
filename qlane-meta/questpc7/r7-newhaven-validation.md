# Run 7 Newhaven checkpoint — slot 5

Status: PARTIAL. Product writes stopped at the lead's capacity checkpoint. No
publication, Git mutation, generated-data edit, migration edit, or dependency
download was performed by this worker.

## Implemented source and runtime slice

- Current named gear recipes cover all five vocations. Existing Game Simple Jo
  Staff identity/appearance51119 is retained; current Wiki external item51166 is
  a separate namespace. Historical starter50166 remains untouched.
- The native-only helper enriches twelve existing gear/book/scarf identities;
  canonical source Item shards remain unchanged. Current active helper has no
  corpse admission. The lead's last generated native data predates the unapproved
  corpse proposal and must remain paired with this restored gear-only helper.
- Preparation is read-only; corpse, all loot, armed-flag consumption and exact
  death claim commit in one existing batch transaction. Guaranteed gear follows
  ordinary stamina/event/duplex transforms and has separate bounded causes and
  fixed placements. Ordinary matching-item drops remain independent.
- First transformed ordinary table is shared by an Arc/OnceLock on the pending
  death and the existing bounded retained-corpse queue lifecycle. It survives
  requeue and recapture while the exact projection remains retained. Cold old
  scope deaths are not recovered across process loss; before atomic commit the
  entitlement stays armed, and after commit corpse/gear/claim are durable.
- Shared root-applied patches include atomic hooks, early current-fence check,
  guarantee source ordinals, compact ordinary root placements, exact guarantee
  metadata validation, cached-plan reservations that charge only the positive
  shortfall, and planner-error fallthrough to the normal settlement refusal.

## Commands and observed results

Every Cargo command below used this exact prefix:

`QPC_SLOT=5 /home/mole/oteryn-cache/codex-bin/cargo`

1. `test --locked --offline -p oteryn-gameplay-dispatch --lib newhaven_corruptor -- --nocapture`
   - Earlier actual run: build2m00s; runtime129.34s;1passed,2failed.
   - Last actual run: build2m20s; runtime126.68s;1passed,2failed.
   - Both PG scenarios reached actual auto-attacks and logged
     `kill_reward_refused reason=corpse_item_inadmissible creature=oteryn:creature.the_corruptor`.
     They then failed `Corruptor dies to actual auto-attacks`; no PG completion
     marker was emitted. The pure guarantee test passed in each run.
   - A strict production-row preflight now reports the admission reason before
     combat. That final fixture change has compiled, but PG was deliberately not
     repeated while the exact known admission block and owner choice remain.
2. `test --locked --offline -p oteryn-gameplay-dispatch --lib newhaven_cached_plan -- --nocapture`
   - PASS1/0; build4m37s; runtime0.00s. Tests initial reservation reuse, positive
     shortfall only, idempotent top-up, refusal above64 without counter mutation,
     and full release on completion.
3. `test --locked --offline -p oteryn-gameplay-dispatch --lib newhaven_corruptor_guarantee -- --nocapture`
   - PASS1/0; build1m00s; runtime0.22s. Across4096death seeds, the rated2M special
     output survives the95% factor and leaves an independent ordinary drop.
4. `test --locked --offline -p oteryn-game-server --test combat_death_reward_postgres tutorial_corruptor_preparation -- --nocapture`
   - Earlier PASS1/0; build2m21s;
     `NEWHAVEN_PREPARE POSTGRES_CASE_COMPLETED 2.650965072s`.
   - This proves read-only preparation, stale-fence refusal, and no caller-selected
     vocation gear. It does not qualify the subsequently changed atomic mint path.
5. `python3 -m unittest discover -s tools/content-migration -p test_quest_finish_newhaven_items.py -v`
   - Provisional corpse48 helper: PASS8/0 in2.510s.
   - Corpse-only new hunks were then saved and withdrawn at the lead's request;
     final restored gear-only helper: PASS7/0 in1.890s.
6. `rustfmt --check --edition 2024` on all six owned Rust files: PASS exit0.
7. Lead-produced result file `.questpc/r7-native-content-check-final.result.json`
   was read: `python3 tools/content-migration/register_spell_families.py --check`,
   exit0,39.134s on the restored helper. This full producer check establishes the
   generated checkpoint. A worker's separate in-memory single-helper comparison
   was not byte-idempotent because the helper emits intermediate pretty JSON and
   intermediate51119 taxonomy; the final production pipeline normalizes those.
   It wrote no files and is not substituted for the actual producer check.

Earlier current-recipe quest unit run passed4/0. No all-five dispatch scenario is
qualified, and no full Viola completion is claimed.

## Constructor and API checks

`rg -n 'PendingKillSettlement \{' crates --glob '*.rs'` found the type/impl,
three production initializers (including the constructor's Self expression),
and the existing test initializer. All initialize the shared tutorial cache.
`rg -n 'claim_tutorial_corruptor' --glob '*.rs' .` returned no matches. The old
server case now calls read-only `prepare_tutorial_corruptor` and retains its
authority/unchanged-state assertions; successful consumption and replay assertions
are in the real dispatch scenarios rather than fabricated direct grants.

## Exact unresolved admission and pending proposal

The current Corruptor profile names corpse52113. Its canonical Item is inactive;
the current loot section marks it nonmaterializable with no container capacity.
Pinned Crystal XML gives capacity48, whereas the accepted Game reward-row contract
requires a materializable container with capacity1..30. No global gate relaxation
or silent source48→30 rewrite was made.

- `.questpc/newhaven-corpse-proposal.patch`: withdrawn, **unapplied** source48
  helper plus its one new regression test. It has never been regenerated or PG
  qualified. Applying it alone leaves the accepted corpse row closed.
- `.questpc/newhaven-native30-proposal.patch`: **unapplied**, explicitly labeled
  Game30 projection preserving source48 in a separate constant. Requires the
  lead's pending owner choice, complete loot-section correction, regeneration,
  and validation. It is not an active policy exception.
- `.questpc/newhaven-activation-proposal.md`: detailed source/lifecycle/ordinary
  loot correction packet. Research authority is `.questpc/research/newhaven.md`.

Full Viola additionally lacks its placed NPC/book acquisition and the existing
native source-specific atomic book burn + Scarf mint +800XP owner. The NPC lane's
storage-only writer refuses native item/XP effects as a whole; no starter grant
or fabricated hand-in was substituted.
