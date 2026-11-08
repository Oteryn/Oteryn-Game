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

## Active Corym WorksDoor source contract — 2026-11-07

Canonical source is Canary04b83b512114bfd888000d6e1433ed8ecaec7c5b:
https://github.com/opentibiabr/canary/blob/04b83b512114bfd888000d6e1433ed8ecaec7c5b/data-otservbr-global/scripts/quests/hidden_threats/action-corym_works_doors.lua
Fresh bytes match263b50458d596e4900dfa491997f184ef6c88879034a61c52aae42c983f42184
(30lines/1108bytes). The unprefixed temporary file is older Crystal source;
use the canonical/fresh copy. Canonical matches storage IDs45974/75/76 and
returns true even on repeat; only a negative flag becomes1 and emits success text.
The key27269 is retained unchanged. Action/position select the native placement
across its existing open/closed appearances; source does not guard appearance.

Registration source:
https://github.com/opentibiabr/canary/blob/04b83b512114bfd888000d6e1433ed8ecaec7c5b/data-otservbr-global/startup/tables/door_quest.lua
SHA bfcfdf9c66ae540922f5e05aace80106d1f1e2945b1f06ad4a35bf7da14191d1.
Storage source:
https://github.com/opentibiabr/canary/blob/04b83b512114bfd888000d6e1433ed8ecaec7c5b/data-otservbr-global/lib/core/storages.lua
SHA05a6f2a9116e8a230d9cf6bc8e15fd306b1cca8b4520e80beb9d7a9809786d0b.
All3 native map roots and existing quest-door bindings match those canonical IDs.
No raw originals will be committed. A handled SET_IF repeat may retain the generic
Quest operation receipt; progress/key remain unchanged and success text requires
an actual flag change. This is internal receipt bookkeeping, not a source reward.

Reuse audit qengine2@93ee8fe847c65f030f47ce245105229fa63b61c0:27QUALIFIED,
18overlap skipped by ownership. Latest delta adds outfit reward-bag acquisition,
not retained world tools or additive catalogue compatibility. Reuse the local
fenced callback/tool presence locks and SET_IF owner. Runtime implementation waits
for actual PostgreSQL RED; compiler failures and memory-killed builds do not count.

### Retained tool and save compatibility implementation

The actual admitted carried ItemID is bound into the existing fenced callback
context and locked by the Root input participant. Retention leaves the item row
unchanged. The key route passes its held SpellLanePermit into the callback so
inventory proof and flag commit share one lane acquisition. Progress-only tools
leave door projection to the ordinary door owner, then refresh coherent quest
session state after runtime/revision locks are released; recovery uses the same
refresh. This makes the next normal door USE observe its committed access flag.

The bounded cohort loader reconstructs the actual NPC13 catalogue at layer0.
Historical fixtures create their immutable pin through that owner's real commit.
Exact additive history preserves all prior track/transition semantics and missing
track defaults. Explicit donor-track bindings reject collisions. Read-only gates
honor the same historical defaults as writers and NPC rules. Source-witness tests
cover the new bindings alongside the original action classes.

Actual PG18.6 all4cases pass194.57s: unknown definition48.022567361s, WorksDoor01
48.62128968s, WorksDoor02 48.642059144s, WorksDoor03 49.279037075s. The positives
cover old saved pins, actual key use, wrong-item/plain-carry refusal, unchanged key,
exact replay, source handled repeats, message conditions and immediate normal door
access. Final strict checks, NPC/key regressions and pins are still in progress.

