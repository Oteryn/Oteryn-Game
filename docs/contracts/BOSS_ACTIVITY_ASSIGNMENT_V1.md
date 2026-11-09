# Boss activity assignment v1

Owner authority: BOSS-ENCOUNTER-1 owner decision 24 (fresh activity instance per
admission), the accepted [instance baseline](../architecture/INSTANCE_SCOPE_AND_RUNTIME_OWNER_BASELINE.md)
and [ENCOUNTER-RT-0](../architecture/reviews/OTERYN_GAME_ENCOUNTER_RT0_ENCOUNTER_RUNTIME_AND_BOSS_LEVERS_DECISION_2026-10-01.md).
This document specifies the allocation boundary implemented by migration 0247.
It does not establish activity admission or runtime qualification.

The assignment kinds are 1 Channel, 2 House and 3 Activity. Activity identity is
exactly `(WorldId, InstanceId)` with UUIDv7 identifiers. Its assignment key is the
33-byte sequence `0x03 || world_uuid_bytes || instance_uuid_bytes`. Channel and
house assignment encodings retain their existing meanings. Assignment rows have
an `activity_instance_id` only for kind 3; their Channel and House columns are
NULL. Assignment identity is immutable, including after revocation.

Session and runtime guard kind 2 continues to mean Instance. Its guard key is
`0x02 || world_uuid_bytes || instance_uuid_bytes`. The readiness authority maps
this key to activity assignment kind 3 before checking the current assignment,
registration, holder possession, ownership generation and attestation. Runtime
keys and assignment keys are separate tag spaces; kind 2 is never overloaded in
the assignment table. No ChannelId is manufactured from an InstanceId.

Activity operations use the existing bounded `RuntimeScopeAssignmentWriter` and
`AssignmentCommand::{Assign, Replace, Revoke}`. Commands retain version 1 and the
existing operation/actor offsets; the embedded scope key has tag 3. A database
owner grants the control login exactly one World, Instance and operation through
`game_control_activity_scope_grants`. Channel and house grants convey no activity
authority. Both the effect and immutable receipt guards enforce this grant.
Runtime roles receive no assignment or grant write privilege.

All three scope kinds share assignment source revisions, high water and immutable
receipts. The existing writer slot, semantic deadline, exact command binding,
predecessor CAS, current node registration and crash reconciliation apply.
An identical operation replays the original receipt; changing its scope or other
command bytes conflicts. Readiness replacement/revocation fences the Instance
guard at the successor ownership generation with `ready = false`. Stale generation
or holder possession cannot restore readiness. Receipts prove allocation decisions;
they never prove a Character handoff, encounter death or reward.

Gameplay revalidates retained assignment history inside its current semantic
transaction, before locking the matching assignment and ready guard. The native
history validator retains the singleton writer's share lock through that
transaction. A previously attested ready projection cannot authorize gameplay
after a partial restore leaves a receipt gap or a materialized assignment that
does not reproduce its latest receipt. Invalid history refuses both Channel and
activity runtime fences; it grants neither participant admission nor recovery.
The activity allocation qualification checks ready Channel and Instance runtime
fences against a transactional receipt gap, then rolls the administrative probe
back and requires both valid controls to pass again.
`boss_cooldown_list_postgres_real_dispatch_assignment_history_gap` independently
checks an admitted Channel connection: after accepted command-133 controls, an
administrative high-water advance leaves one receipt missing while retaining
the assignment, readiness and participant rows. A new command must return
Rejected with an empty payload, the durable read must refuse, and the participant
binding must remain unchanged. Its disposable database is not repaired by
lowering high water or rewriting retained history. This qualifies a refusal
after partial restore, not restart recovery or admitted Instance gameplay.

House and activity identity collisions are refused in either allocation order,
even after revocation. Assignment and bare-session checks serialize using the
same World/Instance advisory lock as house admission. A live bare instance session
also prevents allocating that identity as an activity.

The activity handoff boundary remains unimplemented. Migration 0247 therefore
refuses every live session that attempts to acquire a retained activity identity,
on insert and update. Enabling activity sessions requires the committed group
handoff proof, origin-channel routing and return/recovery receipts, current session
and lease fences, and the actual InstanceRuntime. Readiness publication alone does
not grant any of these. Each admission must allocate a fresh instance; replacing
an assignment is never admission and does not admit participants.

