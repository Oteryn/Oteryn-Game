# STAFF_TOOLS_V1 (NX-1)

Capability **330** requires negotiated `STAFF_V1` (51). Commands **560**
`STAFF_TOOLS_REQUEST` and **561** `STAFF_TOOLS_RECEIPT` carry the existing exact
`staff::SignedEnvelope`: principal, canonical request/receipt bytes and detached
off-host signature. The existing request and receipt signing namespaces stay
separate. No private key or local permission grant is introduced by this feature.

The signed staff request's closed action catalogue uses the donor GM/CM/God group
model under the owner's 2026-11-06 NX-1 decision. Current-Tibia internal tooling is
not publicly documented and the owner accepted donor tiers as the compatibility
reference. The server rechecks the current off-host roster, action tier,
WorldId/ChannelId, session generation, trusted time, control chain, replay and audit
before an effect. Negotiation is transport availability, never authorization.

## Reply

`StaffToolsReply` is a strict protobuf-shaped message:

| Field | Wire kind | Meaning |
| --- | --- | --- |
| 1 | bytes, required once | Existing strict `staff::StaffResult` (at most 512 bytes). |
| 2 | UTF-8 string, optional once | Inspection/progress text, at most 4,096 UTF-8 bytes. |

Information text may be nonempty only for disposition 1 with code `Ok`. A refused
operation or stored receipt cannot smuggle inspection text. Empty/missing field 1,
unknown or duplicate fields, malformed nested results, invalid UTF-8, truncation
and oversized payloads are rejected. Total reply cap is **4,618 bytes**, including
conservative protobuf framing allowance. Signed request cap remains **12,364
bytes**, with canonical bytes at most 8,192 and signature at most 4,096.

The native client shows server-confirmed results and inspection text. A pending
request is not displayed as applied; malformed or rejected replies remain
unconfirmed. Disconnected clients clear connection-local authorization material.

## Compatibility

The original capability 51 commands 60/61 and `StaffResult` bytes are unchanged.
Clients without capability 330 do not send 560/561 or decode the reply wrapper.
No state domain or unsolicited protocol message is allocated. World notifications
continue through the existing negotiated server-message channel.

New assignments: capability 330; command types 560, 561. Integration may renumber
these preassigned numbers together with their fragments and generated goldens.

## Signed issuer and delivery fences

Capability 330 requests require `issuer_character_id`, a canonical lowercase hex
Character UUID in the signed Request JSON. It must match the current fenced GM
character. Legacy 51 history omits this optional field without changing its bytes;
full tools never infer issuer identity from the request target or unsigned carrier.
A signed self-Inspect establishes the current principal/group binding before other
full-tool actions. Transporting a captured inspection of another player cannot
grant that player a group.

A world effect captures the target GameSession, connection generation and scope
ownership generation. Pending delivery cannot mutate a successor connection or
scope. Applying a pending effect re-proves the current principal and donor tier;
a historical outcome permits read-only replay, never reacquiring revoked authority.
`StaffToolsReply.result` confirms the durable audited authorization transaction;
`info` describes the observed world application/refusal. A blocked landing remains
Refused on replay. Broadcast/notify text fits the existing 255-byte message domain;
ban reasons are bounded to 1,024 ASCII bytes. Code 9 `AlreadyBanned` is emitted only
for extended ban requests; legacy 51 operations retain their existing result codes.

Runtime delivery additionally retains an actor's monotonic control revision across
all awaited lookups. Same-session control loss/reentry and terminal write fences
invalidate the retained actor proof even when actor identity and session ID are
unchanged. The issuer's revision is retained before the signed audit submission.
Forced Activity return uses internal durable reasons 10 (active transport) and 11
(recorded loss epoch); these are not wire allocations. Already lost sessions are
checked during the existing grace owner, without changing the original deadline.