### Elathriel exact paid Hellgate key (2026-11-07)
Primary: https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/elathriel.lua#L193-L238
SHA25624dbbbe1ee2bcb74f74462ce131dd10cdf45801d53ecb586bd63993e1450cd7d.
The verified offline corpus contains identical bytes. Key/keys asks5000, topic1;
exact normalized yes at topic1 removesMoneyBank5000 and grants copper2970 with
immutable action3012; success and insufficient funds reset topic0. No also resets0.
The source has no quest storage guard. Extend the reusable paid reward engine to
admit an exact source-bound stateless sale, with no invented progression flag.
The existing world/ordinary quest constructor must continue rejecting unbound empty
graphs; the durable writer must require the matching admitted fee/MINT participant.
Source-owned failure text is Come back when you have enough money. Existing
transaction-refusal protocol result is retained with source conversation text/topic.
Conservative native delivery rule: a full backpack refuses without charging, rather
than delivering a paid key outside the customer's carried custody. Reuse questpc6
paid enginee30938891/2123b7871 and SQL0999/1096; preserve local typed input permits,
Corym transactions, source world effects, and Father's atomic eleven-track write.
Reviewed native placement32684,31671,z9; source copper2970 profile already exists.

Elathriel follow-up: actual PG key sales correctly refused copper2970 because its
ordinary profile classified it UNKNOWN. Refine that existing profile from pinned
Canary99902524 item/appearance facts via the existing bounded key/door importer.
Preserve identity/admission and every known fact; reject contradictory facts.
Do not bypass the generic key-mint classification check. The source item is KEY.
The incoming qualified Corym commit9ff93aede supplies reusable additive catalogue
history; use it rather than a duplicate predecessor map. Source-history guards
still reject edits/removals of old tracks and transitions. Static/source validation
is separate from the pending final paid-sale PG qualification.

Stateless receipt follow-up: reuse QENGINE1003's bounded zero-effect receipt arm
with a deferred exact source+fee+MINT proof. General eight-effect and historical
completion arms remain intact; Father's separate atomic programme is unchanged.
Add a durability build watcher for the shared migrations directory: SQL-only
additions must refresh GAME_MIGRATOR. Existing embedded/filesystem migration
identity unit passes; actualPG now asserts1097 and rejects an unpaired zero-effect
receipt with23514 under the runtime role in all three probes.
Repurchases must retain the existing once-only MINT claim owner. Reuse QENGINE
1004 stable per-command exchange claim hashing and its SQL expected-claim proof,
extending the selector to positive repeatable monetary fees. Each new exact paid
CommandRef is a distinct claim; replay stays the same claim. Do not replace this
with a free repeatable quest reward or waive the MINT claim ledger.

### Shared source container stock (Draconia key3001)
Primary source: https://github.com/zimbadev/crystalserver/blob/9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d/data-global/startup/tables/container_misc.lua#L451-L458
and https://github.com/zimbadev/crystalserver/blob/9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d/data-global/startup/tables/door_key.lua#L9-L20 .
The empty native5972 root at32794,31572,z7 receives one silver2969 numbered3001.
Use shared world/channel/base-digest/reset-epoch MINT ownership, with no personal
quest claim. A reserved, otherwise absent base ordinal names the contained stock;
it is shown only inside its parent pane and never as a loose Ground key. Validate
parent identity, ordinal, emptiness, non-house status and reserved-slot collision
before admitting stock. Existing nonempty containers retain their native contents.
Bounded source-stock containers support withdrawal; accepting deposits is outside
this mechanic. Parent5972 capacity10 comes from the existing pinned Canary importer
at99902524e052f37574194466c2949c576e4ab269, without hand-authored item overrides.
Fresh qengine2 9c581fd10 and questpc2–8 refs contain no MapBase container opener;
reuse existing views, map MINT, replay and reset ownership and implement that gap here.

### Reset projection identity probe
Source stock's durable uniqueness is per world/channel/base digest/reset epoch.
The next PG probes seed the durable *result* of a completed reset activation,
retain the same live Channel cache, and read epoch1 through the production source
reader. They qualify cache/source behavior, not reset publication or quiescence.
Retain and rerun `world_reset_atomic_activation_calendar_house_refusal_and_dispatch_remint`
for the production reset lifecycle. Never label a seeded activation pointer as a
full production reset. The observer probe uses a distinct admitted runtime actor,
session and character lease, following the existing pressure-gate fixture.

