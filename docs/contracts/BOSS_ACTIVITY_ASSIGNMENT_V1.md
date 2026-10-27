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
