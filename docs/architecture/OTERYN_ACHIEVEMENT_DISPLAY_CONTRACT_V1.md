# Achievement Display Contract V1

- Status: Contract candidate. Owner direction 2026-09-30, recorded verbatim on #162 (owner decision record,
  comment 5911933242); the control plane assigned D223-D228 (display-1 to display-6). Needs exact-head independent review (wire
  contract, D49 display amendment) before it is accepted.
- Owner decision record: <https://github.com/Oteryn/Oteryn-Game/issues/162#issuecomment-5911933242>
- Date: 2026-09-30
- Issue: #162; lane `ACHIEVEMENT` (D126)
- Implements: display of account achievements, the "not decided" display item of
  `OTERYN_ACHIEVEMENT_OWNER_CONTRACT_V1.md` (§6), under D48 and D49
  (`reviews/OTERYN_GAME_ACCOUNT_PROGRESS_AND_QUEST_707_DISPOSITION_DECISION_2026-09-28.md` §4.4, §4.6) and
  FND-02 (`FND-02_PROTOCOL_OTERYN_V1_CONTRACT.md`)
- Amends: the D49 display and points rule (§2.1), and §2.3 and §4 of the owner contract by reference
- Evidence: `apps/game-server/migrations/0021_account_achievements.sql` (tables, immutability triggers, grants);
  `docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json` (`command_types` 1-3 as the pattern); `content/achievements/`
  (571 records)
- Does not authorize: a `.proto` file, a registry entry, fixtures, server or client code, a runtime catalogue
  loader, a migration, or the website and ranking export

## 1. Scope

The account achievements panel of the game client: the list of achievements the player sees and the account's
total achievement points. Everything here is a read of facts that the grant path of the owner contract (§3)
already writes.

## 2. Owner decisions (2026-09-30)

Owner direction, recorded verbatim with the questions it answers in the owner decision record on #162 (see the
header); the control plane assigned D223-D228 to display-1 to display-6, in order:

1. Secret achievements not yet earned are not shown. A secret achievement is shown only once the account has
   earned it (Reference behaviour).
2. Points are shown in game as one number: the account's total achievement points. No per-grade breakdown.
3. Delivery is fetch on request. The client asks when the player opens the achievements panel. Nothing is pushed
   on login, and there is no periodic refresh.
4. The panel shows all facts the account has earned, and all of them count toward the points.
5. The panel lists earned achievements only; not-earned records are not sent (display-5, D227).
6. The reply is paged (display-6, D228).

### 2.1 D49 display amendment (decision 4, D226)

Decision 4 supersedes two display and points rules of D49 (§4.4, last bullet, and the third bullet's "compatible
catalogue entry of the world evaluating it") and the matching text of the owner contract (§2.3 last sentence, §4).
This is an owner-acknowledged supersession, not a silent change:

- Superseded: "Elsewhere the achievement is not shown" for a fact whose key a world's catalogue lacks, and "a
  point value comes from the compatible catalogue entry of the world evaluating it".
- Kept unchanged: the fact is write-once and never reinterpreted (D48, §4.4 first bullet, §4.6); the account
  scope and its declared difference from the Reference target; counting once per account; points carry no
  gameplay value; no ranking or reward may depend on the earliest earner.

Rule. The name, description, grade, points and secret flag of any recorded fact come from the Oteryn Achievement
catalogue record of its key (`content/achievements/`), not from a per-world subset. Keys are never removed from
the catalogue (owner contract §2.1), so every recorded key has a record. Therefore:

- the panel shows every fact of the account;
- `total_points` = the sum of `points` over all of the account's facts, each read from the catalogue record of its
  key (the current revision of that key; the revision recorded on the grant request is provenance only);
- a retired record has `points` 0 (owner contract §2), so a retired fact is shown and adds 0.

Conflict check. Against D48 and D49 this changes only where a fact is displayed and where its points are read.
It does not change the fact, its write-once rule, its provenance, the grant path (§4.6 revalidation list), the
account scope or the "no gameplay value" rule. The only accepted text it contradicts is the D49 sentence
superseded above; the owner accepted that on 2026-09-30. D49's motive (a world with no compatible entry must not
reinterpret a fact) still holds: nothing is reinterpreted, because a key has one meaning (owner contract §2.1).
If a later world needs a different catalogue, its display and points rule needs its own decision.