Migration 0265 adds the storage boundary for that future ownership flow. A group
retains its World/Instance, activity/template/content binding, destination
ownership generation and declared participant count. Its append-only membership
retains each source Channel, exact source session/connection/lease/ownership
generations, fresh destination session identity, Character interpretation and
saved entry cell with coordinate-frame/map/artifact digests. The source snapshot
must match the current Channel session, Character root and saved position when
inserted. These consistency checks are not current node, recovery, eligibility,
readiness, content-placement or session-use proofs; the future writer must supply
those independently within its semantic transaction.

Group membership serializes with the parent decision and competing reservations
of the same source session. A group only transitions PREPARED to COMMITTED or
ABORTED, once. Both outcomes and all participant snapshots are retained and
immutable. An aborted source may retry with fresh group/Instance/destination
identities; retained abort history does not spend the source session forever.
A deferred commit constraint requires the entire declared group and every exact
terminal Channel source plus bound Instance destination. The runtime and control
roles have SELECT only on these tables. Neither a prepared snapshot nor a group
receipt grants participant authority. In particular, migration 0265 leaves the
0074 house-session CHECK and 0247 activity-session refusal unchanged. A successful
group commit remains unavailable until the real ownership flow is installed;
this schema is not an admission writer, return receipt or InstanceRuntime.

`boss_cooldown_list_postgres_real_dispatch` additionally qualifies migration 0265
on its admitted Channel connection. Administrative preparations test isolated
origin/session/interpretation/entry mismatches, immutable membership and group
decisions, competing source reservations, retry after abort, the partial-group
barrier and refusal of a complete membership list without destination ownership.
All preparations roll back, the participant binding stays unchanged, and the same
connection must still accept command 133 with the same cooldown page afterward.
This is storage/refusal qualification, not positive Instance gameplay evidence.

The same scenario also uses separate PostgreSQL transactions to qualify source
reservation serialization. Backend lock observations prove a competing group is
waiting on the first preparation before it commits. Exactly one membership is
retained; after its abort, the refused group can retry. A separate probe advances
only the source connection generation while a preparation's trigger waits. The
generation selected before that wait is refused after the advance commits.
Restoring this administrative probe permits preparation again; every prepared
group is aborted with its membership retained. The admitted connection must still
accept command 133 afterward. These are storage races with administrative writes,
not production reconnect, compensation, handoff or Instance admission evidence.

The native PostgreSQL qualification is
`durability::boss_encounter::activity_assignment_tests::boss_activity_assignment_postgres`.
It covers real assignment APIs, exact grants, command replay/conflict, readiness
fencing, session refusal, retained history and house/activity collisions. It is
allocation evidence, not real-dispatch activity gameplay evidence. Repository CI
and independent review remain required before integration.

The shared durable session/Character-root fence distinguishes Channel and
non-house Instance identities. It matches the exact session kind, nullable
Channel/Instance columns, World, connection generation, lease generation and
ownership generation; it checks the current node's activity assignment with
assignment tag 3 and the ready runtime guard with runtime tag 2. House sessions
cannot borrow this activity path. The Character root is checked in the scope's
World against the current Game-owned interpretation. These checks run inside
the existing caller's semantic transaction and deadline and grant no admission.

`boss_activity_gameplay_fence_postgres` qualifies accepted Channel reads and
refusal of a forged Instance fence, an unallocated bare Instance session and an
allocated/ready Instance without a participant handoff. The cooldown real-dispatch
scenario also exercises refusal at the composed and durable boundaries for a
Channel actor presented with an Instance fence whose UUID bytes equal the Channel's.
The forged-scope assertions call those two boundaries directly; ordinary Channel
traffic in that scenario exercises the actual transport. Neither test qualifies
a successful activity session,
an InstanceRuntime, a participant's origin/content snapshot or a committed return.
The 0074 house-session CHECK still forbids origin-channel metadata on a non-house
Instance. It must be extended together with the committed activity handoff proof,
not independently to make a test fixture pass.

The session fence now calls a separate crate-private runtime-scope sub-fence.
That sub-fence resolves the exact assignment key and runtime guard key, locks
both rows in the caller's existing semantic transaction, and proves the current
holder's node possession, ownership generation and attested readiness. The
native `boss_activity_assignment_postgres` case qualifies a successful Instance
**runtime sub-fence** as well as allocation without readiness, wrong World,
Instance, scope kind, ownership generation, node, registration revision and
possession secret, replacement without successor readiness, and revocation.
Each changed node fact retains the fixture's valid possession secret; the
separate secret case keeps the original node facts. Channel authority continues
to pass while the independent activity is fenced. This is not a successful
Instance session/Character-root fence or an admission receipt.

