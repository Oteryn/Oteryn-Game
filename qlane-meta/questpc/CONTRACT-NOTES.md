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

Upper key-door slice: exact native source selectors3004-3008 are retained from
pinned door_key.lua; no proximity aliases. Five real PG access cases green.
First key3001 is declared inside skeleton5972 in container_misc.lua at32794,31572,z7.
Source loads container contents before recursively applying key actions; stock is
filled only when the source container is empty. Primary source:
https://github.com/zimbadev/crystalserver/blob/9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d/data-global/startup/tables/container_misc.lua