## 3. Wire

One command pair under FND-02, fixed here at contract level. The `.proto` file, the registry entry and the
fixtures come in the implementation PR (§7).

### 3.1 Placement

FND-02 §8 says later contracts register typed command payloads carried by `ClientCommand` and `CommandResult`;
`PROTOCOL_OTERYN_V1_REGISTRY.json` `command_types` 1-3 are the existing pattern (id, `owner_decision`,
`payload_schema`, `result_schema`, `max_payload_bytes`, `max_result_payload_bytes`). So the pair is one new
`command_types` entry, `ACCOUNT_ACHIEVEMENTS_QUERY`, with id 10 (reserved on #162 in the owner decision record),
not a new `message_types` entry (`message_type` 256+ is for future extension and this feature needs none).

| Part | Registry message | Direction | Phase | Sequencing class |
|---|---|---|---|---|
| `AccountAchievementsQuery` | `ClientCommand` (7) | `CLIENT_TO_SERVER` | `POST_ADMISSION` | `COMMAND_ID` (FND-02 §13) |
| `AccountAchievementsResult` | `CommandResult` (8) | `SERVER_TO_CLIENT` | `POST_ADMISSION` | `SERVER_SEQUENCED` (FND-02 §14) |

The query is a `CommandId` operation of the session (`CommandRef`, FND-02 §13), so the ordinary ordering,
idempotency and bounded-pipelining rules apply unchanged. It is not a capability and needs none (FND-02 §9: core
semantics are not capabilities; a new command type is additive under §4 same-major evolution, and an older peer
does not activate an unregistered type, §8).

### 3.2 Payloads

`AccountAchievementsQuery`: no field except `page` (`uint32`, 0 for the first page). §3.3 says why it is not
empty. The account is never in the payload.

`AccountAchievementsResult`:

- `total_points` (`uint32`): the account total of §2.1, the same in every page of one read;
- `fact_count` (`uint32`): the number of facts of the account at the time of the read (watermark, §3.3);
- `page` (`uint32`) and `has_more` (`bool`);
- `rows`, repeated `AccountAchievementRow`:

| Field | Type | Rule |
|---|---|---|
| `key` | string | `oteryn:achievement/<slug>`, at most 160 bytes |
| `name` | string | catalogue text, at most 64 UTF-8 bytes |
| `description` | string | catalogue text, at most 256 UTF-8 bytes |
| `grade` | uint32 | 1-4 |
| `points` | uint32 | the catalogue points of the key; 0 when retired |
| `earned_at` | int64 | the earning time of the grant request behind the fact |
| `secret` | bool | catalogue `secret` flag |

Which rows exist: exactly the earned facts of the account, one row per fact (decision 5). A record with no fact,
secret or not, is never sent, so a client cannot learn a secret's name or description before it is earned
(decision 1). The server applies the rule; the client never filters.

### 3.3 Bounds and cap policy

- Parent bounds (FND-02 §19): command payload 64 KiB, command-result payload 64 KiB, encoded frame 1 MiB,
  ordinary repeated collection 4096. The registry entry sets `max_payload_bytes` and `max_result_payload_bytes`
  at or below them.
- Today the catalogue has 571 records. At the text limits above a row is at most about 500 bytes, so one reply
  of all rows (up to about 285 KiB) does not fit the 64 KiB result bound, and per-row limits small enough to fit
  it (about 110 bytes) cannot hold the descriptions. The reply is therefore paged: a fixed server-side page size
  of 64 rows (at most about 32 KiB), at most 9 pages for the current catalogue (an account earning every
  record), `has_more` telling the client to ask for `page + 1`.
- The cap is a rule, not a silent truncation: the server never drops rows to fit; a page holds
  `min(64, remaining)` rows. A catalogue text longer than the per-field limits is a catalogue error caught by
  content validation; a reply that would still exceed the result bound fails closed with a registered
  operation-terminal error (FND-02 §18) and no rows. The page size and per-field limits are contract values; the
  catalogue may grow beyond 571 records without a wire change, and the implementation PR must fail its tests if
  the current catalogue does not fit.
- A `page` beyond the last page returns no rows and `has_more` false. The query carries no other
  client-chosen size, filter or key, so no peer-controlled size drives work (FND-02 §7, §19).
- Pages are not one snapshot across grants. The client fetches from page 0 each time the panel opens and, if
  `fact_count` differs between pages of one open, discards its list and fetches again. Facts are append-only
  (migration 0021), so every grant raises `fact_count`, including a grant of a zero-point (retired) record that
  leaves `total_points` unchanged but shifts rows; comparing `total_points` alone would miss it.

The owner confirmed paging (display-6, D228): an empty query would need the reply without `description` and a
client content contract, which is not chosen.

### 3.4 Order

Rows are sorted by `grade` ascending (1 to 4), then `name` by Unicode code point order, then `key` as tie-break,
so paging is deterministic. The client may regroup what it has fetched. A retired earned fact appears in its grade
position.

## 4. Server read path

1. The account is the account of the session's admitted character (FND-04 admission, on the session's current
   connection generation as for every command; FND-02 §12). It is never read from the payload, so there is no
   way to ask for another account.