`boss_activity_channel_consumers_postgres` qualifies explicit Channel-only
checks at tutorial islet placement, Newhaven departure and bed-regeneration
settlement. An allocated/ready Instance cannot acknowledge the tutorial's
pending islet placement, create or replay a Channel travel receipt, or settle
bed regeneration against the saved Channel projection. The tutorial obligation,
travel obligation and saved position remain available to the valid Channel
consumer; Channel placement, departure and receipt replay succeed. Bed coverage
uses the no-sleeper fixture and does not qualify recovery of a real sleeper in
an admitted Instance. These checks do not substitute for an activity return.

The same gameplay-fence case also qualifies Channel-only saved-position and
respawn consumers. An allocated/ready Instance fence cannot read or overwrite
the saved Channel position, poison another write in a mixed periodic batch, or
invoke either respawn placement callback. The saved cell, vitals, rest state,
lease/write order and Character revision remain Channel-owned. This does not
qualify an Instance death or its required D119 home-temple return.

That case also commits a Channel death through the real durable death writer,
leaving its occurrence-bound pending respawn live. Instance reads, both consume
APIs (with either the exact occurrence or no occurrence selection), and both
placement callbacks refuse without losing that obligation or changing the saved
Channel position. A failed Channel placement retains both; a successful Channel
placement atomically consumes the obligation and writes the supplied cell. Death
receipt replay after consumption does not recreate the obligation or overwrite
the placed cell. This is durable consumer qualification with fixture coordinates,
not real-dispatch Instance admission, D119 destination resolution or restart
qualification.

Command 133 also proves retained session identity use before taking its item-only
participant fence. Inside the same bounded semantic transaction and existing
admission relation locks, the fresh-admission owner validates the Character's
session-use ledger against its immutable membership count and revision range.
The exact session must belong to that Character and retain a nonzero operation
binding. A missing or incomplete ledger, a revision gap, another session or
Character's membership, or a legacy/unbound observation refuses the page.
The existing 65,536-entry per-Character ledger bound limits this history check;
it is not a scan of every participant or session in the World.

For an original fresh-admission session, that owner also finds its immutable
admission receipt by the uniquely indexed session identity, independently of the
live projection's origin pointers. It validates the receipt with its existing
bounded payload/mirror and historical-operation decoder, compares the exact
Character/session and membership's operation binding to the original transport,
and requires the projection to retain that origin. When both origin pointers
are present they must agree. A same-session retained initial-origin pointer is
accepted without requiring the original connection generation to remain current.
Legacy and replacement sessions without an original fresh receipt for their own
identity retain the identity-use check; this addition does not prove their full
admission lineage. Current participant/runtime/root checks remain independent.

`boss_cooldown_list_postgres_real_dispatch_fresh_origin_unlinked` first accepts
the ordinary fresh session, then an administrative same-session projection with
only its retained initial-origin pointer. Removing exactly that pointer must
refuse a new command 133 with an empty payload and the direct durable read.
Every other session, Character, lease, presence and root fact remains unchanged;
readiness and valid assignment history stay intact. The retained admission
receipt, session-use membership and ledger are not rewritten. The projection
control is not a committed control loss or reconnect, and this refusal does not
qualify a production activity handoff, return or restart recovery.

`boss_cooldown_list_postgres_real_dispatch_session_history_incomplete` and
`boss_cooldown_list_postgres_real_dispatch_session_history_gap` first pass the
same real Channel connection controls as the original cooldown scenario. Each
then changes exactly one session-use ledger fact in its disposable database:
withdraw completeness, or advance revision and floor by one without a membership.
Assignment history remains valid, participant/session/lease/root rows remain
unchanged, and a new command 133 must return Rejected with no payload; the direct
durable read must also refuse. No completeness flag, revision or retained receipt
is reset to restore the database. The allocation/gameplay-fence case separately
checks absent history, exact membership, foreign identities, NULL/zero operation
bindings, incompleteness and both count/range mismatches. Those probes roll back
their own fixture transaction without editing immutable history.

This session identity-use proof is a prerequisite for an activity source
participant. It does not prove an activity transfer, group barrier, origin/content
snapshot, fresh destination ownership or recovery. Other consumers' shared
participant fences retain their current behavior; this additional proof is
currently composed at the cooldown read boundary only.

Command 133 also reads its runtime readiness through the admission guard owner's
existing payload/mirror and retained-history validator in that same transaction.
A valid assignment history and a ready runtime projection are insufficient when
that projection no longer matches the readiness publisher's retained decisions.
Invalid or absent readiness evidence refuses the page; database availability
errors retain their existing unavailable result. This additional readiness-history
proof is composed at the cooldown boundary only, not every shared participant
consumer.

