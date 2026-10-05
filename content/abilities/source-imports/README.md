# Current spell source import

This directory imports supported donor data into the spell and selected monster
melee models already available on Oteryn `main` (`b61e6d18`). It contains reviewed
source receipts and operational missing-data flags, without Lua files, migrations,
historical package copies or coverage reports. The accompanying runtime patch
connects the existing Wheel owner to current cast access facts.

| Source | Branch | Pinned commit | License |
|---|---|---|---|
| [Canary](https://github.com/opentibiabr/canary) | `main` | `04b83b512114bfd888000d6e1433ed8ecaec7c5b` | [GPL-2.0](LICENSE-canary.txt) |
| [Crystal Server](https://github.com/zimbadev/crystalserver) | `main` | `96d13eff5a1afef17b11b9abc7d574020381cc53` | [GPL-2.0](LICENSE-crystalserver-main.txt) |
| [Crystal Server](https://github.com/zimbadev/crystalserver) | `summer-update` | `00ce02a57ca5a12e48f32a3476e37471167e4c3f` | [GPL-2.0](LICENSE-crystalserver-summer.txt) |

License texts are copied unchanged from each pinned upstream `LICENSE`, following
the notices used in #1757. Donor-derived records retain that upstream license.
Existing official/wiki provenance in the accepted catalogue is preserved; the
import makes no new wiki-verification claim.

`current-sources.json` pins explicit target identities, source-file hashes and the
base gameplay-payload hashes used for projection. Its `qualification` entries
identify accepted fields retained during normalization; an accepted official/wiki
value is not attributed to a disagreeing donor. The importer pins the reviewed
bytes of the complete receipt. Changing a source, target or file hash requires
fresh source/model qualification and review of the new pin; schema validity alone
cannot establish donor provenance. Regeneration requires no upstream checkout or
network access.

The player-source batch contains 368 bindings: 293 `identity_only` projections and
75 `ordinary_data` imports. Of the ordinary imports, 45 qualify an
`accepted_ordinary_model`; 30 import only `canonical_base` data and retain their
missing-mechanic records. The partial imports comprise six Buzz/Scorch variants
and six Crystal damage-over-time variants with optional elemental stance
controllers still held, plus 18 Crystal equipment/healing BASE reviews. The latter
retain the complete immutable accepted payload, including Canary audio and accepted
Formula values. The differing donor AST, missing Crystal audio, secondary routes
and controller lifecycle remain explicit conflicts or partial scopes. Across bindings and holds, all 746 unique inventory
registration keys are retained, including excluded source inputs. These are donor
variants, not counts of distinct playable spells.

Fifteen identity-only bindings qualify the existing five C.3 party buffs. Their
accepted payloads stay unchanged; the complete donor parameters and the explicit
C.3 selection, mana, condition and lifecycle differences remain in the receipt
and in scope-specific source holds. This does not claim full donor parity.

Thirty-six additional identity-only bindings qualify unchanged closed native
profiles through the existing native authoring helper. Their complete donor models
and reproduced receipts remain present, with normalized controller equality and
raw/whole-controller parity false. S21 Familiar aliases and the four barrier
projections use their existing explicit selection and Effect proofs. All 36
retain source-scope holds; none changes a native gameplay payload.

`unavailable-spells.jsonl` retains 486 records using the five classes from
SPELL-NPC-MAP-0 §1: 249 `SOURCE_CONFLICT_KEPT`, 155 `ADAPTER_MISSING`, 27
`RUNTIME_GAP`, 38 `EXCLUDED` and 17 `CONTRACT_GAP`. There are 108 registrations in
both the binding and held sets: 30 partial base models, 15 party projections,
36 closed native projections and 27 accepted field-rune models whose PvP creation
context remains held. Adding the binding and held counts
does not give the inventory size. Sixteen contract-gap records cover eight additional Crystal state spells. Their
source parameters are known, but the accepted C.4 shape only defines the existing
standard stances and virtues. Source hashes and concrete missing slots/modifiers
remain in each record; execution of these additional mechanics remains held.

A held variant does not disable an accepted
Oteryn spell with the same name. Excluded inputs include non-player helpers and
disabled examples.

The adapter checks the pinned base catalogue, preserves its 246 spell identities,
67 closed native profiles and alias selection, then rebuilds supported dependency
collections and provider hashes. Models qualify against the accepted normalized
Oteryn contract, not raw Lua or complete controller parity. S5's world level curve
and S24's official/wiki precedence remain in force. Source disagreements in costs,
requirements, targeting or formulas stay explicit rather than silently replacing
accepted values. Optional controllers outside a partial base scope remain held.

Ordinary imports update payloads for 20 existing spell identities. This includes
21 donor bindings for seven damage-over-time spell identities: their conditions
and presentation are decomposed into existing Effect roles without dropping
data, with `zero_damage_health_path=true` retained on the Ability. Source proofs
preserve presentation before the health path and condition application only after
the health path accepts. These records require actual operational facts; importing
them does not establish that every cast context is connected.

The field-rune models support field creation when the target tile has `NoPvP` or
the world is `FieldWorldType::NoPvp`. In a PvP world, creation on a target tile
without `NoPvP` remains rejected. Field creation now rechecks the physical
primary-cost transaction, original command, current Character/session/lease/
connection/Content and runtime scope, plus an independently present positioned
caster with no control loss. World and tile NoPvP policy come from current
qualified owner reads. ATTACK-1b already holds disconnected actors through the
current attack owner's in-fight deadline before grace expiry and terminal
release. The 27 scope-specific holds remain because ordinary field creation and
periodic field hits do not publish qualified combat-lock events to that owner,
and FIELD-1 §11.3 excludes player-affecting PvP. These integrations and PvP rules
remain incomplete. Historical field receipts cannot extend original durable
grace, recreate Character leases or grant reconnect authority under DUR-02 §12.

The receipt's `monster_melee` section refreshes the donor revision for 1,253
existing selected melee records and adds 98 source bindings. This supplies the
current primary physical melee profile for 1,351 of 1,508 selected
Creature profiles. It does not import a general monster spell dispatcher.
`unavailable-monster-melee.jsonl` retains 250 operational flags: 157 concern the
selected melee scope and 93 preserve health disagreements for creatures whose
independent melee binding is already imported. Accepted Creature stats remain
unchanged. Of the 157, 106 are excluded from harmful selected melee and 51 retain
non-exclusion holds. Secondary conditions remain omitted from 166 imported primary
melee records. Historical `approximate_nonlethal_physical_melee` labels are retained
for compatibility; current melee health handling allows a lethal result. The
separate `scope_flags` retain the missing general AI execution bridge for ranged
attacks, other melee slots and special controllers. The scheduler and existing
familiar self-heal defenses are already present; a selected melee import does
not complete the other attack/defense slots. The nearest-player AI candidate
list now excludes actual dead or HP-zero states while retaining current actor/
session, pending-commit and reservation checks. This fixes dead targets masking
a living target; it does not add general ranged/special monster spell dispatch.

Regenerate through the existing content pipeline:

```sh
python tools/content-migration/register_spell_families.py
python tools/content-migration/register_spell_families.py --check
RUST_MIN_STACK=16777216 cargo test --locked -p oteryn-game-server --test current_spell_sources
```

`import_spell_families.py` still verifies the immutable r25 foundation. The current
source adapter is applied by `register_spell_families.py`, so regeneration retains
this import without modifying the historical pack. Source receipts keep
`runtime_activation=false`; regeneration and loader qualification do not activate
a server generation.

The Wheel bridge reads the existing allocation under the cast's physical
Character/Item transaction and binds it to the current actor, session, lease,
equipment, CharacterRevision, Content and Premium expiry. The content-pinned
Wheel numeric profile consumes the complete green/red/purple/blue source stages
and supplies the accepted flat damage/healing bonuses (4, 9 or 20 per domain stage)
to the existing source-ordered magnitude finishers. Known zero requires a genuine
current zero allocation; missing or expired evidence remains unknown. This does
not implement the remaining advanced Wheel modifier stages or actor-state
controllers, and does not establish every cast context or activate a server
generation. The source holds describe the remaining model/controller differences;
they no longer claim that the current base Wheel reader is absent.