### An Uneasy Alliance damaging-player death source
Source: https://github.com/zimbadev/crystalserver/blob/9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d/data-global/scripts/quests/an_uneasy_alliance/creaturescripts_renegade_orc_kill.lua
Pinned corpus blob8d1ecd7810b863df6f44e9e4aead98264fe50395 invokes
onDeathForDamagingPlayers and changes explicit QuestDoor0 to1. Conservative
absence rule: no stored flag means inactive, matching Lua absent storage -1;
do not infer eligibility from the lowered catalogue initial0. This slice does
not establish NPC-start or timed crystal-reset qualification.

#### Ordinary creature callback implementation boundary
The existing QUEST-STATE-0 writer, Character revision lane, native damage ledger,
kill retry queue and corpse pins remain the owners. A closed source binding selects
a damaging-player callback; party membership and ordinary XP principal do not select
its recipients. Capture freezes explicit stored flag eligibility with actor/session/
lease and observed quest revision. The same locked writer requires that flag value
and committed track revision not newer than the capture. Generic calls without the
source context refuse. Existing cause occurrence receipts provide replay identity.
This adds no tables, migrations, protocol fields, monetary grants or separate queue.
The native global monster event registration is blob157b75d352a131c57f7dabda0bc2f9699d5d9b66;
the alternate crystal monster blob3c48000b8d76d9b93df9d1457be15e630b5ada2c has
no event. The explicit global quest-source binding is authoritative for this slice.
Full An Uneasy Alliance qualification still requires native Curos start, all damaging
players coverage and the later crystal transformation/reversion source route.

### Curos source intake for the next native NPC slice
Public source verified 2026-11-07: https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/curos.lua
SHA256 6feee8de2475788bca144210fd47cbf857b5dce3a16215ff2647e971bcd3a42e,
Git blob4fdffd8e90b8de6d2f17ce5c38c45020397c48c9,7139bytes. It is byte-identical
to the retained9f5a72c corpus blob; future regeneration can reuse those verified
bytes without committing another source-data copy. Native NPC declaration already
admitted; normalized placement candidate33174,31394,z7,lookType29.
Mission precedence: NewFrontierQuestline16 and18 branches precede UneasyAlliance.
Start requires NewFrontierMission06>=5 and UneasyQuestline<1, then writes Questline1
and QuestDoor0 together. Mission at Questline1/QuestDoor1 sets2; mission at2 sets3;
mission at4 sets5 and grants item10217. No source quest credit for absent door flag.
Next tests must use actual native Curos dialogue and these prerequisite guards.
Later crystal action40030 changes10186->10187 for10seconds and only advances
Questline3->4; check qworld/other lanes for a reusable timer before implementing one.
These are verified source facts for continuing work, not full-quest qualification.

## 2026-11-07 main merge: exact charge overlap

Main's ITEMHOLD fallback overlaps the existing source-exact Ornamented Shield
recipe. A source raw count of one remains one item with one charge; it is not
expanded to the definition's 150 charges. The historical SOURCE mismatch remains
as provenance while the admitted native recipe retains its exact one charge.
The existing `test_charge_native` regression and the strengthened merged variant
assertion enforce that count and charge together. Independent ITEMHOLD admissions
remain available. Frozen SOURCE cores were compared in full before compiler-pin
refresh; no source core or qualification was invented to resolve the merge.

## 2026-11-07 main merge: starting-item provider ownership

The starting-loadout cohort overlaps the already admitted Newhaven profiles for
3571, 3572 and 7991. Transfer existing records verbatim into the cohort's single
provider, remove their duplicate base entries, and regenerate both manifest
digests. Existing identity, source binding and custody must exactly match the
source-derived cohort; conflicting binding/admission is rejected before mutation.
Earlier Wiki/native facts remain authoritative. Added RED/GREEN regressions
prove full-record retention, one provider identity and both digest updates.

