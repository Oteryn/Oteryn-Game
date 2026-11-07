# Questpc contract decisions

- StepOut observes the committed movement's original tile and settles before
  destination StepIn. It is currently restricted to stateless map restoration;
  storage, offerings, experience, item-on-target and self-teleport are rejected
  by both Python and Rust. Existing occupied-wall refusal remains in force.
- A cancelled static departure with an exact matching pending record and a
  strictly newer source-object revision is superseded. Retire that event, keep
  the newer map state, and resume destination entry using the original movement
  snapshots. Never discard a durable callback or exhausted revision. PG RED/GREEN
  covers session recovery, destination feedback and unchanged tracks/receipts.
- This slice covers ordinary committed paced player movement. Source quest
  compositions and other relocation producers require their own qualification;
  no whole title was promoted by this engine test.
- Reused StepOut implementation from questpc5@2d9db9c5abdb62abc8f9883984b5ad5bc22244c8
  after checking qengine2@5a6de28c142de185594217ce1e5cbc7f4b6e6e4e.
- Next source facts: Devil Helmet creates/removes435 at32482,32170,z14;
  Mintwallin removes its lever, creates two1270 walls and removes nine1271 walls.
  Demon Helmet requires two player plates and ejects the top creature before
  recreating missing walls. Position:hasCreature calls getTopCreature and then
  teleportTo(destination,true); do not assume it moves every creature.

Sources:
- https://github.com/zimbadev/crystalserver/blob/9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d/data-global/scripts/quests/devil_helmet/movements_sewer.lua
- https://github.com/zimbadev/crystalserver/blob/9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d/data-global/scripts/quests/mintwallin_quest/actions_devil_helmet_lever.lua
- https://github.com/zimbadev/crystalserver/blob/9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d/data-global/scripts/quests/demon_helmet/movement-walls.lua
- https://github.com/zimbadev/crystalserver/blob/9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d/data/libs/functions/position.lua#L192

Limit each future investigation to ten minutes; record the conservative decision
here and continue. Preserve the atomic eleven-track Father's Burden write and
exact-match charge grants.

## Atomic source objects

- At most16 trigger/object effects; immutable pinned palette facts only. Unknown,
  ambiguous, pickupable, durable Ground/House objects refuse the entire batch.
- Stage a clone of the existing callback state, then publish removals, creations,
  trigger revision and collision together. No durable item or Quest writer.
- Create only when that appearance is absent; missing removals are no-ops. This
  conservative rule avoids duplicate static walls and matches conditional sewer
  creation. Mintwallin's consumed trigger still makes its normal branch one-shot.
- Floor-changing creation currently refuses. Add composed-floor resolution before
  enabling435; rendering a hole without descent is not an accepted fallback.
- Mintwallin's201189 source is byte-identical to retained9f5a72c6 (SHA25661944b37...).
  Bind its exact AID30008; no arbitrary source-coordinate alias.
  https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/mintwallin_quest/actions_devil_helmet_lever.lua

- The current generated floor catalogue does not contain435. Explicitly refuse
  that appearance until its source floor facts and composed routing are implemented;
  absence from the catalogue must not silently make a sewer visual-only.

## Mintwallin qualification

- Production selector remains only AID30008, exactly as the source. The controlled
  test lever at32592,32107,z14 is a fixture anchor, not a production position alias.
- All11 declared mutation targets plus consumed trigger execute atomically. The
  scene builder includes empty targettiles; a missing tile continues to refuse.
- Source packet count grows21->22 with Mintwallin. Exact named-input gate and
  corresponding expected-count tests grow together; no gate is weakened.
- Fresh questpc8@15b02adfd status for Mintwallin isUNFINISHED. This lane's source
  qualification does not certify a deployed complete World route or unrelated
  authored stage approximations.

## Devil Helmet: source fact correction

435 is a USE dropdown, not a passive floor-change. Crystal201189 items.xml marks
it primarytype=dropdowns; data/scripts/actions/items/ladder_up.lua registers435,
adds1 to the object position's z, checks PZ lock, and teleports the player. Keep
canonical floor_change UNKNOWN unchanged; do not invent a down flag. Implement
generic435 USE for static/dynamic objects and run departure after relocation so
UID25030 removes its temporary sewer. This supersedes the earlier floor-routing
assumption. Sources:
- https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data/items/items.xml
- https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data/scripts/actions/items/ladder_up.lua