`boss_cooldown_list_postgres_real_dispatch_readiness_history_ahead` first accepts
commands on the real Channel connection, then administratively appends a newer
readiness history row while leaving the runtime projection unchanged. The probe
uses the admission history key (`0x03 || runtime_scope_key`), distinct from both
the runtime and assignment key spaces. It retains valid assignment history, ready
ownership, session/lease/Character/presence facts and requires both a new wire
command and the direct durable reader to refuse. The disposable database retains
this deliberately inconsistent restoration probe; no history or protection is
rewritten. This qualifies a production prerequisite's refusal, not a readiness
publisher, successful handoff, Instance admission or restart recovery.

Command 133 also validates its Character and account publications through that
owner's payload/mirror and retained-history validator. The validated Character
publication supplies the account key; the existing participant fence still checks
that account against the exact session and live Character root, along with current
eligibility, lease and presence. All three guard validations share the same
admission relation locks, semantic transaction and deadline. Missing or invalid
publication evidence refuses the page; other database failures remain unavailable.
This additional proof is composed only at the cooldown reader, not every shared
participant consumer.

`boss_cooldown_list_postgres_real_dispatch_participant_history_character_ahead`
and `boss_cooldown_list_postgres_real_dispatch_participant_history_account_ahead`
each first accept commands on a real Channel connection, then append a newer
history decision for exactly one participant guard while retaining its projection.
The next command must return Rejected with no payload, and the direct durable
reader must also refuse. The session, lease, presence, Character root, readiness
and valid assignment history remain unchanged. The disposable database retains
the fault without rewriting history or protections. These tests prove refusal
of stale source publications, not a provenance writer, successful activity handoff
or independently current destination ownership.

Command 133's durable cooldown reader uses the item-only Character fence,
including the live root's account, World and current Game-owned interpretation,
before reading cooldown rows. It does not require an expected CharacterRevision:
an unrelated Character write is not a stale item/session fence. The
`boss_cooldown_list_postgres_real_dispatch` case changes each of profile, ruleset,
content and starter-template interpretation independently after the real Channel
snapshot. Both the direct durable read and command 133 refuse; the wire result is
Rejected with no payload. Publishing a new matching interpretation restores
accepted wire reads between cases. The fixture carries command-stream
continuity from its capability-refusal exchange, so sequence/setup failure
cannot stand in for the interpretation refusal. All four changes run on one
live connection after an accepted command completes carried-command
reconciliation. History is preserved through the operator's append-only interpretation publisher. This
qualifies the Channel consumer's Character-root fence, not a successful Instance
session, activity handoff or Instance gameplay.

The cooldown real-connection case changes the durable session to a bare,
unallocated Instance after the Channel snapshot commits, then attempts to send
command 133 over that connection. The connection may close before that write;
refusal is checked at the actual connection boundary. This administrative fixture
transition proves rejection of stale Channel authority; it is not a committed
activity handoff or session provenance implementation.

`boss_activity_full_character_fence_postgres` separately exercises the existing
CharacterRevision-bearing gameplay fence in its bounded semantic transaction.
The accepted controls are Channel participants at the current CharacterRevision;
a different expected revision returns CharacterRevisionMismatch. Wrong Character,
session, connection generation, lease generation, ownership generation, World,
Channel and a typed Instance with the Channel's UUID bytes all return
AuthorityRejected, both with the current and a different expected revision.
A wrong possession secret also refuses, with matching controls between cases.
This verifies the full fence's composition without creating an activity session.

The cooldown real-wire scenario additionally withdraws eligibility, changes the
live lease generation, removes its holder, changes the Character guard's account
or World, changes account presence to a different Character identity, and releases
account presence. Each isolated administrative fixture change must refuse both the
durable reader and a new command 133 on the same live connection. Restoring the
fixture fact must restore accepted results and the same bounded cooldown page.
A disconnect or missing CommandResult fails these cases. These fixture changes
are negative participant-fence evidence, not an admission publisher, activity
handoff or successful Instance consumer. The activity-session refusal remains
installed; full accepted Instance qualification still requires the provenance,
handoff writer and tests together.

Channel item consumers now delegate their session and root proofs to the same
shared fences. Their command's reserved Character, World and Channel are checked
first; an Instance with identical UUID bytes cannot satisfy a Channel reservation.
The root-bearing item fence also checks the current profile, ruleset, content and
starter-template interpretation. It remains item-only: it does not require an
expected CharacterRevision or advance that revision. The root-free helper proves
only the session/runtime/lease boundary; consumers needing a root proof must
still acquire it in their existing lock order.

