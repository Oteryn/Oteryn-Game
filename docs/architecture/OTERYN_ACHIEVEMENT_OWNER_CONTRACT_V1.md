# Achievement Owner Contract V1

- Status: Contract candidate. Owner direction given 2026-09-29; needs exact-head independent review
  (catalogue identity, account-fact grant path) before it is accepted.
- Date: 2026-09-29
- Issue: #162; lane `ACHIEVEMENT` (D126)
- Implements: D126 (`reviews/OTERYN_GAME_OWNER_DECISION_BATCH_D118_D128_2026-09-28.md`) under D48 and
  D49 (`reviews/OTERYN_GAME_ACCOUNT_PROGRESS_AND_QUEST_707_DISPOSITION_DECISION_2026-09-28.md` §4.4, §4.6)
- Evidence: `imports/cipsoft-staticdata/achievements/` (368 client 15.30 records),
  `imports/tibiawiki/achievements/2026-09-29/` (572 TibiaWiki records and their join to the client records; merged
  in #1286)
- Tooling: `tools/content-schema/achievement-authoring/`
- Amended by: `OTERYN_ACHIEVEMENT_DISPLAY_CONTRACT_V1.md` (display and points rule of §2.3 and §4)
- Does not authorize: a migration, runtime or protocol code, client or website display, or populating
  `content/achievements/`

## 1. Owner

The Achievement domain is the single owner of:

- the achievement **catalogue**: the `Achievement` content family in `content/achievements/`;
- the **account fact** `AccountAchievement(account_id, achievement_key)` and the consumption of grant
  requests into it (D48);
- achievement **points** of an account.

It owns no progress counter, no quest state and no reward. It is one domain module (ADR-0019): other
domains ask it for a grant, and it never evaluates their rules.

## 2. Catalogue

One record per achievement, validated by `achievement.schema.json`:

| Field | Rule |
|---|---|
| `identity.key` | `oteryn:achievement/<slug>`, allocated once from the name at allocation with the quest tooling's `slug(name)` (`quest-authoring/ots_chests.py`), so the source-derived candidate refs `canary:achievement/<slug>` and `crystal:achievement/<slug>` of quest and interaction content bind by slug. |
| `identity.revision` | Content revision of the record. |
| `name`, `description` | Game text. |
| `grade` | 1-4. |
| `points` | Integer inside the grade's range: 1-3, 4-6, 7-9, 10; exactly 0 for a retired achievement. |
| `secret` | Whether the Reference target publishes it before it is earned. Presentation only. |
| `premium` | Reference fact that the achievement is earned in premium content. The Achievement domain does not check it; the granting content gates access. |
| `retired` | Optional, default false. A retired achievement can no longer be earned (§3) and gives no points; facts earned before stay and are shown. |
| `provenance` | The client staticdata `source_id` when there is one, and the TibiaWiki page id and revision. Never identity. |

### 2.1 Identity

- The key is the identity. A client `source_id` or wiki page is provenance only: 203 secret achievements have
  no client record, and one wiki page has only an inferred id (`563?`).
- A key is never reused and never reinterpreted. A change of meaning (a different way to earn it, a different
  grade) is a new key; a text correction or a points correction inside the grade range is a new revision of the
  same key. Retiring an achievement is a new revision of the same key.
- The key is allocated once and then never derived again: a later revision that corrects the name keeps its key,
  so identity is not bound to display text. The validator checks key format and uniqueness only;
  `allocate_key(name)` gives the key of a new record.
- A key is never removed from a catalogue; retiring keeps the record. Two records with the same key are a
  catalogue error; the validator rejects them.

### 2.2 Source precedence for population

The catalogue is populated in a separate content change, not by this contract:

- `name`, `description`, `grade`: the client staticdata record where one joins (368), else TibiaWiki (203
  secret). The 57 joined records where the wiki differs keep the client text.
- `points`, `secret`, `premium`: TibiaWiki; the client does not carry them.
- The 10 wiki anomalies of the join report are resolved by owner decision (2026-09-29):

  | Achievement | Value |
  |---|---|
  | Sculptor Apprentice | `premium` true (`Yes`) |
  | Smart Thinking, Sail Away! | `premium` true, as the wiki displays an empty value |
  | Hell Rider | 2 points (Canary `register_achievements.lua` and GuildStats agree) |
  | Taskaholic | 7 points, provisional: no source states the value; a sourced value is a new revision |
  | The More the Merrier | kept, `retired`, grade 1, 0 points: it cannot be earned since 2015 (Canary grade 1, 0 points) |
  | Achievement 563 | not in the catalogue: no name, grade or points, inferred id |

- The quest-sample ref `the_professors_nut` binds explicitly to `oteryn:achievement/the_professor_s_nut`
  (official name "The Professor's Nut"); a ref that matches no slug is listed, never guessed.

### 2.3 Compatibility (D49)

A world's catalogue entry is compatible with a recorded fact when it has the fact's key. Because a key never
changes meaning (§2.1), no other test is needed. Display and points no longer depend on a per-world subset: every
recorded fact is shown and counted from its key's record in this catalogue (owner direction 2026-09-30,
`OTERYN_ACHIEVEMENT_DISPLAY_CONTRACT_V1.md` §2.1, which supersedes the D49 display rule); the fact is unchanged.