## Mainland Citizen source binding, compiling WIP
Sources: https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/lubo.lua
and https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/hanna.lua .
Fetched bodies match existing archived corpus bytes exactly: Lubo SHA256
`a20ddf0044230dafc4c7c372caac1ec252b594cc0ff791fe5f8f622a1410e73d`, Hanna
`cd2ec86031b89bd244acbf43b9a4bfe3c63ef8165b9c7de24aed484f15bb3956`.
Lubo:100 minotaur leathers5878, strictly more than7200seconds, Citizen addon1.
Hanna:3374x1,5890x100,5902x50, Citizen addon2. Reuse existing QENGINE owners;
refreshed qengine2/questpc2/3/4/5/7/8 contain no Lubo/Hanna callback implementation.
Preserve existing zero initials/bounds. Reuse QENGINE Rook timer normalization:
store the opaque start stamp and require elapsed>=7201seconds in the SQL owner.
Lubo acceptance closes immediately through existing EndConversation0. Hanna
partial recipes retain the shared material-refusal reply; verbatim per-item failure
text is not claimed by the planned state/value proof. Own PG qualification pending.

Citizen dialogue composition: source flag0 offers and acceptance share the NPC
programme's topic store. Source flag1 material offers use reward-callback topics.
This source-stage split requires no new engine schema or persistent latch and
retains topic2 Lubo/topic1 Hanna acceptance guards. The two PG stories are the
qualification gate; unit planner success alone earns no quest qualification.

Hot Cuisine source follow-up (2026-11-07): main QNPCFX supplies actual UTC month
and delayed1000ms closure, satisfying the earlier month/dismissal request.
Primary Jean Pierre source is https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/jean_pierre.lua
(SHA256 dd9e5a92688d4563e5bb5590f4d436f35db87b9adec435dfe543f3ea967518bf).
Lines109–160 require year/month comparisons against each stored last-cooked
Unix timestamp, a first-completed-menu bonusyear and per-dish bonusused flags.
A365-day elapsed approximation would not preserve August/leap-year eligibility.
Sent a narrower calendar-period/current-year-write ENGINE-REQUEST; remaining
item/count/bonus bookkeeping can use existing bounded tracks and reward owners.
The131 currently lowered rules contain menus/storage only, not dish item grants;
therefore month/dismissal support alone does not qualify the full quest.

Krailos review (2026-11-07): Fandom API returned402; retrieved
https://www.tibiawiki.com.br/index.php?title=Krailos_Quest&oldid=445501 instead.
Hungry New World starts with Larek (33658,31662,z7), not the banker-only Grubokk
stub. Ghorza's Water route is available when the Salt Lake is dry and changes
shared refill/toxin state; Survival of the Strongest includes Grubokk's machinery
sabotage, while Vuzrog's Thunder Tusk objective counts200 Ogre Savages/Brutes.
These facts differ materially from the chosen packet's abstract champion/ten-goblin
summary. Do not certify the real seven-task quest with that approximation or attach
invented quest lines to the Grubokk banking stub. Sent a shared world-state/NPC
predicate and fenced refill/reset request; current main calendar registry has no
Krailos/Salt Lake entry or player-action mutation bridge.

## Yalahar UID3091 donor-map verification (2026-11-07)
Source: https://github.com/zimbadev/crystalserver/blob/00ce02a57ca5a12e48f32a3476e37471167e4c3f/data-global/world/world.otbm
SHA256 dcb735549bd11de526c4bd441bbf62e4490efbb60ff7aa334530f1692345d8d7.
Repository otbm_reader.read_tiles decoded19,325,129 tiles and found zero items
whose unique-ID attribute5 equals3091. A raw byte match cannot establish a
source placement. Keep the held exact-UID callback unchanged; do not manufacture
an object or substitute coordinates for the stored unique ID.