The live map composition wraps pinned facts in LoopGroundFacts. That wrapper must
forward movement_items and movement_item; otherwise callback-created objects are
absent from the actual client view despite existing in direct tile composition.
Focused PostgreSQL created-grate candidate89.11s found this omission. The final
regression obtains its handle from the serving connection's domain17 snapshot.

Reusable dropdown integration reuses questpc6@3bfd85606's existing handler and
LoopGroundFacts forwarding. It retains PZ-lock refresh, owner/content/scope/reach
checks, sealed landing proof, standard stair cooldown and chat cleanup. This lane
adds exact command-fence validation and the existing ordered generic movement
callbacks after relocation. Focused PG90.436342034s proves static and created
world grates through real USE, lower-floor repeat refusal and no Quest receipts.
No new protocol capability or passive floor flag is introduced.

Devil Helmet source composition (201189f871d23597b82caf88fd611454a2ddefb8):
https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/devil_helmet/movements_sewer.lua
SHA25671542a6acba0cd100b6f2d86339473e86781c777fa10aa339a62ea02d54e1e84,
identical to the retained source-gap script. UID25030 creates435 at32482,32170,z14
only if absent; departure removes435. The controlled test plate at32481,32170,z14
is a fixture, never a production coordinate alias. Both runtime selectors are UID-only.

The five existing claim contracts match the exact pinned chest source:
https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/startup/tables/chest.lua
UID6139: four3029 at32459,32144,z15; UID6140: one3356 at32465,32148,z15;
UID6141: one3269 at32466,32148,z15. UID5017 supplies2969/key3610 at32589,32100,z14;
UID5016 supplies2969/key3667 at32576,32216,z15. Existing canonical claim selectors,
positions, item IDs, quantities and key bindings were independently checked against
these source rows. The original misspellingdevilhemet1/2/3 remains the claim identity.

Draconia escape support reuses questpc6 commitsc651c87d5 andbe812420e. A returning
STEP may name an explicit failure destination; the existing sealed movement owner
validates its landing. If that landing is unavailable, retain the committed entry
position and grant no Quest state or receipt; do not invent another destination.
Absent failure_teleport preserves the old return-to-origin behavior.

SET_IF compares the exact locked flag, writes its authored value only on equality,
and retains every other flag value. It remains receipt-bearing and binds equality
and replacement literals in definition-hash tag6; old effect encodings are unchanged.
Pure read-only guards and native rules cannot be combined with set_if. This matches
Draconia escape sourceDragonfetish1->0 while preserving absent-1 and later2:
https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/draconia/movement-escape.lua
Five focused PG cases102.11s prove safe/blocked landings, replay, exact reset, absent
and later preservation. The final source escape/offering composition follows next.

Draconia escape35012 now reuses questpc6@3bfd85606's reviewed native Item3206
profile and authored callback. Existing canonical-input hashes match exactly;
pinned items.xml confirms dragonfetish weight490. Only3206 is newly admitted,
NonStackable with no container, charges or lifetime. Existing Native records and
callbacks remain byte-for-byte equal as parsed objects, apart from the new escape.

Missing offering teleports to32818,31599,z9 without progress/receipt. One3206
at32816,31601,z9 is consumed atomically with the exact1->0 flag reset (other
values retained), then teleport to32701,31639,z6. Fixture flag acquisition uses
the fenced Quest writer, never an NPC qualification claim; the approach tile is
controlled test geometry, never a production UID coordinate alias.

The callback registry's changed proof hash required a transcription receipt
refresh and dependent Quest-tree regeneration. Approved graphs/core digests and
immutable/compiler inputs remain unchanged. All28 Quest changes are provenance
sha256 leaves; no quest values/rules changed. Native admission is separate from
this transcription receipt, whose native_runtime_admission remainsfalse.

Draconia exit35011 exact source:
https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/draconia/movement-exit_teleport.lua
SHA25680af40ac459eb655670472a0391db78590d456827917d4a64e714947b64723a4,
matching the retained source-gap blob. Four positions32802..32805,31584,z1 must
contain2772,2773,2772,2773 respectively. Success goes32701,31639,z6; failure goes
32803,31587,z1. The trigger is UID35011 only. Fixture32803,31586,z1 is controlled
test geometry and is never added as a production selector alias.

