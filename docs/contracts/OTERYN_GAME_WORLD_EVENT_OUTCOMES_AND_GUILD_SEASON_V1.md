# Oteryn Game World Event Outcomes and Guild Season v1

- Status: **CANDIDATE**, revision 1, PLAT3-CONTRACTS. Docs/fixtures only; no live feed,
  Game score authority implementation or Platform enablement is claimed.
- Contract ID: `oteryn-game-world-event-outcomes-and-guild-season-v1`.
- Game baseline: `28c7d9ab7ff85baef47c9947554f623ff8b2a87c`.
- Authority/producer: `Oteryn/Oteryn-Game`; consumer: `Oteryn/Oteryn-Platform`.
- Parents: `OTERYN_GAME_PUBLIC_PROJECTIONS_V1.md` (PP) §§2–7,
  `OTERYN_GAME_GUILDS_V1.md` (guild identity/revision),
  `game-events/v1/foundation.proto` and GAME_EVENT_FOUNDATION_REGISTRY.json (ANL).
- Platform evidence read at `efdd058839cca2b922391e9a8323236e94e78834`:
  `docs/architecture/OTERYN_PORTAL_RUST_PLAN.md`:558,837 (T94, G-12, T88/T89), and
  `docs/contracts/OTERYN_V2_WORLD_DAILY_FAMILY_CONTRACT_PROPOSAL.md` revision 2
  §§1, 5–7: the proposed daily family supplies no accepted prediction outcome source.
  This is that separate candidate source, not retroactive acceptance of the daily proposal.

## 1. Scope and actual Game sources

No stakes, currency, fees, transferable rewards, payouts or Wallet/Store commands.
Platform owns free prediction submissions and display-only prediction points; Game publishes
outcomes and guild standings. Neither prediction points nor standings authorize gameplay value.
No reward, paid boost or premium advantage is implied by T89 dependency.

Source citations below are relative to the pinned Game baseline, not moving main:

| Kind | Existing code/content evidence | What can become authority |
|---|---|---|
| RAID | `crates/gameplay/encounter-rules/src/raid/schedule.rs`:10 (Running/Done/Cancelled), :17 (firing/reset/deadline/content binding); `crates/gameplay/encounter-rules/src/raid/lifecycle.rs`:95 (end policies), :117 (decision) | Only a committed admitted run and its final native evidence, never scheduler metadata alone |
| BOSS | same lifecycle file :104 (BossDeathOrTimeout), :154 (bosses-dead test) | A separately bound actual boss-death receipt; Done/timeout alone cannot mean boss killed |
| DAILY_WORLD_GATE | `content/encounters/world-changes/fury_gates/selection-source.json`:20 (fury_gates), :22 (reset trigger); `crates/gameplay/world-change/src/daily.rs`:94 (reset-bound daily decisions), :107 (roll) | Committed selected gate/realization, not reference content or a prediction of a random trigger |
| WORLD_CHANGE | `crates/gameplay/world-change/src/daily.rs`:94 (Thawing, Their Master's Voice, trader, Grimvale, Flower Month); `crates/gameplay/world-change/src/scheduled.rs`:6 (reset gathering); `crates/gameplay/durability/src/durability/world_event.rs`:17 (immutable reset record), :42 (startup refuses missing decision) | Durable reset decision plus required realization evidence; public announcement is not settlement evidence |

The Fury Gates content still lists unresolved realization bindings at :12; daily boolean rolls
are not that gate's runtime receipt. No current code is asserted to emit this feed or guild
season scores. Kinds require an accepted source mapping and emitter before advertisement.
Future/random/skipped raid slots in the daily proposal are not scheduled prediction markets.
An event whose result is already public/derivable (such as alternating gathering epochs) is
ineligible for new predictions. Never disclose a hidden random draw or future firing to make
a fixture work. An eligible run needs a publishable advance window and a lock before the
underlying decision can be learned; otherwise publish only a closed/void record.

## 2. Transport, envelopes and bounds

Reuse PP's Game-initiated HTTP/1.1/TLS1.3 mTLS publisher pattern: no inbound Game listener,
broker, browser call or live query on page render. Dedicated outcome/season purpose identity,
separate from public projection, Character and Bazaar identities; configured source namespace
and route allowlist. Connect 1 s, handshake 2 s, exchange 5 s, one in-flight per family;
429/503 retry identical bytes with backoff <=10 s. All responses Cache-Control: no-store.

| POST path on Platform | operation | Max request bytes |
|---|---|---|
| `/internal/v1/game-public/world-event-outcome` | PublishWorldEventOutcomeV1 | 16384 |
| `/internal/v1/game-public/guild-season` | PublishGuildSeasonV1 | 262144 |
| `/internal/v1/game-public/league-watermark` | PublishLeagueWatermarkV1 | 512 |

These are new candidate operations, not existing routes. Version is 1 for this feed, independent
of CC's command version. Snapshot exact order starts `contract_version,operation,
source_authority,world_id`, then subject `event_id` or `season_id`, then
`projection_epoch,projection_revision,source_observed_at`, then body `event` or `season`.
Bodies are complete 0..1 lists; [] withdraws display, **never** implies void or zero standings.
No unknown/duplicate/null members, nesting <=8, raw UTF-8/no escapes or insignificant whitespace.
IDs are canonical non-nil lowercase UUIDv7, counters/times canonical uint64 decimal strings.
Times are positive Unix seconds UTC; score/count fields explicitly allow zero. Keys are ASCII
`[a-z0-9][a-z0-9._:/-]{0,127}`. Hashes are lowercase SHA-256 hex of exact canonical bytes.
No AccountId, CharacterId, ChannelId, session, position, player text or unrevealed draw.

Reuse PP ordering/content/response semantics: accepted/superseded JSON with contract_version 1;
400 malformed/over bound, 401 wrong identity, 409 equal pair differing content, 429 limited,
503 disabled/unavailable; empty failure bodies, responses <=256 bytes. source_observed_at is
assigned with the revision and replayed, excluded from content as PP §4.0. Higher epoch
invalidates the whole feed; an unknown schema disables dependent predictions/standings.

## 3. Authoritative event record

The event list's sole record has exactly these members, in order:

| Member | Meaning and bounds |
|---|---|
| kind | RAID, BOSS, DAILY_WORLD_GATE or WORLD_CHANGE |
| source_key | Stable approved Game content key; not free text |
| occurrence_id | Game-issued UUIDv7 identifying one native run/reset decision; distinct from ANL envelope EventId |
| ruleset_revision | Immutable approved interpretation key |
| content_sha256 | 64 hex content artifact binding |
| scheduled_start,scheduled_end,lock_at | Positive second strings; lock_at <= start < end; immutable once published |
| options | 2..16 distinct keys, ASCII byte-sorted, closed result domain |
| state | SCHEDULED, LOCKED, SETTLED, VOID or CLOSED |
| outcome | [] unless SETTLED or CLOSED; then exactly one option key for settled lineage, [] for void lineage |
| decided_at | [] before terminal decision; terminal one-element Unix-second list |
| correction_of | [] initially; on correction one prior event-record SHA-256 |
| reason_code | One of the closed reasons below |
| evidence_sha256 | [] before terminal; terminal one Game source-evidence record SHA-256 |

Empty/one-element lists follow PP absence conventions; null or zero sentinel times are not used.
`event_id` is the prediction question identity, Game-assigned once per approved question/native
occurrence. Repeating an occurrence/question cannot mint a second market. The evidence record
binds event/occurrence/world/kind, scheduled window, options, lock, ruleset/content, terminal
source receipt digests, result and recovery generation. Hashes are authenticated attestations
only when received from Game's purpose identity; no untrusted hash or Portal observation can
supply a missing source receipt. Audit payload retains exact published event-record bytes.

| Transition | Required authority/consumer behavior |
|---|---|
| New → SCHEDULED | Publish before lock_at; approved advance schedule and unknowable future result; prediction gate opens only after receipt of this fresh complete record |
| SCHEDULED → LOCKED | At lock_at Game publishes locked; Platform closes independently by its clock even if that publication is late |
| LOCKED → SETTLED | Final durable native result, exactly one option and decision/evidence; no scheduled-end timeout inference |
| SCHEDULED/LOCKED → VOID | Definitive cancellation, no eligible realization, invalid source or irrecoverable unknown result; void all predictions, no score |
| SETTLED/VOID → CLOSED | Game's final freeze, same outcome lineage and evidence, reason FINAL_FREEZE; no future v1 correction |
| SETTLED/VOID → corrected SETTLED/VOID | Higher revision, correction_of equals current record hash, fresh terminal evidence, reason CORRECTION; no reopening of submissions |

Reasons: NONE (nonterminal), NATIVE_RESULT (first settlement), CANCELLED,
SOURCE_UNAVAILABLE or INVALID_SOURCE (first void), CORRECTION, FINAL_FREEZE.
All source/question fields stay immutable after first publication. Correcting the question,
options or lock requires voiding it and a new event_id whose lock is still in the future;
existing submissions never migrate. A correction before freeze re-evaluates display-only
prediction points by event_id/revision, replacing the prior contribution once. A correction
of a correction must bind the immediately previous hash. CLOSED is immutable; an owner repair
must publish a separately versioned supersession and keep dependent serving unavailable until
accepted. No silent rewrite of final history or payouts exists.

Platform uses a trusted clock with uncertainty 1 s and accepts submissions only if
`now + uncertainty < lock_at`, current state SCHEDULED, fresh watermark and no epoch/gap alarm.
Submission acceptance/locking is one local Platform transaction; received_at is authoritative,
not client time. Equality closes. Late SCHEDULED delivery does not reopen. Missing result at
scheduled_end leaves LOCKED/pending or explicit Game VOID; never settle by absence. A boss
can time out without being killed; source mapping must distinguish that result or void it.
World-level aggregation across Channels must be fixed in the approved ruleset; no arbitrary
first-Channel winner. A source that cannot prove its aggregation withholds settlement.

## 4. Guild season challenge standings

Subject is `(world_id,season_id)`, immutable season identity and half-open [starts_at,ends_at).
One complete season list record, exact member order:
`ruleset_revision,starts_at,ends_at,freeze_at,state,challenges,standings,computed_at,
source_complete_through,correction_of`.
Times positive seconds; starts_at < ends_at <= freeze_at. State OPEN, FROZEN or VOID.
computed_at >= source_complete_through. correction_of is [] or one prior season-record hash.
Freeze_at is earliest possible final freeze, not a clock-triggered guarantee.

challenges: 1..32 records, sorted by challenge_id UUID bytes, exact order
`challenge_id,source_key,scoring_revision,max_score`. IDs are Game-issued UUIDv7, keys approved
immutable rules, max_score positive uint64 string. Only accepted Game evidence contributes;
no Platform-authored score, public highscore inference, reward claim or paid progression.
The owner approves each scoring/source rule before the season starts. This contract does not
invent scores from raids that only have lifecycle metadata.

standings: 0..1000 unique guild records, sorted by the total order below, exact members
`guild_id,guild_name,scores,total_score,rank`. Guild identity/world/name follow GUILDS and PP
wire/privacy rules. Guild name is 1..64 bytes NFC with PP restrictions. scores contains exactly
one `challenge_id,score` per challenge in challenge order. Scores are canonical 0..max_score
uint64 strings. total_score is their checked sum, <=uint64 max. No float or negative score.
Overflow, excess guild count or incomplete source refuses the whole snapshot, never truncates.
Empty standings certifies known no eligible guilds, only with complete source coverage.

Ranking total order: total_score descending; then score vector lexicographically descending
in sorted challenge_id order; then guild_id ascending as UUID bytes. rank is the one-based
ordinal, no shared rank; all tie-breaks are published and independent of delivery time/name.
Membership attribution uses Game guild membership at each source occurrence's committed cut;
a later join/leave/rename does not rewrite attribution. Deleted/disbanded guilds retain their
season identity/score with a Game-approved public tombstone name until season retention ends;
if privacy cannot permit that display, withhold the row's display and the dependent ranking,
not silently renumber. No member/account roster is exported.

Game deduplicates contribution by `(season_id,challenge_id,source occurrence,guild_id)`;
corrections replace that contribution once, no additive duplicate. All contributions use
session-generation fenced gameplay commits and the current domain/recovery fences. The
standings publisher cannot write gameplay state. Each snapshot is one consistent cut of all
challenges/guilds; partial pages cannot be installed as a complete season.

OPEN snapshots may correct scores with a higher revision and correction_of immediately prior
record hash. Initial snapshot has []; unchanged score refresh need not create a correction.
Game freezes only after ends_at, freeze_at, complete source coverage through ends_at and all
accepted correction windows closed. FROZEN repeats immutable scores/rules/ranking and final
computed cut at later transport retries; no source contribution can enter it again. VOID is
explicit Game invalidation, no rewards/score claims. A post-freeze defect requires owner-approved
new version/supersession, not mutable FROZEN content. freeze_at is not an unbounded retention
promise and cannot substitute for final evidence.

## 5. Evidence and conformance

The normative companion [fixture specification](fixtures/world-event-guild-season/README.md) fixes
recovery, retention, conformance and rollout.
No runtime conformance or enablement is claimed.

## Open questions for owner

- Which specific advance windows/outcome options and aggregation rules are safe to publish?
  Random/hidden or already-known decisions cannot open predictions by default.
- Approve season calendar, challenge/scoring rules, guild eligibility/disbanded display and
  finite correction windows. No supported guild score source is asserted by present code.
- Accept immutable final freeze and separately versioned repair, source mapping/PKI owners,
  measured production limits, and provide the real review PR number for the pending lock.