`boss_activity_item_consumers_postgres` qualifies those binding checks and each
interpretation mismatch independently, with accepted controls after append-only
restoration. `boss_activity_item_consumer_postgres_real_dispatch` uses the existing
Channel item-move fixture: after an accepted command 133 reconciles carried
commands, four interpretation changes each refuse a new command 9 on the same
connection without moving custody or writing a move receipt. Restored command
133 controls succeed; the final command 9 commits exactly one split. This is
consumer-fence evidence for the existing Channel path, not an activity
handoff, Instance gameplay or a boss reward.

Applied migrations and receipts are immutable. Rollback uses a forward migration
that disables activity allocation while preserving all history and generations.
Never delete spent cooldowns or allocation receipts to roll back this boundary.


Actor references now carry `RuntimeScopeRefV1` rather than a compulsory Channel
identity. `ExactActorRef::channel_id` returns `None` for an Instance, and
`runtime_scope` retains the exact World and scope kind. Channel carriers compare
that typed scope before slot access; identical Channel/Instance UUID bytes cannot
borrow one another's slots. Channel placement/corpse digest v1 bytes are preserved;
Instance digests use separate domains and include the World, Instance, ownership
and local actor generations. Channel ActorRef and death-reference/key Debug text
retain their pre-scope shapes for native intent/owner-binding compatibility. No origin Channel is part of an Instance actor.

The carrier's namespace grant, independent continuity guard and actor storage
now retain `RuntimeScopeRefV1`; each position context retains that exact scope
through a lossless private representation. Actor issuance,
lookups, position commits and damage commits compare the exact World, scope kind,
scope identity and ownership generation. Claiming or advancing one namespace
cannot claim or advance another, including a Channel and Instance with identical
UUID bytes. The bone-overlord owner compares its Channel group against this typed
carrier scope. Channel position-context Debug bytes retain their original shape
because existing spell/relocation receipt bindings include those bytes.
The position representation uses the fixed UUIDv7 version nibble as a local kind
tag (7 Channel, 15 Instance, restored to 7 on decode). It retains every variable
UUID bit and the owning World without inventing a Channel identity. It is never
a wire, persisted or assignment key. The direct enum representation failed the
existing 176-byte Slot layout checks; this private 32-byte representation keeps
those checks unchanged. Malformed Instance identities refuse before namespace
claim or allocation. Qualification covers compact round trips and malformed
version/variant/nil Instance identities as well as the existing Slot budget.

`boss_activity_carrier_*` tests qualify isolated kernel slots, movement, damage
and replay in synthetic Channel and Instance namespaces, including equal local
slots/coordinates/UUIDs, a different World, generation replacement and retirement.
These tests construct private synthetic grants; the only production grant issuer
is still the existing committed Channel assignment facade. They are not accepted
participant sessions or real-dispatch Instance gameplay. Runtime lethal receipts
and non-persistent corpse projections retain the exact actor and position scope
for either namespace. The conversion `CreatureDeathOccurrenceRef::death_key`
remains Channel-only and returns `WrongScope` for an Instance; its key layout and
Channel Debug binding remain unchanged. Reward occurrence and committed corpse
Item binding also refuse an Instance rather than deriving a Channel from its
identity or origin. An installed encounter hook still requires the Channel death
key: initial projection refuses before cascading children or retaining a corpse
when that key cannot be obtained.

This representation does not admit an Instance. Existing Channel spell/item
writers reject Instance actors, and the committed creature-death/loot key remains
Channel-only, captured from the validated Channel owner. InstanceRuntime,
successful Instance consumers and handoff/recovery remain open.
The cooldown real-dispatch PostgreSQL scenario checks the admitted Channel path
and directly checks refusal of a typed Instance actor at its composed command
boundary; that refusal is not successful Instance admission evidence.

The tutorial progress API also requires a Channel scope before acquiring its
semantic pass. Its CAS can create the pending Channel islet-placement obligation,
so Instance reads, initialization, conflict replies, equal-state replays and
updates all refuse with `InvalidInput`. The existing shared participant and
CharacterRevision checks still follow the Channel scope check. The
`boss_activity_channel_consumers_postgres` case checks the allocated/ready
Instance and an Instance with the Channel's UUID bytes, before and after the
Channel creates its pending obligation. Matching Channel reads verify that no
refusal creates or changes that obligation. This is a consumer scope guard,
not positive activity session or handoff qualification.


