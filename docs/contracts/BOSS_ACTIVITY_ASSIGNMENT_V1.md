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

The cooldown real-connection case changes the durable session to a bare,
unallocated Instance after the Channel snapshot commits, then attempts to send
command 133 over that connection. The connection may close before that write;
refusal is checked at the actual connection boundary. This administrative fixture
transition proves rejection of stale Channel authority; it is not a committed
activity handoff or session provenance implementation.

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

This actor representation does not admit an Instance. Existing Channel spell/item
writers reject Instance actors, and the committed creature-death/loot key remains
Channel-only, captured from the validated Channel owner. Generalizing the carrier,
InstanceRuntime, successful Instance consumers and handoff/recovery remains open.
The cooldown real-dispatch PostgreSQL scenario checks the admitted Channel path
and directly checks refusal of a typed Instance actor at its composed command
boundary; that refusal is not successful Instance admission evidence.
