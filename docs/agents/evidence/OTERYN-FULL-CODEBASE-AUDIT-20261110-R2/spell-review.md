# Spell — complete package source review

Input: `4f0b6c62a789489fca6c37dd36d5917d096f328e`. Package: `oteryn-gameplay-spell`, `crates/gameplay/spell`. Status: **REVIEWED**, not runtime VERIFIED.

## Evidence and coverage

All **83 tracked package files / 41,011 lines** have substantive review coverage. `spell-coverage-validation.json` independently compares every package file with the exact immutable Git blob, checks each attributed SHA-256 and line endpoint, merges ranges, and confirms an unchanged source checkout. This verification checks the review record; it does not perform source review.

All ordinary source and test text was read. Two long embedded JSON literal lines were reviewed semantically in full: `creature_damage_spell/composed.rs:1014` and `creature_damage_spell/mod.rs:2727`. The analysis follows all 16 composed cases, 12 ability bodies and 32 defense profiles, including every one of the 31 ordered 6-second magic-percentage variants. Exact selected native closures, executable/formula/filter/area links and complete effect accounting passed **817 read-only checks**. The selected 15 records of the separate native catalogue receive no full-file review credit. The initial four overly broad semantic assumptions and their dispositions remain retained: two were legitimate presentation-only inline effects; two were the repository's explicit positive-ICE Frozen Minion healing correction. This establishes the repository contract, not an independently executed donor.

Package test files were inspected as source. No spell Cargo suite, database migration, SQL concurrency test, native client or deployed-server reproduction was run.

## Cast authority, resources and numerical effects

The review follows `spell/mod.rs`, `cast.rs`, the complete `cast/` directory, `owned_cast_facts.rs`, `magnitude_owner.rs`, `native.rs`, `native_combat.rs`, `combat_batch.rs` and the execution adapters. Pure cast plans are distinguished from authority to mutate actual owners. Current actor/session, source index and content, equipment, learned state, Premium, positions, protection and current policy are checked separately. Prepared payment and typed physical successors must match the original state and intent; the later installation phase relies on validation in the same owner turn.

Formula inputs are separated from durable base skills and admission facts. Wheel positional, Runic, Battle Instinct and specialization adjustments are inspected through their current-owner producers and formula-only consumers. Damage ordering includes current immunity, armor, resistance, PvP, equipment and Wheel effects. Known strict source shapes refuse unsupported facts instead of silently producing a generic action. The narrow legacy physical commit seam explicitly refuses without the live attacker fence; its test-only bridge is not counted as production wiring.

Existing effective-magic gate evidence is corroboration under **GP-001**, without extending the claim to ordinary combat. The existing **GR-SOUL-001** also has a separately qualified live level-up path: current committed level gain can install the free-cap facts before current promotion/Premium restoration, which preserves the already-clamped Soul. See `verify-soul-cap.md` and `level-up-soul-extension-validation.json`. It remains one P2 finding, not another ID.

## Conditions, regeneration, training and lifecycle

Complete condition, food, equipment regeneration, Soul-window, field-history and periodic-execution sources were reviewed. A helper-only resurrection hypothesis is refuted by the actual alive-owner admission and the periodic zero-HP stop. Source-specific condition semantics, protection handling, regeneration windows and death cleanup are treated individually. Gift of Life requires current qualifying Wheel state, an independently loaded timer, live HP and its source overkill condition; timer generation and save acknowledgement participate in paid-successor validation. Offline time is excluded from the loaded remaining cooldown.

`mana_training.rs` follows actual paid mana, the sealed direct-acquisition source-mana exception, the Drome off switch, bounded pending points, immediate level advancement and cadence/forced checkpoints. A paid cast banks mana while preserving the pending skill window. A retained unknown checkpoint banks subsequent hits separately, merges them only with the matching receipt, and drains reached levels or forced-save leftovers before reporting completion. Death rebase retires prior checkpoint/payment witnesses. First-vocation and trial conversion preserve the existing conditions/cooldown owner and replay does not refill resources again.

## Companions and familiar persistence

The complete companion execution/lifecycle implementation and tests were read. Summon, Convince, Animate Dead and familiar operations require the actual current source, census, physical reservation, policy and appropriate durable receipt. Source mana exceptions do not follow from an arbitrary numeric input. Haste stages the intended familiar conditions without modifying ordinary summons.

Familiar login distinguishes a foreign open crash row, a clean stored remainder, this session's own clean end and an already open current acquisition. Waiting returns bind the current writer and lifecycle epoch; acquisition advances that epoch. Cast/advance cannot adopt an unreconciled foreign open row. Death and expiry close the actual acquisition with zero remainder. Clean actor end preserves the actual remaining timer or a legitimate waiting remainder. Stored return after a change of fresh-cast Premium/level eligibility is an explicit source contract; it is not evidence of an entitlement bypass. Full census, placement and receipt checks precede physical installation. No duplicate familiar creation path was established by this package review.

## Current world actions, delayed effects and creature spells

House, Locate, movement/world actions, source map initialization and durable stances were followed through their typed owner boundaries. House actions use current physical membership and durable access facts. Locate requires current admitted identity/visibility/scope facts. World mutations need the actual transaction and current map/item owners; a proposed or stale position alone is insufficient.

Delayed and ordinary timers retain the original occurrence and qualified numerical result where replay requires it, and independently check current actor/session/owner state. Creature windups use the existing bounded owner timer lane and a private one-use due pulse. Cancellations precede consumption for stale issuer, content or target. Snapshot metadata cannot manufacture a pulse.

Creature spell composition binds the exact creature, schedule entry, ability/formula closure, content and current owner. Complete bounded recipient sets are preflighted, rather than silently truncated. Primary and callback writes are independent owner operations with their retained results; a durable all-target transaction is not claimed. Appearance, typed damage, mana drain, source conditions, Hazard and source-critical paths were inspected. Explicit repository approximations and stronger protection gates retain their qualifications. Remaining cross-package caller questions are not promoted to findings without actual producer and counterguard evidence.

## Presentations and tests

The presentation owner binds allowed source cue identities and qualified positions into canonical cast intent. It reserves the full bounded queue obligation before draw/write, retains that obligation while commit is unknown, releases only a definitely uncommitted hold, and emits nothing for a replay receipt. Current content/scope/generation and source/target positions are revalidated before installation. Familiar cues require a matching sealed physical defense receipt. Draining the current outbox does not establish network delivery.

Source tests cover gate and payment boundaries, target ownership, chain geometry/selection/ties/caps/rounding, retained lethal replay, recycled actors, control loss, unsupported behavior refusal, exact cooldown deadlines, training replay, familiar lifecycle and presentation holds. Tests that construct synthetic worlds or begin after source qualification retain that limited meaning. No new confirmed finding was added solely from a synthetic fixture or an unqualified helper.

## Outcome

The package satisfies the source-review completion criterion. Runtime integration, deployment activation and database behavior remain qualified separately. Confirmed findings are deduplicated under the existing register; unresolved cross-package questions remain outside confirmed totals. No implementation files were changed.
