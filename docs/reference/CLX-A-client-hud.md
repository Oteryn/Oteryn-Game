# CLX-A: client HUD and character panels

Base: `e91bce557728e52f5d61c38f8fad7dba3c1ee1d3`. Client-only changes;
no migration, capability, command, domain or protocol-message allocation.
The only server-tree addition is a focused PostgreSQL test of the existing
character-query caller; no server production behavior changes.
No changes to S17 casting, quest content, Dawnport, starting equipment, OT-only
features, or Prey Hunting Tasks. The Newhaven tutorial remains the equipment owner.

The status audit describes `c475b511`, before the base's #2994/#2997 client
implementations. Reconcile its gaps against the actual base before writing code.
The mechanics ledger and its JSON are deliberately unchanged.

## Sources and presentation decisions

Existing exact-revision donor evidence is reused from
`docs/reference/gap-cl1-client-combat-hud.md` and `docs/reference/client/GAP-CL2.md`.
Crystal: `zimbadev/crystalserver`, `main`,
`201189f871d23597b82caf88fd611454a2ddefb8`, class `OTS_HYPOTHESIS_ONLY`.
Paths below are relative to that revision. No new combat formula is adopted.

| Fact | Donor evidence | Native behavior / difference |
| --- | --- | --- |
| Soul; Harmony and Serene | `src/server/network/protocol/protocolgame.cpp:4400-4409,11527-11541` | Show the already decoded ActorVitals values in the HUD and the existing native action-bar window. Harmony uses the existing native maximum 5 (`actor_spell.rs:MAX_HARMONY`). No inferred rest or Battle Instinct state. |
| Creature light level and colour | `src/server/network/protocol/protocolgame.cpp:9790-9796` | Keep the existing negotiated VIS-2 fields 11/12 under capability 19. Preserve them in the session's atomic visibility store and existing VisibleAppearance publication. Remove on unlit update, leave, or replacement; never invent an expiry timer. |
| Light composition | Donor packet specifies source values, not the renderer algorithm; native WORLD_INTERACTION0 decision sections 10.2/11.4/14 explicitly make light presentation client-owned | Local presentation choice: level is radius in tiles, linear radial falloff, component-wise maximum with ambient, one sample at each quad centre. The existing 6x6x6 indexed colour cube is retained. This is a recorded native rendering difference, not a claim of an exact proprietary-client falloff formula. |
| GPU composition | Native renderer owns sRGB decoding and encoding (`scene_gpu.rs`) | Extend GPU instance uploads with a linear-space RGB multiplier, multiply sampled RGB, preserve texture alpha. Keep the authored atlas unchanged so one sprite can have different illumination at different positions. The UI palette remains emissive. Weather retains its existing atlas treatment. |
| Combat modifiers / stamina | `src/server/network/protocol/protocolgame.cpp:5016-5325,4401-4405` | Retain #2994/#2997's complete strict decoder, ppm units, unavailable-vs-zero distinctions and hours:minutes stamina display. |
| Ordinary target / cursor casts | `src/creatures/combat/spells.cpp:1238-1261` | Retain #2997's real ordinary-spell command, target/no-target behavior, Shift-digit cursor selection and cancellation. No S17 production or test changes. |

## Row disposition

| Row | Result | Evidence / remaining boundary |
| --- | --- | --- |
| C03 | DONE at base; retained | All 84 admitted combat fields render through capability 199/command 388, including critical, leech, reflection, signed values and unknown facts. `combat_modifiers.rs`, native `gap_cl1_session_loop_delivers_combat_modifiers_and_stamina`. Character panel paths owned by CLPKG5 are untouched. |
| C08 | STOPPED: needs server projection | `actor_condition_snapshot_candidate.rs:1-4,32-45` has no routed capability and no active-condition catalogue/deadlines. Existing ActorVitals contains no such list. Cannot decode full conditions or durations from published data. |
| C10 | STOPPED: needs server projection | Soul is now visible and tested. `actor_spell.rs:128-136` carries no rest eligibility, regeneration state or deadline; `cyclopedia_character.rs:GeneralStats` adds none. A complete rest view requires server-owned facts. |
| C11 | STOPPED: needs server projection | Harmony and Serene are now visible and tested. ActorVitals has no Battle Instinct observation; attack state has only target/modes/in-fight. No inference from generic combat state. |
| C14 | STOPPED: needs server projection | GeneralStats already renders level/experience/skill levels, but no skill tries, thresholds, mana-spent or percentages cross this boundary. Producer `durability/cyclopedia_character.rs:GENERAL_STATS_SQL` and `:96-118`; donor sends progress at `protocolgame.cpp:4387-4389,4421-4433`. |
| C15 | DONE at base; retained | Strict GeneralStats stamina decoder and hours:minutes formatter, including 0/60/2520-minute boundaries. No inferred premium/XP multiplier from absent facts. |
| C18a | DONE at base; retained | `tools/dev-client/tests/gap_cl2_spell_targeting.rs` drives real admitted-session production combat routing, decodes AttackTarget and None/directional intents, and checks server refusal feedback. |
| C18b | DONE at base; retained | Native `local-cursor-spells/tests/play.rs` drives the click-to-combat queue without walking/attacking; the admitted-session test proves ordinary command 3 carries the selected position, independent of an opponent. |
| C20c | STOPPED: needs server projection | Death settlement reconciles durable consequences server-side (`composed_admission/death.rs`). Native death handling only receives zero/restored health; neither ActorVitals nor party/character tab data provides a death-correlated consumed-blessing/Twist result. Current blessings are not evidence of which protection a particular death consumed. CLPKG5 owns death handling and is untouched. |
| S13c | STOPPED: needs server projection | `party.rs:PartyEntry/PartyResult` has membership/name/leader/invitation expiry/shared XP, no active effects or their removal/deadlines. Donor `protocolgame.cpp:7899-7914` and `data/scripts/spells/party/protect_party.lua:7` do not supply the missing native contract. |
| W20b | Implementation and automated native play DONE | Session decode -> visibility store -> existing appearance publisher -> native scene -> GPU upload/shader. Tests cover own and creature lights, expiry/removal, floor filtering, rebuilds, radial falloff, colour, emissive UI and upload layout. Physical Windows/GPU play remains unexecuted in this Linux environment. |
| P12c | DUPLICATE-ON-BRANCH; skipped | Full GeneralStats/death/PvP/tab decoding is already in base #2994. CLPKG5 owns further character panel/registration work; do not edit those files. Its supplied head is `5b4376f89722e7ba2cedebfa6c43e55164e755c4`. |
| S16b | DONE for the owner-approved no-Dawnport slice at base; retained | Existing GeneralStats and offence/defence/misc tabs show admitted character statistics; capability 274 shows all eight base/loyalty skill values in the native panel. HP/MP maxima already render; soul/Harmony/Serene now render too. Dawnport magic-level cap is excluded by the explicit owner decision, not silently implemented. Skill progress remains C14's stopped server dependency. |

