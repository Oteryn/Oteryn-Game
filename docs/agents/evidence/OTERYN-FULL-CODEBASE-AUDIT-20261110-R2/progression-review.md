# Gameplay progression — complete package review

Input: `4f0b6c62a789489fca6c37dd36d5917d096f328e`. Status: REVIEWED. All 43 tracked files in `crates/gameplay/progression` were read and analyzed in full, including module assembly, property tests and parity fixtures. Selected callers outside this package have separately bounded ledger entries. No source changes were made. This completion records source analysis; it does not claim that package tests, PostgreSQL transactions or native client/server scenarios were executed.

## Achievement identity and grant boundaries

`account_achievement.rs` validates bounded achievement keys, source-event identities and revisions before constructing values. `achievement_catalogue.rs` binds the two embedded shards to their expected identities and rejects malformed, duplicate and empty catalogues. The embedded JSON records are outside the package and require their own semantic review; reading the loader and its tests is not coverage of those records. Absent and retired achievements remain distinct catalogue outcomes.

The granters classify successful gathering, item use, committed NPC trades and completed Bestiary stages. They do not authorize or persist an award. Gathering checks distinguish tools and valid resulting items; marble figurines are excluded from the relevant counter. The Yasir rule explicitly follows the committed-trade adaptation recorded in the implementation, rather than treating an opened dialogue as a completed sale. Bestiary stage projection uses the supplied increasing thresholds and caps its final stage; completion rewards are separate from ordinary progress. A later audit of the durable writer remains necessary to qualify replay, account locking and receipt behavior.

## Training, loyalty and ranking

`hit_training.rs` keeps two bounded runtime streaks. A successful hit refills them, blocked hits consume the attack streak, immunity clears the attack grant, and shielding requires both a remaining streak and a shield. Distance misses intentionally retain the previous block result. This behavior is documented and tested in the source; it is not independently evidence of a training exploit. Skill-family order is explicit and matches the stored build index contract.

`mana_training.rs` and `loop_training.rs` use bounded progress and level arithmetic. Offline training separates the initial ten-minute delay, consumed training pool and subsequent refill; the retained pool and elapsed-time caps prevent subtraction from exceeding its source. Exercise and offline rates come from the pinned training file. That entire file was also read as a separate cross-reference. Its fist-wrap and fist-statue entries use family 1, which writes skill index 0. A misleading family-6 comment does not establish the initially suspected fist-to-shielding mapping defect. Rate rounding and the house multiplier occur at their documented integer boundaries.

Loyalty projects an effective skill from cumulative points and a bounded percentage without rewriting the stored training state. This does not close the previously reported cross-package effective-magic inconsistency: callers must still use the intended effective projection. Promotion checks level, existing promotion and the separate current-Premium/pre-delivery permission. Login presentation is not a grant.

`ranking.rs` accepts an already eligible complete snapshot cut. It rejects duplicate Characters, sorts by descending value with exact name/identity tie breaks, preserves competition ranks and applies each list's cap after vocation filtering. Vocation lists intentionally retain the global competition rank. Account eligibility and representative selection belong to the snapshot producer and were not inferred from this pure sorter.

## Daily rewards and regeneration

Daily reward state is keyed by the durable world-reset epoch. It validates cycle, streak and last-claim shape, consumes missed-day jokers as a whole decision, and preserves the specified cycle behavior when days are missed. The civil-time helper expects an upstream local-time conversion; it is not a fixed UTC scheduling rule. Picker validation bounds the current offer, sorted distinct IDs, positive quantities and total selection. The authored reward table supplies the actual rewards, including the vocation-none fallback. Reading the exhaustive 30-day test table does not prove a concurrent claim transaction.

Regeneration has separate fed/unfed timing, fractional carry and per-equipment clocks. Dead actors receive no credit; maximum pools cap gains and clear carry. PZ and daily-streak rules selectively permit equipment regeneration. Effective promotion depends on Premium. The code and tests explicitly record the owner-directed Monk and always-on percentage changes; differences from historical donor values are not automatically defects. Potion effects use their closed item/vocation mappings, bounded duration and checked magic-shield calculation. The shared skill-potion sub-ID is explicitly documented as source behavior.

## Proficiency and durable intent values