## Botany and Lion's Rock source decisions (2026-11-07)
Rabaz source: https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/rabaz.lua
SHA256 b74ee3591cc967215c463d104d887ace6d448f3ea46d7521f0a0a264f5347643.
Final yes consumes11699x1, gives11700x1 plus physical3035x10, awards3000XP
and sets questline4. Shared qengine2 live head279a9cff8 still rejects multiple
GiveItem types. Extend the Bigfoot request; do not silently substitute a wallet
credit or synthetic container. Almanach chest proof alone remains insufficient.

Lion's Rock sources: https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/lions_rock/actions_lions_rock.lua
and https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/lions_rock/movements_lions_rock.lua
SHA256 actions985436b56fe3e623d6574feb0265b2fae6ba04bee76ba2bcb522b4e0b02f5aac;
movements6c2ae2f15c8df502ce1f24cbd1d814097f3f9f0ffd53454ded08b12cdaf398a9.
Three sacrifices require exact target positions and an unlit pagoda; source
resets pagodas after15s. Flower21388 transforms21935 and regrows after60s;
StepIn amphora21945 transforms21946 and resets after15s while granting scroll.
These are shared object states, not per-character cooldowns. Extend the Uneasy
Alliance timed-transform request. Retain source timers and selector requirements;
final fountain separately chooses among seven rewards, excluding lion's heart
when mount40 is owned, and enforces a strict next-day timestamp.

## Blood Brothers and Barbarian source decisions (2026-11-07)
Julius: https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/julius.lua
SHA256 a3184eb9261079bc3fc08b97a5437fc34bf57b7df84b95dba39432316e775ea4.
At source stage11 Julius checks necklace SLOT for3083, then writes Mission01=2.
Carried ownership is not equivalent. Shared item programme currently rejects
Equipped guards, including at live qengine2 head279a9cff8. Requested its durable
owner composition; do not truncate/bypass this stage to activate later bread.

Barbarian sources at https://github.com/zimbadev/crystalserver/tree/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/barbarian_test
(action_mead.lua1bc325...1db88; action_horn.luaee5a98...197f) and
https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/sven.lua
(SHA256885076bfc128330b5ab9c41599bc6e604e710927b75367f9945d5cd3034bbe95).
Mead uses math.random(5)>1; failure resets successes0 and applies a1s outfit.
Counter9 succeeds before total increments; total>20 asks for a new round.
Sven resets total0, not the success counter. Preserve these literal source
boundaries; do not guarantee completion after ten USEs. Mammoth requires live
CONDITION_DRUNK. Requested random quest outcome/condition owner composition.

## Yalahar startup binding correction (2026-11-08)
The earlier map-only conclusion was incomplete. Both raw source maps contain no
UID/AID attributes because Crystal assigns them during startup. Authoritative
assignment source: https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/startup/tables/item.lua
SHA2567f551a398529e18ccad1ba56d740a4961874bbddd4736dd2df00272ddf6ea6ad.
ItemUnique explicitly binds1018/1019,3071–3074 and3091–3102. UID3091 is item8618
at32743,31164,5. Existing quest_access_bindings.overlay_region verified all18
source item/position pairs in current native regions without writing anything.
Import these stored attributes; keep original callback selectors. A coordinate
fallback in runtime matching remains forbidden and unnecessary.

Sewer behavior source: https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/lib/register_actions.lua
SHA2565121124402c29aead613174c80e94e6b8d5c7db1e62ad152ad68f1280fc3df6d.
onUseCrowbar(3304/9598) uses3071–3074, increments Mission01 once per pipe, gives
3s skeleton/5s bog-raider disguises and random rat spawns on the second pipe.
These coupled effects remain requested from qengine2; a four-USE approximation
would omit source behavior. No new whole-quest qualification claimed.

