# Soul Seals fights v1 — T114

Owner decision B, 2026-11-09 authorizes delivery, including the Game authority.
Soulseals are the existing Character Task Board quest track, not purchased coins.
No Platform commercial balance is read or written by this mechanic.

## Reference and rules

Reference revision: Canary `47dfd51f45280a59a1d3e50ba7edd573d7234446`.
The following are source behaviour, not official-client protocol adoption:

- `data/modules/scripts/taskboard/soulpit.lua:63-98`: the server supplies the
  available races and grants a single-use, session-bound entrance authorization.
  `settings.lua:192-195` gives it a ten-second lifetime. Restart does not restore it.
- `soulpit.lua:110-119`: valid Bestiary difficulty is an integer from zero through
  five; the cost is `(difficulty + 1) * 10` Soulseals.
- `soulpit.lua:122-181`: select a configured race, require the full balance, start
  a **solo** fight and spend once. Failed start/persistence must refund the debit.
  Oteryn implements this more strictly as one rollback-safe Type2 transaction.
- `data/modules/scripts/taskboard/catalog.lua:125-137` supplies a configured,
  deduplicated, sorted valid-race roster. Its default roster is empty.
- `data-otservbr-global/lib/others/soulpit.lua:31-84,132-140` and
  `scripts/quests/soulpit/soulpit_fight.lua:37-106` supply existing staging,
  level eight entry, the encounter's four waves, ten-minute limit and return.

Source URLs are under
`https://raw.githubusercontent.com/opentibiabr/canary/47dfd51f45280a59a1d3e50ba7edd573d7234446/`.
TibiaWiki MediaWiki API requests for `Soul_Seals` and `Soulpit` on 2026-11-09
returned HTTP 402; no protection bypass or page content is claimed.

## Minimal Oteryn adapter choices

**owner decision B 2026-11-09, reference silent:** the reference supplies neither
an implemented `startSoloFight` adapter nor a nonempty default roster. Use the
existing qualified Soulpit creature roster, intersected with the current native
creature/AI closure and valid Bestiary difficulty. Never create a Soul Core or an
item-craft receipt to represent a seal payment. Reuse the existing Soulpit
encounter/Instance handoff; transfer only the initiating player on staging slot
zero. Other occupied staging slots do not join this solo fight. Existing core
entry keeps its group rules and item debit unchanged.

The server derives the species, cost and entry facts. A client supplies only the
server catalogue race index and opaque authorization. All live checks run again
before entry. Insufficient funds, stale authority, missing capacity, invalid
species, dead/uncontrolled actors or a failed successor cause no seal loss.
A committed command replays its retained handoff without another debit, even
when its one-use authorization has expired. A conflicting replay is refused.

The existing Character root/lease/session-generation and World/Channel/Instance
fences apply to the balance, receipt, encounter and fresh destination session.
The quest debit, audit receipt, encounter, participants and shared admission
commit atomically. A retained refusal never becomes a successful debit silently.
No lifecycle or completion outcome is accepted from the client.

## Wire and client

Protocol-oteryn only, existing `TASK_BOARD_V1` capability 72; new command 6120.
No capability or offer number is added. The bounded catalogue is paginated and
carries canonical names, difficulty, cost, confirmed balance and entry rules.
The dialog selects a confirmed entry, shows the seal cost and required staging,
and submits its authorization through the same real command caller. Server
refusal and session end clear pending state; the UI never predicts a debit.

## Request

Command 6120 is under capability 72. Version byte 1 followed by tag 0 and
`after:u16le` queries a catalogue page; tag 1 followed by the 16-byte opaque
nonzero authorization and `race:u16le` requests entry. Exact lengths are 4 and
20 bytes. Race/cursor resource bound is 4096. No price or species key is client
input. A new query replaces the session's previous authorization.

## Result

The version-1 bounded binary reply is defined by
`crates/protocol-oteryn/src/task_board_soulpit.rs`: status Catalogue/Started/Refused,
optional confirmed balance, optional authorization, next cursor, total fights,
server entry rules, refusal reason, and up to twenty strictly ordered entries.
Each entry carries race index, name, Bestiary stars and its seal cost. Catalogue
names are at most 128 UTF-8 bytes; reasons at most 256; the whole reply at most
8192. Non-catalogue replies have no entries, cursor or authorization. A refusal
may withhold balance rather than invent a zero. The single-use authorization is
valid for ten seconds and bound to the exact session/connection/lease/scope and
active content digest. A successful result reports the retained post-debit
balance. Progress/completion remains exclusively the existing Soulpit owner.

## Authority qualification

The authorization boundary checks identity/binding (token, Character, session,
World, Channel/Instance, content), current authority (connection, lease and scope
ownership generations plus the independently resolved live gameplay fence),
and time (the server's monotonic exclusive deadline). Missing, replaced, stale,
mismatched, expired and reused evidence is refused. The
`t114_authorization_is_one_use_and_binds_every_live_fence_and_content` test changes
one bound fact at a time; refreshing also revokes an older permit. Client clocks
and prices are not authority inputs, so manipulating those is not an applicable
server authority operator.

At the durable boundary, the shared entry owner retains its existing recovery,
node, root, equipment and participant session-generation checks. Seal funding
adds the current balance, reference-derived cost and solo-slot proof inside that
same transaction. The `t114_soulseal_*` PostgreSQL cases run through admitted
command frames and the runtime database role: successful admission and retained
replay; insufficient funds; insufficient runtime capacity; and a deferred failure
after the debit/successor preparation, proving all payment effects roll back.
The existing core-funded shared-entry/lifecycle cases remain applicable and must
retain their original behavior. Commercial balance substitution is not applicable:
this path has no Platform or purchased-coin balance access.