## 3. Grant path

1. A granting domain (Quest completion, a reward claim, an interaction `Achievement` child, a counter owner at
   its threshold) records a **grant request** inside its own fenced character transaction (D48, §4.6):
   `(account_id, character_id, achievement_key, achievement_revision, earned_at, source_event)`, where
   `source_event` is the fenced character event that earned it.
2. The Achievement domain consumes the request **in the same transaction**: it inserts the
   `AccountAchievement` fact if `(account_id, achievement_key)` is absent and does nothing otherwise. The
   request row stays as the fact's durable provenance.
3. A request for a key the world's catalogue lacks fails the granting transaction closed: nothing is written.
   Content validation prevents it before runtime and covers every pinned revision of granting content in every
   family (quest, reward claim, interaction, counter owner) against the world's catalogue; because a key is
   never removed (§2.1), a later catalogue revision cannot invalidate a pinned granter.
4. A request for a retired achievement is a no-op: the Achievement domain records nothing for it and the
   granting transaction continues, so a pinned quest or counter that names a since-retired achievement still
   completes.

Same-transaction consumption is the minimum sufficient choice: no background consumer, no pending state and
no retry path. It keeps the one-row, first-commit-wins rule of §4.6 by the unique key. A granter that cannot
share the account row's transaction is superseding evidence for a later asynchronous consumer; requests
already keep the provenance that one would need.

Exclusive-choice achievements (Marid Ally, Efreet Ally) can be earned by different characters of one account
(D48); the catalogue holds no exclusivity rule.

## 4. Points

An account's points are the sum of `points` over all of its facts, each read from the catalogue record of its key
(a retired record has 0 points). They count once per account and carry no gameplay value (D48). Rankings rank accounts. How a ranking or a character
page reads them is a Platform or Atlas export question and is not decided here.

## 5. Delivery order

1. This contract with the schema and validator (this change).
2. Catalogue population in `content/achievements/` under §2.2, with its own review of the held records.
3. The migration for grant requests and `AccountAchievement` (persistence review); it proves the §4.6
   revalidation list of the account-progress decision for achievements.
4. The first granter wired end to end (a reward-claim or quest completion), then display.

## 6. Not decided

- Physical schema and migration (step 3).
- Progress counters: each stays with the domain that counts (D48) and needs its own owner.
- Protocol and client display are in `OTERYN_ACHIEVEMENT_DISPLAY_CONTRACT_V1.md`; the website and ranking export
  stay undecided.
- Declared differences in the Reference parity manifest (account scope, D48) (follow-up of the account-progress
  decision).
