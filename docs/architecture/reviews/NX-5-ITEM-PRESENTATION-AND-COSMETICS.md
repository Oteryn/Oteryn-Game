# NX-5 — item presentation and canonical cosmetic unlocks

Task contract: W19c/W19d against `faf8ae8700d935bca612261730aca5466526389a`.
The task authorizes admission of the evidenced Global item mechanics; no Dawnport,
casting, starting equipment, Store, or other OT features are introduced.

## Production selection and scope

The canonical authoring is materialized with the existing validating producer:

```sh
python3 tools/content-schema/cosmetics-authoring/cosmetics_authoring.py --authoring content/cosmetics/outfits/authoring.json --output content/cosmetics/outfits/native-cosmetics.json
python3 tools/content-migration/register_spell_families.py
```

The registrar pins the cosmetics section in the default `content/spells.manifest.json`
and promotes it to V6. The ordinary node/game-ops manifest loader and existing
issuance/activation owner then activate the V9 source-bound cosmetics section.
No alternate deployment profile, implicit loader fallback, inferred numeric identity
or unchecked admission bypass is needed.

The bounded catalogue has Citizen as the starter/fallback, thirteen ordinary
item-unlock outfits, eight mounts and 49 explicit provenance-compatible grants (47 item grants and two existing quest-base grants)
covering all 46 non-OT item rules. This completes the item-unlock slice, not the
broader S06 quest/taming population. Base outfits acquired by quests remain
non-starters; a missing earned base still refuses an addon item. Existing account
scope, Premium acquisition gates, mounted-speed rules, duplicate handling,
component counters, exact replay, receipts and session fences remain in force.
Random addon doll 6579 and random mount item 37746 are explicitly refused by the
production cosmetic USE route under the owner's no-OT-only decision. Historical
receipts remain readable. Source-only tests/catalogue entries remain evidence.

## Evidence and admission decisions

Crystal is `zimbadev/crystalserver`, revision
`201189f871d23597b82caf88fd611454a2ddefb8`; donor paths below are relative to that
repository. These are donor rule hypotheses, cross-checked against the existing
accepted Game contracts and staged structured outfit evidence. Each authoring
field has a value-bound promotion; `decision: NX-5` refers to this document and
the owner's NX-5 task contract, not a new owner policy.

| Rule | Donor / accepted evidence | Result |
| --- | --- | --- |
| Ground rocket draws yellow/red/blue at source tile; carried rocket emits fire area then hit-by-fire at player, loses 10 HP; consumption/replay share receipt | `data/scripts/actions/items/fireworks_rocket.lua:3-15` | Retain existing rules, publish committed cues in order once |
| Effect IDs and firework colours | `src/utils/utils_definitions.hpp:62-86` | 7/16 carried effects; 29 yellow, 30 red, 31 blue |
| Present effect at player | `data/scripts/actions/items/explosive_present.lua:3-7` | Public positional fire cue |
| Equipped party hat cue, no consumption | `data/scripts/actions/items/party_hat.lua:3-12` | Public gift-wrap cue |
| Die and trumpet cue source | `data/scripts/actions/items/die.lua:3-10`; `party_trumpet.lua:3-7` | Existing retained source position/transform rules; carried trumpet has no world-tile cue |
| Voodoo target effect is private to the using player | `data/scripts/actions/items/voodoo_doll.lua:9-17` | Exact actor/session queue; never public map journal; no second cue on replay |
| Costume effect at player | `data/scripts/actions/items/costume_bags.lua:12-19` | Public retained magic-red cue; existing costume duration unchanged |
| Quest-earned prerequisite bases remain usable by addon items | `crates/gameplay/durability/src/durability/character_appearance/quest_grants.rs:18-32`; real writers `quest_state/soul_war.rs:161-165`, `quest_state/rotten_blood.rs:582-583` | Explicit compatible mappings for Revenant and Decaying Defender, no quest implementation changes |
| Voodoo default target reach | Crystal `src/lua/creature/actions.cpp:269-280,688-693` | Same floor and one square; a visible far target is still TooFar |
| Outfit gender pairs, base/addon requirements, premium check, duplicate no-spend, full-outfit achievement | `data/scripts/actions/items/usable_outfit_items.lua:1-102` | Existing transaction rules now resolve through explicit canonical grants |
| Afflicted six-piece collection and addons | `data/scripts/actions/items/usable_afflicted_outfit_items.lua:4-51` | Existing nested/carried/ground custody rules unchanged |
| Five direct mount tokens | `data/scripts/actions/items/usable_mount_items.lua:1-34` | Numeric mount IDs win over misleading donor display labels (23538 is Sparkion, ID 94) |
| Multipart mounts | `usable_phantasmal_jade_items.lua:2-34`; `usable_singeing_steed_items.lua:2-32`; `usable_spirit_of_purity_items.lua:2-32` in `data/scripts/actions/items/` | Existing counters and one-time grant, canonical mount target |
| Mount identities, client looks, metadata | `data/XML/mounts.xml:123,127,128,191,196,213,246,261` | Eight explicit key/look mappings; source digest in each record |
| Outfit names, sex-specific looks, premium, two addons and acquisition | `docs/reference/content-prep/outfits-content-1/admission-evidence/evidence-00000-00135.json` (exact record pointer in each citation) | Structured evidence determines sex/name; donor numeric order never determines sex |
| Citizen starter | Same evidence, `/records/22/facts/base_acquisition` | All players have base; neither addon is a starter |
| Palette, account ownership, compatibility, mount speed, fallback mechanism | `OTERYN_GAME_CHARACTER_APPEARANCE_OWNER_DECISION_2026-09-28.md`, §§2–4; `tools/game-atlas-appearances/export.py:574` | Existing 133-colour palette, +10 profile fact, explicit no-addon/no-mount fallback |