Generic object_conditions are bounded by the existing8-effect bound and require
stateless callbacks: no storage, offering, portable item target, XP or native reward
rule. They read one current pinned callback/door snapshot under the runtime owner;
there is no await between this read-set and the map/movement commit. Only current
immutable Channel Base/Added objects qualify. House/value/portable ground objects
are outside this read-set; absent tiles, unreadable facts and missing appearances
fail the condition. This conservative rule avoids treating unowned custody as a
physical world predicate. Existing callbacks default to no conditions.

Source-world native bindings (Crystal9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d):
https://github.com/zimbadev/crystalserver/blob/9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d/data-global/scripts/quests/thais_quest/actions_life_ring_lever.lua
https://github.com/zimbadev/crystalserver/blob/9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d/data-global/scripts/quests/mintwallin_quest/actions_devil_helmet_lever.lua
https://github.com/zimbadev/crystalserver/blob/9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d/data-global/scripts/quests/hidden_threats/movement-chests_check.lua
The normalized source-world-objects custody packet records script hashes, native
map roots and reward facts; its generator verifies the retained corpus without
copying donor files. Life Ring's exact native lever toggles six4598 grounds to5770
and restores them. Closing uses the existing sole-current-actor authority for
source getTopCreature semantics; ambiguous stacked occupants refuse atomically.
Every actor proof and the map projection commit together, with rollback on failure.
The native bag contains3052 and3085. Source200 charges exactly matches the pinned
necklace definition200; fresh full charges have no timed override row.

Hidden Threats source departures require both exact personal once-claim receipts
and line2, then set3. The retained source transition and definition hash remain
unchanged; generic Computed evaluation remains unsupported. Only the verified
source SET is bound temporarily under those predicates. Progress-only departure
recovery has no map projection and cannot be pinned by another player's later
shared origin revision; the real PostgreSQL regression exercises that interruption.
This component does not certify the remaining NPC/forge/rescue/room storyline.

The initial source-world batch restored the reviewed callback base while moving
three concurrent additions to a generated shard. Incoming69ad45e6e subsequently
refreshes its source-fix proof for the complete base, including those additions
and Draconia. Integration preserves that exact approved base and removes the
now-redundant intermediate shard; no callback is lost. The native source-world
shard remains loaded with cross-shard uniqueness and the combined runtime cap.
No source-fix approval or guard is weakened during this integration.

Map-authored key materialization follows the pinned Crystal key Item types2967-2973
(and only those exact native Item definitions in the map wrapper). Source XML:
https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data/items/items.xml
Each declares type=key. Its source action number is the immutable carried key
number, matching key-door comparison:
https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data/scripts/actions/doors/key_door.lua
Retain base-digest/placement/channel/reset-epoch stock, not personal quest claims.
Only one nonstackable noncontainer key may be bound; unrelated action/UID/door,
house, contents, text, charges and teleport restrictions remain. The durable
writer accepts a trusted binding, includes it in typed evidence and intent, and
writes the immutable key row atomically with its map MINT receipt. Existing
unbound intent bytes stay unchanged. This owner-authorized feature amends the
older reward-claim-only restriction; it does not infer any quest key placement.
Refreshed qengine2 and questpc6 have no map key binding implementation to reuse.

Shared source-key wrapper uses only the seven standard key types2967-2973. The
source also has a specialized wooden cage key31379; that item retains its own
interaction owner and does not gain the standard door-key exception. Key actions
are exempted only after the exact existing pickup wrapper admits the placement: a
nonzero number and one item, no other binding, no house, nested contents, charges,
text or teleport. The original source attributes remain available to owners.

Draconia route facts were retrieved from
https://www.tibiawiki.com.br/Draconia_Quest?action=raw
after Fandom returned402. The route explicitly requires two players and keys
that replenish at server save. One player holds the southern switch to admit
the other to a second switch; holding the second admits the first to the portal
leading to Key3007. Conservative rule for the missing complete switch script:
occupancy is Channel-local and current; each gate is open only while its own
plate has an occupant, closes on departure, and uses no personal quest claim.
The native Draconia map has standard silver-key objects with source action
attributes absent. Those need explicit reviewed/pinned source-number bindings
in the next composition slice; an unbound inventory key is insufficient. This
research stopped after the source/wiki/native comparison, without inventing
production selector aliases or treating personal rewards as shared stock.