## Yalahar startup bindings and jade replica qualification
The raw OTBM has no script UIDs because Crystal assigns them at startup. Reuse
the shared quest_access_bindings importer and original ItemUnique facts from
https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/startup/tables/item.lua
The pinned Git blob is verified before generating SHA pins;18 exact roots are
restored, without creating objects or replacing UID matching with positions.
Only14 supported mechanism callbacks may activate. Sewer3071–3074 stays with
qengine2 until its coupled counter/outfit/random effects exist. Whole quest is
not qualified by a mechanism component test. Test-only stage arrangements retain
all production tracks and compatibility history.

Post-collector raw-speed regression revealed own Jorge jade replica31268 was
physically admitted with source equipment facts but unknown speed. Qualify its
speed from the same pinned Canary XML and Abilities zero default used by the
existing quest item importer, rather than relaxing the four remaining unknown
equipment records. XML omits speed for jade amulet; zero is the documented source
default. Regenerate the exact cohort/unknown-speed pins with repo tooling; require
raw ability equivalence and source facts to pass. Other unknown profiles remain
unchanged. Source: https://github.com/opentibiabr/canary/blob/04b83b512114bfd888000d6e1433ed8ecaec7c5b/data/items/items.xml

Verified zero default at https://github.com/opentibiabr/canary/blob/04b83b512114bfd888000d6e1433ed8ecaec7c5b/src/items/items.hpp#L29 (Abilities::speed=0).

## Children source follow-up; no partial oil/puzzle publication
Pinned source grease action has Lua precedence bug `not target.actionid ==8013`.
Owner exact-match charge rule takes precedence: a future implementation must
match the original five startup LeverUnique8013 objects (item9125 at33349,
31123..31127,z5), never accept arbitrary targets. This is a documented conservative
source correction, not an inferred coordinate fallback. Source success is public
TALKTYPE_MONSTER_SAY; current callback success_message is private. Request public
speech rather than silently changing the audience. The subsequent lever puzzle
needs conditional multi-object toggles plus exact5s reset; no partial flag-only
callback is published. Sources:
https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/startup/tables/lever.lua#L94
https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/children_of_the_revolution/actions_mission4_grease_oil.lua
https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/children_of_the_revolution/actions_mission4_levers_puzzle.lua

## Kissing a Pig source review
Keep the approved authored recipe's explicit linear lesson order, deterministic
Zebron practice and deferred five coins; do not import source gambling. Fandom
request returned402; public https://www.tibiawiki.com.br/wiki/Kissing_a_Pig_Quest
confirms Hoggle's garden dialogue and the completion message "You advanced in
lawn mowing" but supplies no numeric cut threshold. Pinned Crystal Hoggle has
greetings only. Its shared onUseMachete transforms jungle grass3696→3695 and
calls decay. A native-map survey found38 existing grass roots in a rectangle
east of the house; this is not claimed as the quest's required count or complete
garden boundary. Hoggle is upstairs (z6), garden ground isz7; do not invent a
basement placement from the Wiki map-link floor. Request fenced successful-tool
objective receipts preserving native decay before adding quest progress; never
replace normal machete behavior with a permanent transform or charge failures.
Sources: https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/hoggle.lua
https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/lib/register_actions.lua#L864

## Kingsday finite native binding decision
Use the existing approved three-stage, on-demand Oteryn recipe unchanged. Entering
courtyard cell32313,32182,z7 records exploration; King Tibianus at the captured
Global candidate32311,32172,z6 accepts `hail king`, then `kingsday` records the
journal and awards canonical `oteryn:achievement/loyal_subject` once. The final
confirmation makes the recipe's separate journal stage explicit. Calendar,
raids, exhibition and jail remain outside this chosen recipe. Reuse QENGINE2's
immutable authored binding attachment; compose only this owned journal in a
small new file, preserving all earlier catalogues. PG travel setup moves only
the actor between captured locations; progress must come from literal commands.
Sources: https://tibia.fandom.com/index.php?curid=76244&oldid=845148
https://tibia.fandom.com/index.php?curid=76245&oldid=994341

