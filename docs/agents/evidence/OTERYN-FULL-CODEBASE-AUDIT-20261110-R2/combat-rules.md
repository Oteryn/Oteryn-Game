# Combat rules package audit

Base: `4f0b6c62a789489fca6c37dd36d5917d096f328e`. Status: REVIEWED — all29 tracked package files,5131lines, manifest/source/tests analyzed. Exact paths, hashes and dependency ranges are in ledger.json. No source edits.

## Entrypoints, invariants and failure behavior

AttackState owns runtime target/lineage, swing cadence and PvE in-fight expiry. New targets preserve the prior clock; stale/dead/nonattackable targets clear, temporary floor/range/PZ/reentry restrictions wait, and stalls yield one swing before scheduling from actual execution. The dispatch owner first reads live skill/current equipped weapon and applies ranged readiness, then preserves every original target gate. Its bounded ordinal is cleared at exhaustion, preventing a saturated pure sequence from reusing the same production draw lineage. Block refill advances on a fixed phase even while full.

Formulas build expression trees for the existing Content engine. Melee/monk/distance use their pinned floor and level-curve operations; defensive selection uses current known hand facts, shielding/weapon/fist skill and fight mode. AttackConstants comes from the embedded validated table; positive divisors, finite factors and exact parity metadata are enforced. Source parity remains explicitly pending where the tests preserve known uniform-mean residuals. This audit does not convert those accepted residuals into unsupported production bug claims.

Damage resolution separates immutable occurrence/purpose draw identity, defence, armor, immunity, source mitigation and elemental response. Positive integer domains are widened before multiplication, final representability refuses where required, leech never exceeds actual damage and reflection is capped by attacker max health. Actual dispatch critical rolls use1..10000 and fatal rolls0..10000; zero/out-of-domain arbitrary helper inputs are not wire actions. Existing RNG-001 seed visibility risk remains separately recorded and is not duplicated.

Chain selection respects qualified hit permission, sight, floor and visited identities; deterministic distance/id ordering prevents ambient iteration order changing sequential choices. The creature adapter bounds current players to64, rejects duplicate local IDs and mismatching runtime context; the player caller requires chain facts and rejects no first target. Pure coordinate arithmetic still assumes admitted world coordinates; no actual legal-map extreme-coordinate overflow was established. Forge is a complete re-export of Content; Content implementation coverage is separate.

PvP legality follows the accepted ordered rules and fails closed for unknown tiles/relations. Fresh login restricts initiating aggression, not retaliation; that is the explicit contract. Hardcore bypasses secure/expert restrictions but retains level/PZ/black-skull gates. The field adapter sets secure=false/RedFist before common legality, while retaining durable relations and current tile/identity checks. CombatLock deadlines only rise; durable epoch kill expiry must be translated to owner semantic time by the separate live owner. Bounded aggression relation eviction only removes expired entries.

Skull calculations use strict rolling-window lower endpoints, positive bounded milli-points and a separate one-time increment. The actual SQL history adapter excludes the current death occurrence, pages in timestamp+UUID order and uses the same exclusive window boundary, refuting duplicate current-death counting. Current higher skull severity/later deadline cannot be shortened by evaluation. Blessing disposition has explicit Adventurer/ordinary/Twist/red-black precedence; item-death custody is outside this pure helper and prior DUR-003 is not repeated.

## Refuted hypotheses and limits

- A ranged shot is not accidentally rejected by the melee one-tile gate: the caller validates qualified range first, then clamps only distance for the shared target gate.
- Target changes and stalls do not provide extra swings; timer state is retained and cadence restarts from execution.
- Field secure-mode refusal is not reachable through the field adapter: secure is explicitly disabled there.
- The current death's unjustified point is not counted twice in skull evaluation.
- A custom malformed armor threshold could violate an internal arithmetic precondition, but production uses the embedded threshold3 and no external table loader caller was found. No reachable issue asserted.

## Findings and test evidence

No new confirmed finding. Every package source and test file was reviewed; no Cargo test or combat/SQL integration was run for this package. Test assertions were assessed for boundary coverage, not counted as passing. Existing character standalone tests are unrelated. Remaining package files: none. Wider combat/dispatch/durability owners are credited only for the exact dependency ranges listed.
