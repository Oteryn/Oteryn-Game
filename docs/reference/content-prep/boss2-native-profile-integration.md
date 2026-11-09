# Native profile qualification after qmech integration

Stationary World scenery retains nonmaterializable admission. Native corpse
counterparts require materializable, Ground-only admission: the production
creature reward catalogue filters unmaterializable Items, and
`spell_item_transaction.rs::prepare_corpse_in_transaction` rejects them too.
Their nonportable destination and the existing death/corpse receipt remain
mandatory; data admission does not grant an arbitrary MINT caller. Source
physics, container capacities, decay, and appearance bindings remain unchanged.

The initial floor-assembly repair incorrectly extended the scenery restriction
to corpses. A forward correction restores its ten native corpse counterparts
and qualifies the four overlapping Elemental Spheres counterparts the same way.
The negative admission test now rejects actual inventory portability and also
rejects an unmaterializable corpse. The new Ground-corpse regression was red
for ten source records before this correction, then all three coherence tests,
four Elemental Spheres tests and the Elemental Arrival test passed.

Source: Crystal `201189f871d23597b82caf88fd611454a2ddefb8`,
[`data/items/items.xml`](https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/data/items/items.xml):
8106 at line 22531, 8139 at 22709, 17938 at 36511, and 17970 at 36536.
The existing capture tools verify XML SHA-256
`8a4a526c6e45b5a282679334eb1e1e25a5b72af13aa92c8a0d409755ebdbc509`
and the pinned native appearance bank before writing either packet. Wiki-pending.

The independent coherence tests were red for all ten corpses and four shared
records, then green; the elevator extension was red for eight shared records,
then green. Elemental Arrival and Elemental Spheres tests also pass. The
equipment cohort refresher recognizes both recorded comment forms while
retaining unique functional anchors and exact source membership checks; both
original tests and three new regression cases pass.

The existing World-to-quests dependency for Elemental Arrival is recorded in
the dependency contract without changing layer rules. Three unchanged scene
test probes moved into its existing test module; the production size ratchet
was not raised. Architecture and client Clippy checks pass.

Qualification remains PARTIAL. Native registration now reaches
`M42_CANONICAL_WITNESS_INPUT_DRIFT`; its own refresh refuses with
`M42_CAPTURE_PREDECESSOR_PIN`. Read-only comparison proves every selected recipe
row still matches the immutable predecessor, while seven shard pins describe
newer files containing unrelated changes. No witness SHA was hand-edited.
Quest-authoring checks exposed two reward-claim variant failures. The focused
native-client run passed six scene tests, but the start-area test rejects the
placement-index digest and the Ahau exit capture does not reach the return
position. These failures remain visible; no test is skipped or relaxed.

The subsequent M42 capture repair keeps the fixed predecessor and all selected
recipe-row hashes. It verifies the whole-shard hash against the current packet,
then independently compares the consumed rows with that predecessor. Three new
capture regressions and the original verification test pass, including refusal
of a fresh pin over changed recipe rows and refusal of an old whole-shard pin.
The repository refresh and verification commands pass; the generated witness
changes only `source_packet_sha256`, with no row or recipe hash changed.

The shared explicit-ID capture also filtered its result through the global
default cohort, dropping the real requested parchment 2833. It now retains the
requested selection; the three existing parchment tests pass. Default capture
membership is unchanged. Regeneration next reaches
`QREADY1_CRAFT_ITEM_CONFLICT:oteryn:item.tibia.i21506`. The complete two-crate
client run collected all failures: 438 cases, 424 passed and 14 failed; the
placement-index pin itself was refreshed with its validating repository tool.
The Ahau exit gesture is still under investigation, so this is not a completed
post-integration native play qualification.

The crafting join now retains established native facts through the existing
strict profile reconciler. Its only text normalization preserves the current
owner's terminal period for otherwise identical descriptions: current pinned
Canary `data/items/items.xml:42717–42724` defines items 21506/21507. The retained
craft capture omits those periods and leaves marketability UNKNOWN. No physical,
identity, binding or custody conflict is ignored. The real overlapping oil/dust
admission was red, then green; five negative conflict vectors still refuse
without publishing any output. Native registration next reaches
`PREY_MODEL_STALE`, whose existing simulation tool is being run.

### First command after a presented native scope

The native Ahau replay exposed a second command drain: the shared presentation
owner had already discarded the retired scope's queue, but the legacy
`activity_reset_commands` flag subsequently discarded the first destination
STEP. Matching presentation now clears that already-consumed reset under the
same mailbox lock. A mismatched session still releases neither fence.

The first-destination-step regression was red while the wrong-session negative
case passed; both then passed. The unchanged actual server entry/exit transcript
also passed through the native client in 0.220708162 seconds, including native
own appearance and the destination teleport effect. The peer observed the real
STEP and delivered all eight captured return frames. Clippy with `-D warnings`
passed for all client targets. Post-regeneration server PG reruns remain due.

### Reproduced native inputs

The existing simulator completed 15 cases over 777 creatures, then the native
importer regenerated 48 files with `activation=false`. The missing producer
module import in the equipment identity-pin tool is fixed. Its seven tests keep
exact source metadata, exact declared identities, duplicate/foreign rejection,
raw equipment checks and idempotence. Fixtures now include the required producer
metadata and verify the full identity witness rather than only a stale count.
The complete source cohort still declares 6251 profiles against a runtime union
of 6219; its independent sum guard remains enabled. This batch preserves the
generated inputs while that source ownership overlap is investigated.

### Independently proved cohort overlaps

The overlap composer now checks the exact identities of each source packet
before subtracting a closed, explicit intersection. Elemental Spheres shares
14 identities with floor assembly; Rafzane shares eight with the elevator;
five endgame corpses already belong to SRX3; crafting shares two QENGINE items;
ENGNPC-M05 and the two talisman declarations each share one QENGINE item.
These 32 duplicate contributions explain the previous 6251/6219 mismatch.
The runtime union is not an input to these deductions, and its independent
sum check remains enabled. Both regeneration and `--check` pass at 6219.
Four new tests cover exact deductions, idempotence, a missing corpse proof,
duplicate/foreign source identities, and refusal to repair an arbitrary prior
declaration.

The focused Rust production-profile test compiled successfully (4m49s), then
ran for 104.913 seconds and exposed a separate Galthen loot defect: item 44648
has raw speed 15 but its native ability sum is zero. This batch does not alter
that assertion. Quest-authoring checks still report the two reward-claim
variant failures; content-tree validation reaches `LOCK_COUNTS`. Those checks
are not reported as green.

The post-fix native PG rerun reached real database migrations, but neither
mechanic body ran: `srx3_cloak_` cases took 74.825 and 75.355 seconds and both
failed with `while executing migration 2020: error returned from database:
quest reward quantity variable requires exact guard anchor: actual BIGINT;
at line 3909`. Migration 2020 belongs to another lane and is outside this
lane's 2170–2179 range. A later migration cannot repair an earlier fresh-schema
failure. No foreign or merged migration was edited; current PG qualification
remains PARTIAL pending that prerequisite fix.