`proficiency.rs` distinguishes lexical/storage validation from actual admitted definition shape. Full-width progress is retained up to PostgreSQL's nonnegative BIGINT limit. Training and catalyst changes must increase progress on the same track and revision; selection changes affect exactly one choice; migration and modification have distinct permitted directions. Actual authority and payment are supplied by the durable adapter.

Request normalization rejects mixed causes, duplicate tracks and invalid revision/cardinality combinations. Occurrences require the UUIDv7 version/variant shape. The versioned intent digest includes Character, expected revision, cause, track revision, ordered before/after values and policy digest. String separators and explicit choice tags avoid ambiguous framing for already checked values. A digest proves binding only, not permission or commit provenance.

## Wheel, gems and Atelier

The Wheel reader binds embedded files to exact manifest hashes and compatible envelopes. Complete offline schema qualification remains separate from this runtime loader. Allocation uses 36 bounded slots, known capacities, four nine-slot domains and five vocation families. Its finite adjacency pass opens only slots connected to an already open full predecessor; a disconnected cycle cannot authorize itself. Spendable points and ring-unlock points are separate, so Grade-IV bonuses do not unlock a ring early. Removing points uses the runtime's current combat/PZ permission despite the compatibility parameter name. A strict decrease after level loss remains subject to the other shape and adjacency rules.

Revision reads distinguish current, reset-required and unresolved capacity states while retaining stored allocations. Stage derivation fails closed for an ineligible or noncurrent allocation. Runtime effects preserve fractional percentages, use checked aggregation and apply individual gem grades and per-gem rounding. Signed resistance penalties remain signed. Source-specific modifier corrections and double mastery additions are explicitly encoded; their historical provenance has not been independently re-established by this package review.

Atelier prepares a clone, performs all resource, shape, revision and vessel checks, increments its revision and only then publishes the new state. Refusals therefore do not publish an earlier partial debit. Reveal validates vocation and modifier position, unique identity, quality, available count and fee. Lock, vessel occupancy, domain rotation, last-domain dismantling, fragment bounds and shared grade revision have separate guards. Serialization preserves wide gem identities and rejects duplicate grade keys. The unchanged parity tests were fully read but not run.

The pure gem-contribution helper is not an authority boundary. Actual callers in `spell_access_facts.rs`, `composed_admission/wheel.rs`, `wheel_presentation/effects.rs` and `character_wheel/capacity.rs` check current allocation and applicable level/promotion/Premium facts before invoking it. The capacity caller in `item_transfer.rs` separately requires current account Premium. These counterguards refute the proposed bypass based solely on calling the pure helper after a zero-effect projection. They do not certify every upstream transaction or cache invalidation path.

## Charm evaluation and actual consumers

The evaluator requires the expected Character's state, a race-specific assignment set, valid stages and at most one charm per category. It rejects inconsistent committed-hit facts. Deterministic rolls bind the decision root, occurrence and distinct hook/category index; replay returns the same evaluation without itself authorizing repeated application. The caller must retain a unique occurrence per actual target event.

Generated charm damage cannot chain another attack/kill charm. The explicit secondary auto-attack exceptions admit critical and leech effects while excluding ordinary procs and Carnage. Living-target effects do not apply after the target dies. Damage is bounded by the selected creature/attacker maxima and level caps, with checked or widened percentage arithmetic. Mana drain has explicit Numb/Adrenaline routing and excludes Cleanse on that event.

Historical `FailedClosed` markers in the pure engine are not proof that a current gameplay consumer is absent. The actual attack adapter checks current actor lease and state and explicitly discharges selected markers at implemented consumers. The reviewed range includes death-loss capture, owner/session/occurrence fencing, Gut and critical effect handling. Complete downstream consumer and durable settlement review remains part of the dispatch/durability work.

## Verification and limits

All 43 package files are represented by full-file review entries with exact SHA-256 and line ranges against the immutable base. The ledger is independently checked against Git blobs and the clean worktree before publication. Every package test source was read, including the full 1,594-line Charm suite and the Bestiary/Charm parity matrix; none of those tests was executed for this completion. No Cargo build, database run or native test is claimed. No additional confirmed finding is added by this package report. Cross-package findings and outstanding caller, content-admission and transaction investigations retain their existing separate status.