Primary-source refinement for native Draconia stock: the pinned KeyDoorAction
startup table explicitly lists each key object and its door, and Storage.Quest.Key
resolves IDs3001-3008 to those exact numbers. This supersedes the earlier room-
based wiki inference for the five existing native stock objects3003-3007. Sources:
https://github.com/zimbadev/crystalserver/blob/9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d/data-global/startup/tables/door_key.lua
https://github.com/zimbadev/crystalserver/blob/9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d/data-global/lib/core/storages.lua
The new producer reuses questpc6's source custody/native-map pin pattern, verifies
both source SHA256/git blob identity and the exact unique non-House native key,
and emits only those five source action bindings. No binary map is rewritten.
The runtime binder reuses questpc6@f91534062's source_bindings owner, restricted
to this slice's capture; exact position/item/floor/house checks and conflicting
native attributes remain authoritative. Effective startup attributes must pass
the same closed pickup eligibility wrapper, including derived unique/action
refusals, so a stale handle cannot materialize an entry the binder seals.

First native key-door access slice reuses questpc6 commit8aacc18bb's framed M06
source-door scenario and the same startup binder. The primary table gives doors
3001=(32814,31597,7),3002=(32796,31597,7),3003=(32811,31597,7), and3012 at
(32675,31649,10)/(32676,31671,10). All five unique non-House native roots are
locked door5097; there are no fixture-injected action numbers. Source capture
checks SHA256/git blob custody, native region hashes and exact source/native shape.
The binder combines stock and door captures with one global bound/ambiguity check.

Native5098/5099 profiles already exist. Only5097 is added to the existing key-door
profile importer/admitter. Physics/classification come from the pinned Canary
items.xml and appearances.dat at99902524e052f37574194466c2949c576e4ab269, verified
against canary-ground-proof.json (XMLbf773d2c..., appearances17a72b30...). Sources:
https://github.com/opentibiabr/canary/blob/99902524e052f37574194466c2949c576e4ab269/data/items/items.xml
https://github.com/opentibiabr/canary/blob/99902524e052f37574194466c2949c576e4ab269/data/items/appearances.dat
The importer confirms5097/5098 DOOR blocks movement/projectiles;5099 is open.
Raw source assets remain temporary and are not committed. Existing profile records
and stock capture stay unchanged; no binary map or native graph admission edit.

Hidden Threats NPC programme uses the internally consistent Canary revision
04b83b512114bfd888000d6e1433ed8ecaec7c5b. Ten exact NPC source hashes and fresh
URLs are retained in samples/native-rewards/hidden-threats-npcs.json; the generator
verifies the unchanged donor corpus and reuses the existing literal reader.
https://github.com/opentibiabr/canary/blob/04b83b512114bfd888000d6e1433ed8ecaec7c5b/data-otservbr-global/npc/corym_servant.lua
https://github.com/opentibiabr/canary/blob/04b83b512114bfd888000d6e1433ed8ecaec7c5b/data-otservbr-global/npc/corym_butler.lua

Independent source programmes may share a canonical aggregate through an explicit
program_key. Hidden Threats activates only the Corym programme for Tibia Tales;
the older Cursed Crystal capture stays intact and outside this activation scope.
Absent programme keys are omitted from canonical serialization, preserving old
programme digests. All original track/transition compatibility checks remain.

Conservative native projections are explicit: source -1 and inherited absent0
are both unrescued flags; every rescue sets1 once. Counter0..7 must already be
initialized before a rescue can grant anything. Ratter repeats open the existing
mine and retain later quest-line progress. Item effects and quest writes are
atomic; a full backpack or later failure cannot lose materials or rewards.

The reusable NPC item participant resolves up to eight plain input types across
carried stacks before mutation and grants at most one reward type per branch.
It reuses qengine2e45e8f8a9's carried consumption and deferred source-proof design
and the existing once-claim MINT/audit owner. Exact programme track changes,
input totals and fresh reward quantity must match their same-transaction quest
and NPC receipts. A sealed NPC capability is constructed after equipment fence
admission; world callbacks still require their spell capability, and the NPC
capability cannot authorize ground-item mutation. Father and Marlin retain their
existing value owners; Father's acceptance remains one eleven-track write.
Immediate source end-conversation effects clear only the selected exact NPC
engagement. Recorded response flags support replay without inventing a protocol
capability or leaving inventory views stale.

