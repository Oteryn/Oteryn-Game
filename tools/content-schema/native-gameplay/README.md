# Native gameplay artifact candidate

This is the explicit local candidate `OTNGP01` server envelope, not an amendment to
`FIRST_PRODUCTION_CONTENT_PROFILE/v1`. Baseline record cardinalities and limits stay
unchanged. The envelope retains the complete qualified baseline server artifact and the
same client projection. The existing activation controller, sealed node-boot guard,
quiescence checks, scope issuance and monotonic activation sequence remain required.

Set `OTERYN_NATIVE_GAMEPLAY_MANIFEST` to a manifest pathname to opt in. It is the single
selector for the node and for `oteryn-game-ops content activate`, so the issued digests are
computed over the room the node boots. The canonical book is `content/spells.manifest.json`;
see `content/abilities/SPELL-IMPORT.md` for the activation steps. Without that
variable the node loads the existing baseline. An empty pathname, missing input, malformed
manifest, wrong input hash or wrong independent scope issuance prevents readiness. The
node never obtains the catalogue from a global `include_bytes!` or enables this example
by default. Paths resolve relative to the manifest; absolute paths are also accepted.

The strict manifest discriminator is `OTERYN_NATIVE_GAMEPLAY_MANIFEST/v1`. All four
file pins (`catalog`, `source_selection`, `creature_profiles`, `presentation_profiles`)
are required; each has precisely `path` and lowercase hexadecimal `sha256`. The sample
[manifest.json](manifest.json) points at the existing complete 246-identity spell
catalogue and its explicit Canary/Crystal familiar source selections. It also includes
actual decoded Rat and Knight familiar projections from the captured monster bundles.
[profile-sources.json](profile-sources.json) identifies the complete source bundle hashes.
These monster sources are OTS hypotheses under their explicit older source revisions;
the example is not a declaration that all creatures or familiar species are present.

The full local source producer `build_full_spell_artifact.py` adds the real
Skeleton and five Familiar profiles. Its `--appearance-source` reads pinned
Canary Git blobs and generates Outfit selections for every illusionable Creature
in that complete input, including Skeleton look33. The producer independently
requires each exact Creature link to match its Presentation look. The small
`spell_appearances.json` fixture remains the Rat/Avatar provider for the smaller
example; the full generated provider is written to the output manifest directory.

The creature document schema is `OTERYN_NATIVE_CREATURE_PROFILES/v1`, with `records`.
Each record has the complete typed `ProjectV2AuthoringProfile` in `profile`, plus its
exact Presentation DefinitionRef in `presentation`. The presentation document schema
is `OTERYN_NATIVE_PRESENTATION_PROFILES/v1`, with complete Presentation authoring
profiles in `records`. Existing V2 validators qualify all retained authoring data.
Missing health, speed, details, exact presentation revision or outfit binding is an
explicit unsupported input; the loader never fills those values from a prototype.
The decoded artifact retains complete flags, source immunity names, familiar metadata,
resistances, bestiary and other supplied V2 details, as well as full presentation data.

Runtime companion policies use decoded maximum health, base speed, display name,
source identity/revision, summon/convince flags, mana cost and familiar classification.
Only the exact source grammar `canary.appearance:outfit/<positive-u32>` supplies an
outfit number. An explicit `Invisible` presentation with no asset supplies outfit zero.
The original source condition strings remain in the decoded details. The typed
speed/DOT policy subset maps Canary's documented immunity aliases to their exact
ConditionType; `invisible`/`invisibility` and `drunk` are retained source-only families,
not converted into immunities for unrelated conditions. Unknown names reject loading.

The binary layout is an eight-byte `OTNGP01\0` discriminator followed by five ordered
sections: baseline server artifact, executable spell catalogue, source selection,
creature profiles and presentation profiles. Every section has a big-endian u32 byte
length, a 32-byte SHA256, and exact section bytes. Trailing bytes and nested envelopes
reject loading. There are no invisible global defaults or profile lookups.

