# Kills listener V2 (PLAT-WIRE-1)

Status: proposed listener contract, awaiting Platform agreement; **not an assertion
that a kills producer/listener already exists**. The closest Game → Platform wire is
`character_deaths` in [public projections V1 §4.3](../../OTERYN_GAME_PUBLIC_PROJECTIONS_V1.md),
which is still a candidate. V1's latest-20 replacement list cannot support a lossless
kills listener. `world_totals` V1 supplies public death totals independently; it must
not be treated as individual kills. This task implements that totals producer;
activating this V2 event producer is subsequent work after agreement.

## Reuse the existing transport

Use the existing public-projection mTLS purpose and
`POST /internal/v1/game-public/character-deaths`, with
`operation: "PublishCharacterDeathsV2"`, `contract_version: 2`, family
`character_deaths`. This is a proposed version on the existing route, not a new
inbound Game endpoint, WebSocket, polling API or admin-control operation. Reuse the
existing 3-second exchanges, bounded responses, definite holds/transient backoff,
public epoch fence and operator resync model. Platform returns `accepted` only after
its transaction commits; duplicate identical deliveries are acknowledged. Unsupported
V2 is a definite refusal, never a silent downgrade to the lossy V1 list.

## Event identity and durable listener watermark

`event.schema.json` is the exact public-safe event body. `event_id` is the existing
DEATH-0 occurrence UUIDv7, `character_id` the already-public victim Character UUIDv7,
`world_id` the receipt's death World, `level` its level_before, `pvp` its durable
classification, `occurred_at = floor(committed_at / 1000)`. No account, position,
private text or inferred killer/cause is exposed. Killer names require the V1
commit-time capture amendment; there is no such producer source on this base.
All deaths are events; a PvP-only kills view filters `pvp=true` after ingest.
Public serving also follows V1's current-epoch profile/disclosure gates: a missing
or empty victim profile hides event details; ingest is never ownership authority.

The proposed producer must insert its public event and assign `sequence` in the
**same transaction as the death receipt**. Sequence is per `(source_authority,
projection_epoch)` across all Worlds, starts at 1 and has no committed holes. Use a
transactional counter row locked until commit, not PostgreSQL nextval, UUID order,
wall-clock order or max(receipt ID). Include a stable `event_id` for deduplication
across replays. A restore changes epoch under the existing external fence.

The listener stores `(source_authority, projection_epoch, sequence)` as its durable
watermark, atomically with the materialized event. It advances only over the
contiguous committed prefix. Duplicate equal sequence and body is idempotent;
equal sequence with different body is `409 conflict` and an alarm. Use the existing
`200 accepted` after commit; `200 superseded` only for an identical stored event or
an obsolete epoch, never for an unseen event below a newer sequence. A newer epoch invalidates
the old read model and requires rebuild; it cannot inherit the old sequence.
The existing time-based liveness watermark remains separate: its
`complete_through` must not pass an undelivered event. It is never a replay cursor.

## Replay and gaps

On startup/reconnect and after delivery uncertainty, Platform supplies its durable
cursor to the existing **operator resync workflow**, not a new page-serving query.
The producer requeues retained events with sequence greater than that cursor in
ascending batches (at most 32 events, each at most 2048 bytes), on the same push
route. Initial cursor is `(current epoch, 0)`. Listener acknowledgement follows its
own durable ingest, not notification of downstream subscribers. Delivery ack state
must remain distinct from the replay log; deleting outbox rows cannot delete history.

On sequence > watermark+1, do not advance the cursor, publish a count, or drop the
missing event. Mark the view stale and request replay from the last contiguous
cursor through the operator workflow. Reorder/duplicate deliveries are harmless;
buffer at most 32 events, returning the existing `503 unavailable` while a gap
remains (no acknowledgement of unapplied events), then refuse/retry without buffering
more. Replay that fills the gap drains the buffer in sequence order. If cursor is ahead of the producer,
wrong epoch, or older than retained history, explicitly require rebuild. Retention
must never silently shift a cursor; no automatic event retention period is agreed.

## Consistent full rebuild

Stop this publisher (gameplay may continue). In one PostgreSQL REPEATABLE READ
snapshot read the epoch and committed counter H, and export all public event rows
with sequence <= H ordered by sequence, in bounded pages on that same transaction.
The first page's H is the rebuild fence; independently opened page transactions are
invalid. Exported artifacts carry epoch, H, page index/count and SHA-256 digest;
transfer them through the existing operator resync/artifact workflow, not a new
public endpoint. Snapshot history must include backfilled historical receipts;
assign their sequences deterministically before admitting live events.

Platform validates every page and digest and stages a fresh model. It atomically
replaces the model and cursor with `(epoch,H)` only after all pages are complete,
including an explicitly empty snapshot with H=0. Resume/requeue events > H and
consume the contiguous suffix; concurrent deaths after the cut therefore arrive
exactly once. Hold the view unavailable until both replay and the time watermark
are current. Never mix pages/epochs, clear a live model at rebuild start, or advance
to a producer high-water value without applying the corresponding rows.

Open agreements: Platform V2 acceptance/rollout; retention and artifact/resync
handoff; whether "kills" means PvP deaths or needs durable killer attribution.