Corym production conversation admission requires a literal opener in addition to
stateful native programme rules. A separate closed ten-NPC greeting shard pins
both exact NPC hashes and the source handler's greeting/tag defaults; the older
23-NPC greeting contract remains intact. Production-book construction is tested.
All ten controlled-fixture actors are installed once in a separate walkable row;
subsequent dialogue moves the character to each actor. Runtime NPC placement
immutability is preserved, including the earlier failed fixture evidence.

Upper key-door slice: exact native source selectors3004-3008 are retained from
pinned door_key.lua; no proximity aliases. Five real PG access cases green.
First key3001 is declared inside skeleton5972 in container_misc.lua at32794,31572,z7.
Source loads container contents before recursively applying key actions; stock is
filled only when the source container is empty. Primary source:
https://github.com/zimbadev/crystalserver/blob/9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d/data-global/startup/tables/container_misc.lua

Hidden Threats forging keeps the original CRAFT-1 source snapshot frozen and
adds only two exact Canary04b83b512114bfd888000d6e1433ed8ecaec7c5b witnesses:
https://github.com/opentibiabr/canary/blob/04b83b512114bfd888000d6e1433ed8ecaec7c5b/data-otservbr-global/scripts/quests/hidden_threats/action-key_forge.lua
https://github.com/opentibiabr/canary/blob/04b83b512114bfd888000d6e1433ed8ecaec7c5b/data-otservbr-global/scripts/actions/tools/metal_file.lua
The normalized capsule is craft-authoring/samples/hidden-threats-forge.json;
its digest is pinned before emission. The original129 recipe rows retain their
exact canonical SHA2564929f6b9c229b2c5099d17e16f04eca94c39af12ade0ea4c0dbd751791a12c1f.
Sixteen new fixed recipes cover two filing operations, eight heating operations,
four source-allowed hot-half pairs, hammering and quenching. Equal-kind hot halves
are source-allowed; the native owner requires distinct plain carried instances.

The unchanged file uses the existing craft tool participant. The consumed source
is the target material; its replacement uses the existing BURN/MINT proof and
freed backpack slot. File identity and state remain intact. This uses the existing
main-backpack craft authority; unsupported custody fails without mutation. No
new SQL bypass or approximate consumption rule is introduced. An explicit fixed
water-station recipe precedes the fishing fallback; rods retain the fishing owner.

New Item policies reuse the canonical pinned importer and binding table:
https://github.com/opentibiabr/canary/blob/99902524e052f37574194466c2949c576e4ab269/data/items/items.xml
https://github.com/opentibiabr/canary/blob/99902524e052f37574194466c2949c576e4ab269/data/items/appearances.dat
Both originals were freshly fetched into temporary storage and matched existing
canary-ground-proof.json hashes. Only normalized required policies are retained.
The four lava targets, heated halves and file have no canonical temporal/charge
state. The source lava decay call has no authored transition; native behavior
therefore invents neither cooling nor charge state. Target10113 is water.

The post-write failure case identified a generic craft result-classification
bug: converting Result with ok() discarded explicit PostgreSQL CommitRejected
as an unavailable outcome. Craft dispatch now returns a settled Rejected only
for that known rollback. CommitOutcomeUnknown and unavailable results retain
fail-closed reconciliation. The fault test requires an explicit rejection,
executed deferred failure trigger, complete item/receipt rollback and successful
same-owner retry; no transport or ownership guard is relaxed.

Latest reuse audit before Corym door support: qengine2 `a0100244c6f7a72573630aebd19acaff21a383fa` has25QUALIFIED,17 overlap here (exact list in QUEST-STATUS). Its new reward exchanges/target sets do not provide retained world-item tools or additive world-callback catalogue compatibility; no such implementation to merge. Existing local fenced callback/item owners remain the reuse base.