## Overlap and review

The coordinator supplied CLPKG5's exact head and changed-file list, SRVPKG3
`c97858c3f440fd26676d3bc9be6d6a35b2c711ff` and its list, and confirmed QPKG3
does not exist remotely. No listed path was edited. The lighting change uses
the already existing appearance publication instead of adding mailbox, Stepper,
play-loop or shared character-panel wiring. No quest content is authored.

Self-review covers every changed line and the production callers. Visibility
changes are applied to a clone before validation; failed deltas cannot leak light
changes. Sources stay bounded by the existing 256-entity visibility limit. The
new GPU stride, allocation and vertex attributes use the same 44-byte layout.
The old global-light regression now checks the uploaded lighting state and
unmodified texture instead of requiring CPU atlas darkening; it retains its
ambient, persistence, target and name checks. No test is disabled or loosened.

## Reproduction

Source `/workspace/oteryn-env/env.sh` for every independent Cargo/PG shell.

Red checks retained outside git:

- `/tmp/clxa-red-vitals.log`: `cargo test --locked -p oteryn-client --lib clxa_soul --quiet`; failed because Soul/Harmony/Serene were absent from HUD text.
- `/tmp/clxa-red-scene.log`: `cargo test --locked -p oteryn-client --lib clxa_character_light --quiet`; failed with 0 sources instead of 1 before native scene wiring.
- `/tmp/clxa-red-session-runtime.log`: `cargo test --locked -p oteryn-session clxa_entity_light --quiet`; failed with None instead of Some when the previous discard-on-snapshot behavior was temporarily restored. The source was restored in a finally block.

Green/final validation commands and exact filters:

```sh
cargo fmt --all --check
cargo clippy --locked -p oteryn-session -p oteryn-renderer -p oteryn-client-ui -p oteryn-client --all-targets --quiet -- -D warnings
cargo clippy --locked -p oteryn-gameplay-dispatch --all-features --all-targets --quiet -- -D warnings
cargo test --locked -p oteryn-session --quiet
cargo test --locked -p oteryn-renderer --quiet
cargo test --locked -p oteryn-client-ui --quiet
cargo test --locked -p oteryn-client --quiet
cargo test --locked -p oteryn-dev-client --test gap_cl2_spell_targeting --quiet
oteryn-architecture-check workspace .
cargo test --locked -p oteryn-gameplay-dispatch --all-features --lib cyclopedia_character_tabs_postgres_real_dispatch -- --nocapture
cargo test --locked -p oteryn-gameplay-dispatch --all-features --lib crosshair_spells_postgres -- --nocapture
cargo test --locked -p oteryn-gameplay-dispatch --all-features --lib clxa_hud_postgres -- --nocapture
```

No full nextest or full PostgreSQL suite is used. The new `clxa_hud_postgres`
case checks real database values through existing command 388, with distinct
skill values and stamina boundaries 0/60/2520 minutes. The other PostgreSQL
commands reverify existing producer scenarios. No Windows target, tools or packages
were installed; native tests exercise the actual app projection headlessly.

Executed PostgreSQL evidence: character tabs 1 passed in **3.40 s**; crosshair
spells 2 passed in **68.49 s**; the new HUD case 1 passed in **3.33 s**, printing
`POSTGRES_STARTED CLX-A` and confirmations for stamina 0, 60 and 2520. These are
test execution durations, excluding compilation. Client crate tests: session
159, renderer 44, UI 258, app 299, and admitted-session targeting 3 passed.
