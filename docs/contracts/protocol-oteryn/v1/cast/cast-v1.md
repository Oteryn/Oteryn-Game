# S17 Cast V1 candidate contract

Owning design: S17-CAST-DESIGN, accepted section 6, revision
956a1cdff6188381b25419251654f63a8b19536f (PR #3000).
Capabilities 289 VIEW, 290 CONTROL, 291 CHAT and 292 CONSENT are **unoffered**.
SPECTATE1 capability 33, commands 34/35, domain 25 and family 15 are unchanged.
Player controls use commands 502 CONTROL, 503 CONSENT and 505 MODERATE.
504 CHAT / kind 10 CasterReply carries isolated caster replies.
Domain 102 carries replacement scenes; domain 103 carries live caster chat. No client UI is implemented here.

## Message

`crates/protocol-oteryn/src/cast.rs` is the executable schema. Canonical protobuf
subset: fields are required, emitted in order even at zero, with minimal varints;
unknown/repeated/out-of-order fields, trailing bytes and invalid enum/range values
are refused. Each variant starts with field 1 (varint kind).

| Kind | Message | Subsequent fields, in order |
|---|---|---|
| 1 / 2 | Start / Update | 2 revision, 3 Platform access revision, 4 delay 0–300, 5 cap 1–50, 6 password-required boolean |
| 3 | Stop | 2 expected revision |
| 4 / 5 | ConsentSet / ConsentState | 2 revision, 3 allow-shown boolean |
| 6 | Join | 2 one-time observer material, 1–4096 bytes |
| 7 / 8 | Leave / Keepalive | none |
| 9 / 10 | Chat / CasterReply | 2 plain UTF-8 text, 1–256 bytes |
| 11 | Moderate | 2 expected revision, 3 opaque 16-byte subject, 4 action (1 mute, 2 unmute, 3 kick, 4 ban), 5 nonzero 16-byte idempotency key |
| 12 | State | 2 revision, 3 positive epoch, 4 delay, 5 viewer count 0–50 |
| 13 | Slate | 2 positive epoch, 3 positive sequence |
| 14 | Clear | 2 positive epoch |
| 15 | Viewer ChatLine | 2 epoch, 3 sequence, 4 opaque handle (16 bytes), 5 text |
| 16 | Live CasterChatLine | 2 handle, 3 text |
| 17 | End | none; generic, no fresh cast content |
| 19 | CasterChatEmpty | none; playing-caster initial snapshot only |
| 18 | Scene | 2 epoch, 3 sequence, repeated 4 tiles then repeated 5 actors |
| 20 | CasterState | 2 revision, 3 epoch, 4 delay, 5 viewer count, 6 opaque 16-byte cast handle; owner only |

Caster control fields never constitute Platform access proof. Game must resolve
the access revision from the trusted Platform policy owner. Reusable passwords
never reach Game. Observer material must be purpose/audience/node/account/cast/
epoch/access-revision/expiry bound, single use and globally reserved before Game
admission. Only the online, mutually authenticated Platform consume/renew adapter creates
production `VerifiedAdmission` values. A ticket is not a character-login credential.

Observer envelope: required varint field 1 = **16**, field 2 = generation,
length-delimited field 4 = Message. Generation zero permits only Join. After
admission the generation is positive. Observer client messages are exclusively
Join, Leave, Keepalive and Chat; gameplay, control, consent, reply and unknown
opcodes are refused before dispatch. Observer server messages exclude live
CasterChatLine, ConsentState and CasterState. Family 16 has no legacy/player fallback.
The envelope limit is 16432 bytes; its message limit is 16384 bytes.

CHAT requires VIEW for an observer or CONTROL for a playing caster. Observers
cannot select CONTROL/CONSENT. Playing-caster controls require normal selected-capability negotiation;
observer VIEW/CHAT selection is bound to the purpose-separated account profile
and checked against Game offers before admission. Codec support never activates
an offer. The native listener routes family 16 before player bootstrap and never creates
a character actor. Password verification, global quotas and worker routing are
composed; client consumers are a later task.

## Scene

Each tile submessage has varint fields 1 x, 2 y, 3 public item type (all u16).
Each actor has 1 cast-local handle (u32), 2 x, 3 y, 4 public appearance (u16),
5 public health percentage (0–100). All submessage fields are required in order.
At most 512 tiles and 128 actors, no opaque player payloads. Scene replaces the
whole world view atomically. No names, account/character IDs, speech, inventory,
private UI, room chat, exact vitals, labels or unknown presentation subtypes.

Missing/stale bystander consent yields a whole-view slate. Explicit start is
the caster's own consent; other characters default off. The trusted committed
interest set excludes hidden/off-screen actors. Consent withdrawal purges queued
scenes, immediately clears the viewer scene and requires a fresh eligible
baseline. Release rechecks revision/freshness. Final synchronous nonblocking
writes serialize with the consent barrier under the cast owner, outside the tick.

The SRV11 producer uses the existing native map planner and committed entity interest; see the completion schema below.

All viewer content, including chat and metadata, follows selected D (default 0).
Accepted viewer chat reaches the separate caster queue live even at D=300.
Delay changes reset epoch and clear queues. Access rotation ends observers.
Stop, kick, timeout, logout/death/scope/session/lease/connection changes purge
queues and release reservations. Casting is allowed in competitive areas.

## Composition and rollout

Production remains off pending client consumers and measured node/worker/egress
qualification. `OTERYN_CAST_ENABLED` defaults false and enabling it requires
explicit node capacity, HTTPS Platform URL, protected client identity and pinned
Platform CA. TLS 1.3, no redirects/proxy, bounded requests and online consumption
are mandatory. All four capability offers remain false, independently gating
the native command path. Competitive areas do not prohibit casting.

The companion Platform 0968 implementation accepts an existing account-only
native admission ticket through the authenticated Gateway cast-watch route.
No character selection is accepted. It verifies optional Argon2id passwords,
reserves at most two sessions per account across casts and fifty per cast under
account-then-policy locks, and issues a hashed, single-use `ocv1.` ticket.
Consumption binds account, node, cast, epoch, access revision and selected chat.
Renewal rechecks current account security generation and all policy bindings.
Five-second global reservations have conservative local deadlines and online
renewal every two seconds; a keepalive alone never extends a reservation.
Leave, kick, expiry, stop and access rotation release or invalidate admission.
Password changes belong to the authenticated caster account; Game receives no
reusable password. A password-required new cast stays closed until a verifier
is installed. Policy announcements alone cannot remove an installed password.

Viewer chat has per-account and room limits, plain-text bounds and selected-chat
checks. It reaches the playing caster live through domain 103; viewer copies
follow D. Moderation uses opaque handles from those live chat lines. Mute affects
the account in that cast; kick removes one session; ban persists for the caster account.

Global consent withdrawal uses the 0966 durable invalidation/worker-lease barrier
and capture permits documented in `S17_CAST_CONSENT_PERSISTENCE.md`. Revocation
clears buffered and shown scenes before false commits, including delayed frames
on other workers. Missing, stale, expired or conflicting facts fail closed.

The Game audit adapter (0967) writes registered type-10 security envelopes with
UUIDv7 identities and P30D expiry. The existing typed Character/item outboxes
require unrelated receipts and retention tuples, so they cannot hold these facts.
Node possession/currentness and stream ownership are independently checked.
Incarnation/ordinal prefixes reject gaps and conflicting replay; acknowledgements
are constructed only after commit. Cast/observer terminal slots are reserved
before starts/joins. Viewer delivery waits for its audit prefix; a storage error
or invalid receipt closes casts, purges queues and retains reserved terminals.
Consent writes audit atomically with the preference. Runtime cannot delete audit;
Control has a bounded expiry-only routine. Export/GM composition is still pending.

## SRV11 viewer and moderation completion

Kind **21 Media** is the next free kind after base kind 20. It adds no
capability, command, family or state domain. Required fields: 2 positive epoch,
3 positive sequence, 4 PublicMedia (at most 8192 bytes). It is server-only and
uses the existing consent, delay, sequence and revocation barrier.

PublicMedia has required field 1 discriminant:

- 1 Map: fields 2 reset bool, 3 last bool, repeated 4 DisplayTile (max 32).
  Each tile has 1 x, 2 y (u16, relative to caster +16), 3 floor+15 (0–15),
  repeated 4 appearance IDs (u16, max 10, ordered ground-first). Map chunks
  replace the previous map from reset through last, covering all visible
  floors. They contain no item targets, handles or object revisions.
- 2 Effects: field 2 strict presentation batch, restricted to MagicEffect and
  Missile. Coordinates are relative to caster +16; floors use the existing presentation wire’s positive 0–15 convention (the inverse of map floors). No value text, spell words, entity references or sounds are accepted.
- 3 Speech: fields 2 x, 3 y (u16, relative to caster +16), 4 floor+15, 5 local speech
  mode (1–4), 6 text (1–256 bytes). No speaker identity/name or private/room
  routing can be represented. Only local speech actually delivered to the
  caster from a currently visible speaker is eligible.

Actor scene x is relative+16; y is relative+16 plus `(floor+15)*32` so floors
remain distinct. The normal native appearance and health representation is
preserved, including its existing placeholder appearance limitation. Map
appearances come from the real caster map planner and include creatures/NPCs
through committed interest. Every media packet carries the same visible-player
consent bindings as the accompanying replacement scene. Missing map authority
produces a slate. Full map captures remain at one per second.

504 CasterReply now reaches the isolated room, with per-sender and room limits.
Chat output text is bounded to 320 bytes to include a server-generated
`Viewer-<opaque handle>` or `Caster` prefix around the original 256-byte text.
Input bounds remain 256. Names expose no account/character identity.
Moderate action **4** bans the authenticated account for that caster account.
Migration **1095** stores owner/subject account pairs, capped at 512 per owner.
Ban and kind-12 immutable audit event commit atomically. Active same-account
sessions leave immediately; admission and two-second renewal query the durable
ban, including after cast/node restart. Mute/unmute remain cast-account scoped,
kick remains session scoped. The normal player command idempotency boundary
covers moderation retries; an already stored ban emits no duplicate audit.

The preference adapter and Platform handoff are documented in
`docs/contracts/S17_SRV11_GAME_PLATFORM_HANDOFF.md`. Offer/consumer qualification
remains independently gated as in S17-A; this server package does not activate
an unsupported native client.

The caster camera normalizes runtime `z` to native negative map floors before
joining entity and map captures. Actor floor planes and public speech use that
native convention; presentation effects retain their existing positive wire
floor convention. The map's `first_visible_floor` and window filter apply to
all projected tiles, actors, local speech and effects. Roof-hidden floors are
removed server-side, before any cast bytes are queued.

The production scene preserves committed interest's own-actor-first ordering:
public handle 1 is the caster camera anchor. Its relative x/y is (16,16), with
the native floor packed into y as above. Floor perspective uses that anchor;
map chunks never expose the caster's absolute world coordinates.

### CLCAST public camera

Within PublicMedia (Message kind 21), nested kind 34 is Camera: required
varint field 2 is `floor + 15`, bounded 0..15. The map producer sends it before
the reset map chunk. It shares that capture's consent binding and chosen delay.
It contains no absolute position, actor identity or actionable handle. The
native viewer stages the camera and scene until the final map chunk, and drops
all staged/shown data on a slate, clear or terminal failure. Kind 34 is the next
free nested kind (4) plus CLCAST's assigned lane offset (30).

### CLCAST character consent read

Message kind 52 is ConsentQuery, with no fields. Existing player command 503
(capability 292) replies with ConsentState for the current fenced character.
It cannot be carried by an observer envelope. Native settings read the current
revision before enabling/disabling consent. Kind 52 is next free outer kind 22
plus the assigned CLCAST lane offset 30; no new command or capability is used.

### CLCAST readiness, roster and control read

Outer kind 53 Ready has required epoch (varint field 2, >0), selected-and-enabled
chat (boolean field 3), and chosen delay (varint field 4, 0..300). It is the first
server observer frame and replaces an immediate clear when policy/epoch changes.
It clears staged/shown pictures; it contains no delayed cast content or viewer
count. Generation remains bound by the observer envelope. Unknown fields and
role/direction mismatches fail closed.

Owner-only kind 54 CasterRoster has required cast revision (varint field 2, >0)
and zero to 50 repeated field-3 byte strings, exactly 16 bytes each. Handles must
be nonzero, unique and strictly sorted. It is carried only by Foundation domain
103, delta type 32 (next free type 2 + CLCAST offset 30), maximum 1024 bytes.
Domain revision advances with both roster and live-line frames. It never carries
account IDs and is forbidden in observer envelopes.

Outer kind 55 ControlQuery has no fields and uses existing player command 502.
It returns CasterState or End for the current fenced caster session. Busy owners
remain a refused request, not an assertion that a still-running cast is stopped.
Kinds 53–55 continue CLCAST's allocation from next-free outer kind 22 + offset 30.