Draconia native callback anchors use pinned startup LeverUnique30006/30035 and
TeleportUnique35011/35012, with exact source position/item and no proximity aliases.
Native roots have no attributes and no house. Existing bounded binder preserves
native attributes and rejects ambiguous capture unions. Source capture verifies
both SHA256/git-blob custody and actual native region hashes:
https://github.com/zimbadev/crystalserver/blob/9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d/data-global/startup/tables/lever.lua
https://github.com/zimbadev/crystalserver/blob/9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d/data-global/startup/tables/teleport.lua
Fresh owner-lane scan finds34 distinct locally listed titles QUALIFIED elsewhere;
27 remain locally UNFINISHED but are explicitly excluded from implementation here.
Local counts are not promoted by an unmerged external implementation.

Draconia route provenance checkpoint: three primary files at Crystal201189f are
byte-identical to retained9f5a72 corpus blobs (pressure handler, teleport handler,
plate table). The teleport table differs elsewhere; its four owned UID38003-38006
rows are semantically identical, including destination/effect. Hashes, URLs and
line mappings are retained in cloud-draconia-route-source-validation.json.
Pressure source is player-only remove-if-present on entry, create-if-absent on exit;
no personal claim or additional party counter is substituted. Primary URLs:
https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/movements/others/remove-create_item.lua
https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/movements/others/teleport.lua
This checkpoint publishes provenance only; route implementation remains uncommitted.

Route verification correction: portal38003 alone already retains its exact native
teleport attribute. Portal38004 and the two internal portals lack startup selectors.
Preserve38003 as a native regression; restore five anchors/seven callbacks only.
StepOut map effects require the existing reversible flag; authored departures now
use it without relaxing validation. Pin compiler and dispatch test rustc must run
serially in the16GiB environment after an observed OOM kill. Pre-PG registry/setup/
compile failures are explicitly excluded from PostgreSQL proof counts.

Draconia route slice GREEN: two source pressure plates use existing player-only
StepIn/StepOut object effects; the existing reversible-map tag admits departures
without storage/reward guard changes. Guest has real admission/lease and initialized
pinned position. Native38003 keeps its teleport; bundle floor byte is signed native
i8 reinterpreted asu8, distinct from B3 legacy z. Five new source PG cases and the
native regression pass separately; initial fixture/registry/OOM failures are not PG
proof. Final strict checks pass; no full-suite run or whole-title promotion.

### Draconia Dragonlord greeting exchange (2026-11-07)
Primary: https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/an_old_dragon_lord.lua#L59-L78
SHA256: 85016cc08856f49d94c507f43bc84f54801d3a2844946ae391015a0d8c84a052.
The offline corpus contains byte-identical source (Git blob
1edc28f6c6bbbb9e5ea2ef74b836151a8a21ead4), verified by the producer.
Only exact Dragonfetish value 1 refuses another mushroom exchange. A qualifying
hi/hello consumes one white mushroom3723 and grants dragonfetish3206 together
with flag1. Every source greeting exits without retaining focus. Other admitted
greeting words receive the source's LEAVE response. No new item definitions or
NPC-specific transactional owner are necessary. The reviewed agreeing native
placement is32796,31558,z2; PG transport uses a controlled scene separately.
The original frozen NPC programme and greeting cohorts retain their boundaries.

### Draconia native level25 gate (2026-11-07)
Primary selector: https://github.com/zimbadev/crystalserver/blob/9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d/data-global/startup/tables/door_level.lua#L16-L23
SHA256 d8868f675aae7c541ddc3e76b3648b6a886e0ae31a4a614747d17e1784d234fc;
Git blob0d1a38cf3b0309d81ce8a08c6fc495df74c86444. Its action1025 group selects
native door5102 at32804,31583,z2. The other source level25 location remains outside
this capture. The existing generic level-door owner interprets action minus1000,
reads canonical actor facts under the owner lock, and refuses key bypass.
Closed/open profiles5102/5103 reuse the pinned native item importer:
https://github.com/opentibiabr/canary/blob/99902524e052f37574194466c2949c576e4ab269/data/items/items.xml
XML SHA256 bf773d2c8f1d2767b31a20749fd5278ae48f2ed5b05757a678e3788e2a70bf85;
appearances SHA25617a72b30b5c3c9ca8c1283cfb2febd2a93a145ff8ab66916f7a412d0f1dee5a1.
Source-derived closed physics block movement/projectiles; open physics block neither.
The focused PG scene supplies only initial canonical actor levels24/25/26 and
unbound source door identity/position; the production source binder and wire owner
must establish the action and evaluate the predicate. It does not certify XP gains.