2. One read over `game_account_achievements` (the facts of that account) joined to
   `game_account_achievement_grant_requests` on `(account_id, achievement_key, source_kind, source_event_id)` for
   `earned_at` (the fact row holds no time), and to the loaded Oteryn Achievement catalogue by key in memory. The
   catalogue is the validated content, loaded once. No runtime loader for `content/achievements/` exists today;
   adding it is part of the implementation PR, not this contract.
3. A fact whose key has no catalogue record cannot occur (the grant fails closed for an absent key, owner
   contract §3.3, and keys are never removed, §2.1). If it occurs it is a server integrity error: the query fails
   closed with an operation-terminal error rather than skipping the fact, because a skip would understate the
   points.
4. Grants. Migration 0021 grants `SELECT, INSERT` on both tables to `oteryn_game_runtime` (the `GRANT SELECT,
   INSERT ON game_account_achievement_grant_requests, game_account_achievements TO oteryn_game_runtime`
   statement) and `SELECT` to `oteryn_game_control`. The query needs `SELECT` only, so it needs no migration and
   no grant.

### 4.1 Why no character fence

The character fence (session-generation, `CharacterRevision`, the character root row lock) protects character
writes. This path writes nothing and reads no character state: it reads two append-only tables whose rows are
immutable, since migration 0021 rejects `UPDATE`, `DELETE` and `TRUNCATE` on both by trigger. Any committed read
is therefore a correct subset of a monotonically growing set. The worst case is a stale read that lacks a grant
committing a moment later, which the next open of the panel shows. The query cannot cause a lost update, a double
grant or a fence bypass. The transport and command rules of FND-02 §12-§13 still apply to the request itself.

## 5. What the client shows

- One list in the order of §3.4 with, per row: name, description, grade, points, and the earning date when
  earned.
- Earned achievements only (decisions 1 and 5); the server enforces it.
- One number, the total points (decision 2); no grade breakdown.
- The panel fetches when it opens and shows what it fetched until it is reopened; no push, no timer (decision 3).

## 6. Not decided and excluded

- The website and ranking export (Platform and Atlas), and how a character page reads points.
- Per-character display of achievements.
- Notifications when an achievement is earned.
- Progress counters (owner contract §6).
- Layout, art, localisation and the date format of the client panel.

## 7. Delivery order

1. This contract.
2. The implementation PR: the `.proto` file (`docs/contracts/protocol-oteryn/v1/`), the registry `command_types`
   entry with its bounds and `owner_decision`, the golden and independent raw-byte fixtures, the malformed and
   oversize corpus and the round-trip property tests required by FND-02 §22, the runtime catalogue loader, the
   server handler and its PostgreSQL tests (own account only, no unearned record returned, retired fact shown
   with 0 points, a fact under a key absent from a per-world subset shown and counted, paging bounds).
3. The client panel.

FND-02 sections this contract relies on: §4 (same-major schema evolution), §7 (Serialization and IDL), §8
(Envelope and registries), §9 (Capability model), §12 (Connection-generation fencing), §13 (`CommandId`), §14
(Server-authoritative sequence), §18 (Error model), §19 (Hard resource limits), §22 (Independent wire evidence).