Candidate limits: 56 MiB outer server artifact; baseline server bounds unchanged;
32 MiB catalogue; 256 KiB source selections; 8 MiB each creature/presentation section;
4096 records per profile section; provisioning manifest 16 KiB. Reads enforce bounds
before allocating the full input, including file growth between metadata and open.
These are explicitly candidate limits and require the delivery owner's contract
registration before production admission. The baseline single-staged/two-resident
activation ceiling remains unchanged.

The outer SHA256 is the active server generation digest. Changing any payload, source
revision, profile or selection changes this pin even when inner section hashes are
correctly regenerated. Staging checks the independent expected outer digest before
decoding, then revalidates the baseline server/client pair, compiles the full spell
book and source policy, and builds the companion table under that same outer digest.
`ActiveGeneration` stores those compiled values. Node startup takes its spell book
from this active generation and installs the table into the actual ChannelRuntimeV1
before admission; installation compares its actual Channel Content pin. Movement
cells and spell tile metadata are reconstructed from the same qualified room under
the outer digest. A baseline control-plane issuance cannot authorize this envelope.

The compiler API takes caller-provided `NativeGameplayInput`. `NativeEntryProject::
qualify_gameplay_room` and `activate_native_entry_room_with_gameplay` expose the opt-in
qualification/activation seams; provisioning valid bytes does not issue authority.
The example intentionally does not change the control-plane issuance or activate a
running deployment.

## Versioned optional owner profiles

The explicit candidate manifest `/v2` requires a fifth file pin `item_profiles` and
uses `OTNGP02\0`, six binary sections and a 64 MiB outer limit. The item document is
`OTERYN_NATIVE_ITEM_PROFILES/v1` with records containing complete typed `authoring`
(`ProjectV2ItemAuthoring`), `semantics` (`ReferenceItemSemantics`), and `attributes`
(`speed_bonus`: signed integer or null). Both existing authoring and item semantic
validators run. Null is unknown. The loader never derives item attack, shield defense,
slots, required vocation or speed from an asset number or missing metadata.

Manifest `/v3` additionally requires `spell_appearances`; `OTNGP03\0` has seven
sections and a 72 MiB outer limit. Its qualified native Outfit definitions preserve
full colours, addons, mount and source proof. Creature links must match an active
Creature DefinitionRef and its source look type exactly. Numeric appearance membership
alone is insufficient. The source importer owns the explicit constructor defaults.

Manifest `/v4` additionally requires `build_training`, with `path`, `sha256`, and
`content_revision`; `OTNGP04\0` has eight sections and an 80 MiB outer limit. The
last section binds that explicit revision and complete training profile together.
The compiler qualifies source files and all eleven vocation tables with exact
binary32 source multiplier semantics. The runtime consumer must compare the formula
revision with the actual Character build revision before training. Neither compiler
nor loader substitutes the outer digest for the Character's authored revision.

The additional sections each have the same 8 MiB bound and independent section
hash. Versions form a required prefix; omitted newer providers stay unavailable.
The version-one example remains usable without enabling equipment, appearance or
training providers. These expanded caps and contracts are local candidates.

## Current cast access producer contract

`spell_access_facts::load_owned_cast_facts_in_transaction` reads class build, seven
skills, progression level and initialized equipment state from actual durable owners
in the compositor's same SQL transaction. It independently checks recovery, node,
session, connection, Character lease, Channel scope and actual active Content pin,
then compares the live player owner. Its immutable binding includes exact actor and
session, Character/player/equipment revisions, lease/connection generations and
Content digest. Rejected casts do not silently initialize equipment.

The registered `CurrentSpellAccessOwner` port is a candidate integration seam for
Platform Premium entitlement and Character learning/Wheel projections. A usable
projection must contain that complete binding, a positive upstream authority revision,
an exclusive expiry and its complete value. Platform must provide authenticated
account entitlement with its own durable revision and revocation/conflict fence;
Game cannot originate a commercial entitlement. Learning and Wheel producers need
actual Character grant/allocation receipts, revision fencing and complete current
sets. Only a complete current Wheel projection may interpret an absent perk as
zero. The default unavailable producer returns no projections, so Premium/learning/
Wheel-dependent spells and missing combat magnitude attributes refuse before RNG.
No Premium, learned spell, Wheel allocation, base critical rate or combat mode is
seeded by this package.