`boss_activity_carrier_death_projection_replays_in_its_scope_without_channel_custody`
qualifies synthetic runtime death receipts and projections in a Channel, an
Instance with identical UUID bytes, another Instance and the same Instance UUID
in another World. Foreign receipts and owners refuse without mutation. A failed
projection leaves no projection; a lost response retains one exact projection
and retries return it. Bound damage, HP and position facts cannot be replaced.
Historical replay after slot reuse leaves the successor untouched; an ownership
generation change fences even retained receipts. Instance runtime projections
cannot yield a Channel death key, mint a reward occurrence or bind a corpse Item.
These are kernel checks, not admitted Instance combat, durable Instance loot,
Bosstiary credit, handoff or restart qualification. The production grant issuer
and composed death/reward path remain Channel-only.

## SRX-3 scoped physical corpse descendant

The one-item MINT owner now accepts the exact committed death's Channel or
Instance scope. This extends the historical Channel-only restriction above;
it does not create a Channel identity for an Instance. Crystal
`201189f871d23597b82caf88fd611454a2ddefb8`,
`src/creatures/monsters/monster.cpp:2412-2426` still creates a corpse and its
most-damage owner for a reward boss; `:2567-2569` suppresses ordinary loot for
reward bosses, whose personal entitlements retain their independent owner.

`CreatureDeathOccurrenceRefV1.instance_id` uses the next free nested field 6.
It is absent for a Channel death and present with an empty `channel_id` for
an Instance death. The MINT envelope carries the same exact namespace. Both
namespace fields, neither field, malformed UUIDs, a mismatched World, and a
Ground scope different from the death are rejected. Historical Channel bytes
and existing type-2 event tuples are unchanged.

Migration `0984` adds the namespace and Instance identifier to the existing
reservation/receipt owner. Its full key includes both the namespace and UUID,
so a Channel and Instance with identical local identifiers cannot share a cause
or replay. Current assignment generation, current node incarnation, recovery,
exact scope lane, capacity, immutable intent, reserved identities, audit bytes,
and the durable retry budget remain mandatory at the same boundaries.
An Instance additionally requires its installed activity and at least one
unreturned participant; a retained control assignment alone cannot authorize
new MINTs after the activity has returned everyone.

After commit, the existing physical corpse projection binds the Item in that
same scope and generation. After all participants return, the existing
`retire_activity_ground_step` owner retires the Instance's Ground and contained
Items before the activity runtime can retire. It does not move them to the
origin Channel. No new ordinary reward route or copied personal loot is added.

The same original MINT receipt installs the existing corpse-chain schedules for
Primal Menace's source corpse: Crystal `201189f`, `data/items/items.xml:73328-73347`
qualifies `39530 → 39529 → 39528 → 39527`, with durations 10/300/300 seconds
and the final 60-second lifetime. The existing field-chain consumer admits
Instance Ground only for these original corpse schedules and their installed,
unreturned activity; it retains exact scope/generation, predecessor, due time,
custody, successor, and audit proofs. Channel temporal receipt JSON omits the
new default scope columns to preserve its existing bytes. Instance receipts
include the discriminator and actual Instance identifier. General inventory
timers and unrelated Instance spell fields are outside this extension.

SRX-3 consumes the exact native corpse/yield admission producer slice from
`5e60c19701fe3acdda0a3d1331da1239440a9d1e` (DUPLICATE-ON-BRANCH producer
provenance). It reads existing exact native Item physics and legal destinations;
it does not infer unknown Item properties or copy a new reward table. Grand
Mother Foulscale's production raid supplies a nonempty source qualification:
Crystal `201189f`, `data-global/monster/raids/grand_mother_foulscale.lua:24`
corpse 5973, `:46` rewardBoss, and `:93,95` guaranteed green dragon scale/leather.
The original raid catalogue supplies the actor; its ordinary corpse and personal
reward descendants retain their separate existing owners.

The W11 native Item overlay preserves exact source corpse physics and every
original decay descendant (51 Items, including Grand Mother Foulscale and Elder Bloodjaw), with 15 separately qualified carried Item profiles (14 ordinary loot yields
and the explicit physical-corpse callback Item43860). Crystal `201189f` `data/items/items.xml:63392-63508` gives capacity48
for the six Soul War root corpses; `:76817-76890` gives capacity8 for the five
Rotten Blood roots, and `:16589` gives capacity7 for corpse6028. The generated
capture records each exact XML line. Capacity48 is accepted only for matched
actual Soul War rewardBoss/creature/corpse pairs with a verified native Ground-only
Item profile. It does not change the existing30 materialized-entry/work bound;
these reward bosses' ordinary physical corpse has no personal loot entries.