Kingsday callback provenance includes the captured Canary47dfd51f King source
identity/placement alongside the immutable Wiki recipe references. That source
witness names native identity and position only; it does not attribute the
on-demand Oteryn progression to donor Lua or pretend a calendar script exists.

## Exact NPC13 source predecessor reconstruction
The unchanged unit and retained-key PG test both pin the published Tibia Tales
NPC13 definition8c6494650b3d22ccd66d6893c77f8b0ad16b06dfcd69725316150c1be49ee084.
QNPCFX changed bound refinement from declared programme ranges to literal-write
ranges; rescued01/06/08 therefore narrowed from min-1 to0. Two later canonical
reward callbacks (hallowed-holy-water/outfox-helmet) also entered layer0. A typed,
read-only catalogue probe restored the three historical lower bounds and omitted
those later callbacks; QuestStateCatalogue::new reproduced the exact pinned hash.
Restore that finite historical prefix for layer0 and register its validated hash/
defaults for active layers. Active tracks/transitions/bounds remain unchanged;
all prior NPC legacy maps and predecessors are retained. Do not replace the
expected hash or relax the general additive-extension validator. No save is
rewritten and no caller-supplied hash is admitted. Existing PG RED on18.6 ran
53.374819329s at the exact saved-definition assertion; no tests were edited.
Source: https://github.com/Oteryn/Oteryn-Game/commit/099836485
Compare quest/npc_rules.rs declared-range refinement against current literal
write bounds; private probe facts and candidate source remain in/tmp/questpc-npc13-*.

After restoring the hash, the same unchanged regression exposed a second issue:
NPC rule catalogue reconstruction discarded predecessor maps built by earlier
source cohorts, including Draconia. Preserve that map when constructing the NPC
catalogue. The generic additive-extension rejection rules remain unchanged.

The native-kill extension between reward and NPC composition also discarded
history maps. Carry both its existing NPC legacy map and predecessor map into
the reconstructed catalogue; then preserve predecessors through NPC composition.
No new counters, transitions, bounds or reward paths are added by this repair.

Retaining both maps also recovers the real Citizen aggregate predecessor from
questpc-citizen-reward-callbacks.json. Update the exact history-key inventory to
require it alongside Draconia and Tibia Tales; retain the original8c649465 hash
and both original key assertions. Add exact-default lookup and cross-quest
substitution refusal assertions for the newly retained Citizen predecessor.
This strengthens the complete-history invariant rather than dropping history
to satisfy the old, incomplete two-key expectation. No PG tests were changed.

## Retained-key PG carried target repair
The existing exchange_carried helper used is_some() on use_with_map_item and
then always required a vessel Item handle. Hidden Threats passes a world Object,
so that fixture-side conversion fails before sending a command; the helper
subsequently reports server liveness loss instead of the earlier client error.
Refresh handles only for MapUseTarget::Item; preserve Object targets exactly.
Ready-work maintenance alone was tried and failed, then removed. No runtime
policy, liveness deadline, door assertion or saved-definition hash is relaxed.

## Main c77 reconciliation decisions
Main already owns 35 keys in the lane's source profile cohorts. Count net-new
profiles against immutable main c77 input documents, independently of generated
output; keep every one of the 166 source profile assertions. The repo updater
records and verifies that baseline instead of accepting an observed count.
Canonical main fluid child MINT markers initialize child state once; the lane
ordinal-zero fluid plan initializes only root state. Preserve both guards.
Source pattern UNKNOWN values may refine recursively only when ordered pattern
identities match; known facts and differing pattern IDs remain conflicts.
One-way non-lever passages may restore on departure only when both explicit
replacement IDs exist. Existing durable-departure refusals remain unchanged.