Fallback colours `[0,0,0,0]` are an explicit content choice inside the accepted
fallback mechanism, not an assertion about Tibia character-creation defaults.
Native presentation keeps the existing schematic emissive renderer: firework
colour IDs now produce distinct eight-spark bursts; other toy cues use their
colour and the existing pulse. This is visual representation, not a new gameplay
rule or redistribution of third-party sprites. Official effect sprite animation
is not claimed by this change.

## Runtime ownership

Only successful settlement of the original pending item effect publishes visuals.
The captured cue position precedes the durable await, including ground and player
targets. Unknown commits retain pending owner work. Historical receipts without
pending work emit nothing. The bounded production presentation journal now accepts
item effects alongside sounds, with independently negotiated capability 350/domain 102, with independent observer cursors, existing exact
runtime/session checks and reference visibility. Player-only voodoo effects are
filtered by the exact acting actor. Reconnect setup skips historical journal entries.
The main consume/transform owner publishes directly after settlement; no second
item owner, item-action outbox, or deleted receipt table is needed. Presentation
stays best effort. Casting presentation remains unchanged.

The native appearance feature decodes the existing catalogue, displays newly
unlocked outfits/addons/mounts, and preserves draft selection. No grant is inferred
from a toast or item result: choices still come only from the server catalogue.

## Overlap and numbers

The coordinator's consolidated base removes the duplicate item owner in favour
of main #2970. This patch keeps that main owner and has no dependency on the
removed usable-action files, grants, migrations 0970/0971 or presentation cause.

Numbers: capability 350 `ITEM_EFFECTS_V1`; next-free state domain 102 with empty
snapshot type 1 and item-effect batch delta type 1. No new commands or migrations.
Capabilities 351–354, commands 600–609 and migration 0993 remain unused. Capability
23 stays unoffered, and S17 cast behavior is unchanged: visual capture comes only from the main item settlement owner. The new scoped stream rejects every non-MagicEffect payload.

## Verification record

Initial red evidence on the original task base: the ground-rocket PostgreSQL regression received no live cue
(45.19 s); the production manifest PostgreSQL case refused the absent authored
input (3.51 s); the content coverage test failed on missing authoring. Both native
client regressions failed on the absent visual distinction / unlock feedback.

Final validation commands and measured results are returned with the task diff.
The catalogue `authoring.json` is maintained source data: explicit accepted facts
and value-bound promotions, never rewritten by a generator. The repository
producer writes a small input descriptor pinning that source, the existing staged
Outfit source manifest, and the expected derived section digest. The production
manifest loader assembles the native section in memory, verifies all file pins
and the resulting digest, then uses the existing strict cosmetics decoder.
No generated packet larger than 200 KB is committed or needed for transfer.

## Focused validation after the owner blockers

All runs below use base `faf8ae8700d935bca612261730aca5466526389a` plus this
uncommitted patch and the retained PostgreSQL 18.6 environment. Both W19c and
W19d have passing server and native client coverage on that base.

| Command/filter | Result |
| --- | --- |
| `cargo test --locked -p oteryn-gameplay-dispatch --all-features nx5_ -- --nocapture` | 6 passed, including four live PostgreSQL cases; 61.71 s |
| `cargo test --locked -p oteryn-gameplay-dispatch --all-features novelty_ -- --nocapture` | 21 live PostgreSQL cases passed; 321.33 s |
| `cargo test --locked -p oteryn-gameplay-dispatch --all-features item_cosmetics_ -- --nocapture` | 8 live PostgreSQL cases passed; 108.33 s |
| `cargo test --locked -p oteryn-client --lib novelty_server_effects -- --nocapture` | native world projection play test passed; 0.01 s |
| `cargo test --locked -p oteryn-client --lib nx5_item_unlock -- --nocapture` | native appearance play test passed; 0.02 s |
| `cargo test --locked -p oteryn-gameplay-content cosmetic -- --nocapture` | 23 passed; 21.20 s |
| `cargo test --locked -p oteryn-session --lib` | 169 passed; 21.09 s |
| `cargo test --locked -p oteryn-protocol-oteryn -p oteryn-client-runtime -p oteryn-client-ui --no-fail-fast` | all selected targets passed; library suites 706 / 30 / 260 |
| Python cosmetics authoring discovery (`test_*.py`) | 25 passed; includes descriptor size and unchanged maintained-source checks |

The three PostgreSQL logs contain 33 completion markers and no no-admin skips.
The new file-size regression was observed red at 4,045,715 bytes before changing
the producer. Its output is now a 408-byte descriptor. The loader regression
checks the producer's exact section digest, bad source/section pins, duplicate
members and shard path traversal. The source authoring file stays 196,591 bytes.
The old index-negative test now requires the actual `POPULATED` state while
retaining its exact `MalformedDocument` rejection and all 252 identity-only Mount
record assertions. No test was skipped or weakened.

Self-review covered every changed path: removed-owner references, source pin and
section digest binding, bounded local reads and aggregate size, duplicate JSON,
source shard locators, unchanged strict cosmetics admission, session fences,
receipt replay, private presentation visibility and native decoding/projection.
No blocking finding remains. Broader S06 population, casting and manual GUI play
remain outside this task. No commit or push is made.