Three existing canonical reward Item references (43946,43947,44048) need bounded
native source bindings absent from the historical XML-only partition. The source
proof replays pinned DAT objects, `src/items/items.cpp:226` ItemType initialization
and `src/items/item.cpp:86` factory acceptance, plus actual loot at
`data-global/monster/quests/rotten_blood_quest/bakragore.lua:100,102-103`.
Their XML overrides are absent; `src/items/items.hpp:300` supplies their zero-weight loader
default. Their exact carried physics and source profile bodies are hash-bound in
the native reader. Those three existing references retain their identities. Additional actual Lua-created
Items43860,43589,43590,42851,42853,43625 receive reference-only declarations at
Item ordinals34111–34116 through the bounded repo producer. All canonical gameplay
semantics remain UNKNOWN. Their exact native profiles derive separately from
pinned XML/DAT, official appearance membership and the C++ Item factory/defaults;
no alias or inferred name/container/use rule is introduced. A source binding alone
grants no Item use or inventory materialization.

The same corpse timer owner consumes W11's exact original body durations rather
than the older uniform four-body schedule. Crystal `201189f`
`data/items/items.xml:16589` gives 6028→4206→4207→4208 at10/300/300/60 seconds;
`:63392-63508` gives Soul War four-body chains at10/600/600/600 seconds,
except33859 at10/900/600/600; `:76817-76890` gives each Rotten Blood three-body
chain at5/5/5 seconds. The last duration is terminal lifetime, not a further
transform. Existing immutable schedule rows preserve the original chain identity;
retirement requires the actual final transform receipt and exact terminal delay.
An unrelated or unknown source key cannot acquire a source corpse schedule.
The existing unscheduled-corpse60-second rule remains the accepted legacy policy.

Rotten Blood's existing Quest kill receipt accepts the existing Channel52-byte
native death DATA or Instance's discriminator2 followed by that same52-byte
payload. The Quest consumer validates every participant's exact runtime scope
and ownership generation before querying its source. The existing same-transaction
party-death receipt, membership, native boss key, damage-contributor and Character
revision proofs remain required. Namespace admission alone grants no progression.

Crystal201189f `data/items/items.xml:16272,12116-12126`: Grand Mother Foulscale
reuses that same source corpse owner for5973→4025→4026→4027
at10/300/300/60 seconds; the native overlay includes all four original body
profiles. Its personal reward remains independent of the physical corpse.

A same-socket activity barrier can install a fresh GameSession with generation1
and command1. ClientCommand's optional field16 binds its bytes to the current
GameSession UUID. Legacy unbound commands remain accepted on fresh/cold transports;
a transport that has crossed any activity barrier requires the binding thereafter,
including a later House route on that same socket. Any provided binding must
match the current session. Old or unbound successor bytes are discarded before
buffering/replacement peeks, ingress windows, command counters, replay acknowledgments
or physical owner calls. The server produces no fabricated result for those bytes.
Malformed/duplicate bindings retain normal protocol errors. The current bound
command1 remains usable after stale source commands arrive.

Rotten callback witnesses preserve the existing Tibia Global balance decision,
recorded read-only in `docs/architecture/reviews/OTERYN_GAME_CRYSTAL_REAUDIT_2026-10-28.md:195`:
Bakragore660000 HP and the other four600000 HP come from the already-qualified
`official-balance.json` overrides, rather than Crystal's350000. The accepted
rotten-charge explosion is flat1250 agony (official news7852), rather than
Crystal's40%-maximum-health expression. This does not replace the separate
standing-position agony percentages. Canary `47dfd51f`
`data-otservbr-global/monster/quests/rotten_blood/echo_of_{chagorz,ichgahal,murcion,vemiath}.lua:16-20`
qualifies all four Echoes'90000 HP and speed100. The other native physical profile
fields retain their existing exact canonical bindings; the full source bodies
are frozen by the bounded27-profile callback witness. Wiki rechecks are wiki-pending.


The Rotten Blood movement closure contains eight additional Ground-only profiles:
37000,42851,42853,43294,43295,43589,43590,43625. Existing43297 already has its
source profile. The exact producer binds the actual onMove bodies of Murcion,
Ichgahal, Chagorz, Vemiath and Bakragore, the shared boss mechanics and
`movements_fields.lua`. Callback-created Items remain under scoped Item custody;
map-only flames/pillars retain their source WorldObject identities. These are
separate owners: a source sprite never substitutes for an Item reference.