`0035_character_equipment.sql` and its transaction helper own actual slot custody.
A move compares the exact current equipment snapshot, resolves source-qualified
active item requirements/reservations, and atomically writes ItemInstance custody,
item revision, equipment revision, receipt and audit. Existing baseline branches
remain in the guards. Explicit combat-mode commands supply Canary factors 1, 0.75
or 0.5; an uninitialized mode remains unknown. Rune charge storage/consumption and
external access producers require their own qualified owners.

Creature records may also carry a complete `behavior` V2 authoring profile. Existing
validators qualify its family, typed movement, targeting, schedules and other retained
fields. AI policies read preferred distance only from that explicit profile; its
absence stays unknown. The two source-bundle examples retain complete Behavior data
with exact percent-to-ppm conversion and source speech modes, so Rat distance1 and
Knight familiar distance4 remain distinguishable. `reward_boss` comes directly from
the complete Creature system eligibility flags.

## V5 source configuration and separately pinned source-world input

The explicit `OTERYN_NATIVE_GAMEPLAY_MANIFEST/v5` candidate requires the complete
V4 prefix plus `familiar_config: {path, sha256}`. `OTNGP05\0` retains all V4 section
bytes and appends a strict supplements object as section nine (maximum 128 KiB). This
object retains exact UTF8 `familiar_config` bytes, required nullable `wheel_profile`
bytes and the explicit `native_map_profile` selector. The source configuration
itself is bounded to 4096 bytes; the Wheel source profile to 64 KiB. Its artifact ceiling is 88 MiB. The compiler compares the complete decoded
configuration and source proof; changing the familiar duration, cooldown rate or
VIP configuration requires new source qualification and independent Content issuance.
V1–V4 do not gain configuration defaults when that section is absent.
The manifest may explicitly pin `wheel_profile` only under `/v5`; null or omitted
means Unknown, never a global source-table fallback. A nondefault
`native_map_profile: "source-qualified-spell-entry-r2"` requires V5 and preserves
the original four-cell geometry with separately qualified source Ground metadata.
Its default is `"accepted-entry-r1"`. Selecting profile2 together with `source_world`
is rejected, since the source-world input selects the independent profile3.

A manifest may explicitly supply `source_world: {path, sha256}`. This input is
bounded to 8 MiB and its exact SHA256 is checked before qualification. The separate
source-world compositor consumes those exact bytes and produces the server/client
outer artifact pair; it binds the source map, client projection and native gameplay
under one independent outer issuance. It is not an unqualified in-memory map switch.
The existing entry map remains selected when the pin is absent. The actual stage
rebinds all immutable creature, appearance and familiar configuration tables to the
outer artifact digest before the runtime can install them.

Native Item records additionally require `production_binding` containing the exact
already admitted Crystal Item identity row and `production_definition` containing
its actual canonical target (`family`, `production_key`, `revision_ref`). The source
alias and canonical revision are both preserved; the runtime performs no numeric
identity promotion. A source ID lookup uses the embedded exact external ID. Field
metadata is optional `attributes.field_condition`: missing means Unknown; only a
complete recipe matching the immutable pinned XML/parser/condition source may assert
a damaging element or a known non-damaging barrier.

## Character Wheel owner candidate

Main's WHEEL-W1 owner (`character_wheel.rs`, migration 0070) is the only Character
Wheel owner; this profile declares no owner tables or admission seeding. Runtime Wheel
effects stay unadmitted until SPELL-WHEEL-GATE-1, so the spell side reads no Wheel
stages and Wheel-gated spells are refused.

The active caller-supplied source profile retains seven exact Canary Git blobs,
36 slot capacities/full-neighbour rules, priority passes and source revelation
thresholds/statistics. Allocations require current authenticated Premium, level51,
a promoted vocation and source-owned available points. Source-equivalent successful
assignments are staged entirely before mutation. An unsupported gem modifier or
missing Monk shrine bonus owner refuses assignment. Monk spell projections stay
unavailable until the separate shrine owner can qualify them; the adapter does not
turn absence into a false unlock. Other source bonus stages (slot skill modifiers,
charms, proficiency and account/VIP roles) are independent magnitude/access inputs.
