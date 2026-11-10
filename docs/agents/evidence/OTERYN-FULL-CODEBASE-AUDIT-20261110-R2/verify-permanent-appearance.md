# CL-016 — Permanent environmental appearance is removed by a periodic condition pass

Status: **CONFIRMED (static source proof), P2**. Base: `4f0b6c62a789489fca6c37dd36d5917d096f328e`.

## Evidence and actual outcome
The movement owner creates a permanent swimming Outfit or bath ItemOutfit, applies it to the current player spell state and commits that state. These definitions deliberately have no expiry; their condition instances have `ends_at=None`, no scheduled tick and `remaining_total=0`. The ordinary qualified periodic owner nevertheless runs `take_due` or `take_due_non_damage`, whose final retention keeps a no-expiry instance only if its remaining damage total is positive. The appearance is therefore removed even when no tick was due and the player has not left the environmental tile.

The staged periodic cycle detects the changed conditions, advances the revision and installs the successor. A subsequent qualified appearance observation selects the base appearance when the temporary condition is absent. This is a state-lifecycle and presentation defect. No movement, item ownership, combat immunity or authorization bypass is asserted.

## Independently inspected source chain
| Source | Reviewed evidence |
|---|---|
| `crates/gameplay/dispatch/src/gameplay_transport/paced_movement.rs:165–220` | Successful real movement calls the special movement owner. |
| `crates/gameplay/dispatch/src/gameplay_transport/paced_movement/special_movement.rs:250–315,370–485` | Production swimming/bath definitions, intended exit removal, application and revision commit. |
| `crates/gameplay/foundation/src/foundation/condition.rs:240–285,1045–1105,1348–1380` | Permanent definitions and the zero-total, no-expiry instance representation. |
| `crates/gameplay/dispatch/src/gameplay_transport/composed_admission/vitals.rs:150–195` | Native and non-native actual periodic routes. |
| `crates/gameplay/dispatch/src/gameplay_transport/spell_periodic.rs:325–380` | A player with no periodic damage goes directly into the environmental condition cycle. |
| `crates/gameplay/dispatch/src/gameplay_transport/actor_spell.rs:780–870` | Current control and tile facts, staging, counterguards and successor installation. |
| `crates/gameplay/spell/src/spell/actor_conditions.rs:490–655,1470–1555` | Staged store tick, state assignment, change detection and revision publication. |
| `crates/gameplay/foundation/src/foundation/condition.rs:1520–1595,1840–1870` | No due appearance occurrence is needed to reach unconditional erroneous retention. |
| `crates/gameplay/dispatch/src/gameplay_transport/appearance_change/observation.rs:115–230` and `crates/gameplay/foundation/src/foundation/condition_spell.rs:385–425` | Current condition lookup and base-look fallback. |

## Counterguards and verification limits
`expire_non_ticking` preserves no-expiry appearances (`condition.rs:900–940`), but only the branch with unavailable tile facts uses that narrower operation. An ordinary controlled living player on a qualified tile reaches the full tick branch; a spell commit in flight or missing control can defer it. Explicit StepOut removal is correct and is not required for this failure. The existing permanent ItemOutfit test reads a future appearance and removes it explicitly (`condition_spell.rs:585–630`); it does not call the periodic owner.

The lead independently inspected the producer, actual consumers, state transition and counterguards after the client/foundation reviewer submitted the candidate. Exact reviewed files were compared with Git blobs at the pinned base and hashed. No Cargo build, native client/server run, SQL execution or new test was performed for this finding. Runtime frequency, deployed content and observed client rendering remain unverified.