Crystal `201189f` `bakragore_mechanics_death.lua:21-43` adds at most one explicit
extra bag to the physical corpse, independently of the private ordinary boss loot
table. Its chance for essence counts1–4 is5/8/12/15 percent, followed by a uniform
choice between43895 and43860. The actual physical corpse pointer passes through
`src/creatures/creature.cpp:717-720,759-762`; the rewardBoss skip in
`src/creatures/monsters/monster.cpp:2567-2569` only skips ordinary dropLoot.
This source callback does not redirect the extra bag into personal rewards.

Bakragore preparation precedes the boss chamber. The literal boss entry is
(33044,32407,15), whereas the four essence flames occupy the separate cross around
(33076,32398,15). A read-only replay of pinned `world.otbm` with the repo tile
extractor and actual XML/DAT policies gives a264-cell walkable chamber component
(x33035–33052,y32390–32409), with no floor changes or boundary escape; unrestricted
diagonal reachability gives the same component. None of the four flame-adjacent
cells belongs to it. Therefore default zero essences cannot be replaced by an
assumption that a player activates those flames after boss-chamber entry.
`monster/quests/rotten_blood_quest/rottenEssences.lua:1-28,30-111` supplies the
actual stored-essence consumption and flame activation callbacks. Nonzero phase
facts require those real callers and an explicit, fenced preparation handoff;
taints alone are not preparation evidence.

D26 isolates each Instance's internal mutable encounter state; it does not
change eligibility for the source Channel's external flame preparation. Crystal
`rottenEssences.lua:74-94` guards a global flavor flag, then sets it and burns the
activating Player's stored essence. Bakragore `bakragore.lua:531-541` reads any
positive global flavor on its first think, independently of who lit it or who
entered its chamber. Its:570-608 clears those flavors and resets the corresponding
fixed visuals. Consequently the external phase owner is scoped to the actual
source Channel/generation/activation/reset epoch, shared across its Players.
Admission retains origin provenance; it must not consume or freeze the phase.
The actual first native think captures and consumes the current external phase
atomically, including explicit0. No source Character-in-roster condition or taint
inference may alter that selection. Instance-local counters and delayed actions
remain private under the existing accepted baseline. The earlier per-roster
"accepted difference" paragraph was an unsupported inference and is superseded
by these literal source rules.

Rotten Blood's level250 admission is also explicit in pinned Canary47dfd
`data-otservbr-global/scripts/quests/rotten_blood_quest/actions_murcion_lever.lua:6`
and `actions_chagorz_lever.lua:6`; Crystal omits the field in those two configs.
The other three Crystal configs state250 explicitly. The existing Quest owner's
all-roster level250 check therefore uses donor evidence rather than a guessed
BossLever default.

The Elder Bloodjaw source corpse is43669→43668→43667→43666, with successive
10/300/300-second transforms (Crystal `data/items/items.xml:75446-75463`). Its terminal43666
has no XML decay declaration in either pinned donor; `items.hpp:292,302` defaults
to duration0 and decayTo−1, and `item.cpp:3377-3380` rejects automatic decay.
The source-bound terminal reference uses Item ordinal34117, added through the
repo producer and guarded binding packet. The resulting corpse remains present
until InstanceRetire; a nullable retirement deadline means no timer, not a
60-second fallback. The source-ground corpse closure now contains51 profiles.

Rotten essence ItemUseCause4 composes the exact one-unit burn and its source
Quest flag0→1 in one Type2 transaction. `OneItemUseV1.instance_id` field15
identifies an actual Activity Instance and excludes channel_id/house_key. A real
named House retains the existing house_key plus origin Channel provenance; the
House assignment remains custody authority. Ordinary Channel wire bytes remain
unchanged. Generic item-only commits cannot issue cause4 independently.

Pinned Crystal also registers `rotten_bakragore1.lua:8-13,26`: the preparation
cross has its own real lever at(33079,32398,15), with five waiting cells
(33078..33074,32398,15). All four flame-adjacent cells connect to that roster
inside the actual227-cell walkable cross component. It reaches the same boss
and destination as the primary lever. Its literal20-hour cooldown is superseded
by the already selected68-hour final cooldown; both entry routes share one boss
and cooldown identity. The complete admitted roster must belong to one literal
lever group; combining members from the two disjoint lever groups is not a donor
entry. This permits real preparation and entry without a fabricated relocation.

The43667→43666 timer transform additionally requires empty direct and nested
corpse custody. A nonempty historical/corrupt state fails closed through the
existing timer receipt guard; it cannot silently strand children under a Generic
Item. This changes no valid empty Elder source flow and leaves the separately
fenced InstanceRetire tree path intact.
