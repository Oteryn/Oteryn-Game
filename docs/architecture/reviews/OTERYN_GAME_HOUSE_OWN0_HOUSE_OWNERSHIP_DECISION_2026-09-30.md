# HOUSE-OWN-0 House ownership

- Decision: `HOUSE-OWN0-PHYSICAL-HOUSE-OWNERSHIP-V1`
- Status: **CANDIDATE**. Acceptance needs exact-head validation, independent review (persistence,
  economy, security and protocol) and protected integration. It builds on BANK-0 and BANK-FEE-0
  (both on `main`), MARKET-0 (PR #1367, `CharacterInbox`) and HOUSE-CUSTODY-0, and integrates after
  #1367. Owner
  questions H1 and H2 (§14) are answered (#162 5913348961); H2a supersedes EXP-HOUSES-01 §10 until
  Premium is delivered.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: the owner's direction to start house ownership (2026-09-30); it fills, for ordinary
  physical houses, the numbers and shapes EXP-HOUSES-01 §26 leaves open, and the HOUSE-CUSTODY-0
  §3.5 gate (Ground items on house tiles)
- Builds on: EXP-HOUSES-01 (owner-accepted) §3, §4.1, §7, §8, §10, §11, §12, §14, §15, §16, §17,
  §19, §20, §23, §24 and §25; the House catalogue contract (`rent_gold`, `kind`, doors by
  position); HOUSE-CUSTODY-0 (`HouseInterior`, provenance, §3.5, §4); BANK-0 (balance, ledger,
  junior rule); MARKET-0 (`CharacterInbox`, the World job pattern, the Market credit ceiling);
  DUR-03 §15, §17, §18 and §34; the gold fee decision §4.4 (D178); the scope matrix house rows;
  owner rule 5905825574 (Global parity)
- Amends, each pending on acceptance of HOUSE-OWN-0 (#162 5912405163), in this PR:
  EXP-HOUSES-01 §26 and §10 (pointers); HOUSE-CUSTODY-0 §4 (pointer); BANK-0 §3 (the house
  ledger kinds and reference, §9 here); BANK-FEE-0 (price and rent are bank-only, outside its §3);
  DUR-03 §39.3 and the gold fee decision §4.4 (the house `FeeBurnCause` variants, admitted by the
  owner as D238).
- Runtime, migration and production authority: NONE. Each child needs its own #162 allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Implementation brief

| Child | Worker | Builds | Depends on |
|---|---|---|---|
| HOUSE-1 | hard, persistence, economy and security review | property, tile, slot, auction, bid, rent, ban and disposition tables; the bid, settlement, rent, move-out and eviction transactions; the content fence; the World jobs (§3-§9) | BANK-1; INBOX-1; HOUSE-CUSTODY-1; the house interior runtime child |
| HOUSE-ACL-1 | hard, security review | the ACL tables and revisioned edits (§10) | HOUSE-1 |
| HOUSE-WIRE-1 | impl, protocol review | capability `HOUSE_V1`, the house commands and domain: house list, bids, move-out, the House Management panel (§11) | HOUSE-1; HOUSE-ACL-1 |

HOUSE-CUSTODY-0 §4 orders ownership after the house interior runtime (HOUSE-RUNTIME-0 and its
child), so no auction opens before a house can be entered and furnished. Later, each with its own
decision: beds and Rested, Residence, guildhalls, Character Bazaar disposition, guild and wildcard
ACL patterns, direct transfer between players.

## 1. Question

How does a character get, keep and lose an ordinary physical house?

## 2. Facts

**PROVEN**

- EXP-HOUSES-01 (owner-accepted): one World-global property state per `HouseId`; owner
  `HouseId -> CharacterId`; one personal housing slot per (Account, World); **active Premium** plus
  `PhysicalHouseEligibility` at acquisition (§10, §23), and a Premium lapse does not evict; a
  public proxy auction where every bid nominates its future owner Character, is backed by reserved
  funds, is extended near the close, and settles over the effective valid-bid set (§11); rent with
  grace and a value-safe eviction under a content fence (§12.3, §14.7); forgotten items are
  neither destroyed nor gifted; the ACL is OWNER, SUBOWNER, GUEST, edited only through a House
  Management GUI (§16, §17); required evidence (§24) and conformance scenarios (§25). Numbers are
  deferred (§26); a change to a binding clause needs §30 supersession.
- House catalogue: 995 records (878 private houses, 51 shops, 66 guildhalls) with `rent_gold`,
  town, tiles and doors; a door is its house key and position; `private_house` and `shop` are
  ordinary physical houses; `rent_gold` may change with a revision.
- HOUSE-CUSTODY-0 (candidate): `HouseInterior` items each with a provenance naming the placing
  Character; no runtime grant on the house tables until the child that opens them; before any house
  becomes owned, the ownership child decides the Ground items on its tiles.
- BANK-0 (merged, #1357): roots are never deleted and their Account and World never change; credits
  stop at `BANK0-RL-01`. MARKET-0 (candidate): `CharacterInbox`; Market credits and escrow returns
  may exceed `BANK0-RL-01` up to `MARKET0-RL-10`.
- D178: every new fee source needs its own owner decision.

**CIPSOFT_OFFICIAL** (the Tibia manual, `houses.md` §5.7.2 and §5.7.3)

- Only Premium characters that have left the starter island bid; one active bid or house
  transaction per character.
- An auction starts with the first bid and lasts 7 days; proxy bidding shows the minimum needed to
  lead, which may be 0; a bid cannot be withdrawn, only lowered to the current price; the bank
  must hold the limit plus the first month's rent, reserved until outbid; the winner pays the price
  plus the first month's rent.
- Rent is monthly, in advance, from the bank; an unpaid rent starts a one-week grace with a warning;
  after it, eviction moves portable items to the Inbox and bars every character of the Account from
  renting for 30 days; the house returns to auction.
- Moving out takes effect on a chosen date 1 to 30 days ahead; items go to the Inbox.

## 3. Property state (HOUSE-1)

- **Property.** `game_house_properties`: one row per (World, house key) for every `private_house`
  and `shop` of the active catalogue, created `VACANT`. Columns: state (`VACANT`, `AUCTION`,
  `OWNED`, `MOVE_OUT_PENDING`, `DISPOSITION`, `RETIRED`), owner CharacterId and AccountId, `paid_until`,
  `grace_until`, `move_out_at`, `revision`, `acl_revision`, `content_fence_operation`. Guildhalls
  get no row.
- **Tiles.** `game_house_tiles`: (World, house key, position), materialized from the active bundle
  and refreshed with it. A guard refuses a Ground location row on any house tile in any channel
  (the HOUSE-CUSTODY-0 §3.5 gate, §8).
- **Slot.** `game_account_housing_slots`: one row per (Account, World) with `PHYSICAL_HOUSE` and
  the HouseId; no row means `NONE` (Residence is not built). A deferred guard keeps the slot and the
  owned property equal both ways.
- **Owner lifecycle.** Roots are never deleted, so a reference cannot dangle. A future deletion or
  World-transfer workflow (EXP-HOUSES-01 §15) locks the Character root, then its bid and property
  rows, and refuses while either exists.
- **Scope.** World, strong durable, one state on every channel (the scope matrix row).
- **Identity.** A property is keyed by (WorldId, house key): that is the `HouseId` of the catalogue
  §2.1, never a content revision.
- **Catalogue revisions** (catalogue §4), applied when a new bundle activates at a planned reset:
  - a new house gets a `VACANT` row;
  - a retired house in `VACANT` becomes `RETIRED` and is never auctioned again; one in `AUCTION` has
    its auction cancelled with every escrow returned by release steps (§4), then becomes `RETIRED`;
  - a house in `AUCTION` that is re-keyed or whose doors or tiles change has its auction cancelled
    the same way; it then continues as `VACANT` under its (new) key, and a re-keyed old key becomes
    `RETIRED`;
  - the reset preflight refuses a bundle that retires or re-keys a house in `OWNED`,
    `MOVE_OUT_PENDING` or `DISPOSITION`, or changes its tiles so that the HOUSE-CUSTODY-0 §3.6 check
    fails: an operator first runs a §7 disposition with cause `CATALOGUE_RETIREMENT` (no ban). Rent
    already burned is not refunded: refunding would be a new value source (D178);
  - door entries whose position no longer is a door of the house grant nothing (catalogue §2.2)
    and are deleted by the reset; a rent change applies from the next due date.

## 4. Auction (HOUSE-1)

- **Eligibility, checked at bid and at settlement.** The nominated Character belongs to the
  bidder's Account and the auction's World, is not junior (BANK-0 §4.4), and meets
  `PhysicalHouseEligibility` v1: level at least `HOUSEOWN0-RL-01` (20, tunable). The Account's slot
  is empty and it has no active house ban (§6). Premium (owner answer H2a, a supersession of
  EXP-HOUSES-01 §10 under §30): not required until the Premium consumer contract is delivered;
  from then on active Premium is required at acquisition (`NOT_PREMIUM`), and a house acquired
  before that is kept under §10.1.
- **One bid per (Account, World)** across all auctions (the slot allows one house; Tibia's rule is
  per character). At most `HOUSEOWN0-RL-05` (256) bidders per auction; a further bidder is
  `AUCTION_FULL`.
- **Bid** `{house, max}`: the acting Character is the nominated owner (EXP-HOUSES-01 §11.2;
  picking another character needs a later wire decision). The first bid on a `VACANT` house starts
  the auction: `ends_at` = +7 days (`HOUSEOWN0-RL-02`), and the house's `rent_gold` is snapshotted
  on the auction.
- **Escrow.** Every bid holds `escrow_gold` = its max + the snapshotted rent, as DUR-03 §18 value in
  custody on the bid row, debited from the bank when the bid is placed or raised
  (`HOUSE_BID_RESERVE`). A bid is `HELD`, then `WON` or `RELEASED`; a guard keeps `escrow_gold` =
  max + rent while `HELD` (whether or not the auction has closed) and 0 once `WON` or `RELEASED`.
  Each release step moves bids from `HELD` to `RELEASED` by key.
  So every bid that can win is backed (§11.3), and a bid without funds cannot enter or set the
  price. Lowering the max, down to the current price, returns the difference
  (`HOUSE_BID_RELEASE`). A bid is never withdrawn.
- **Price.** Proxy: the current price is the second-highest max plus 1, capped at the leader's max,
  or 0 with one bidder (`HOUSEOWN0-RL-03` increment 1 gold; no reserve price, as in Tibia). Equal
  maxima: the bid that first reached its current max leads. `HOUSE_QUERY` never shows a max.
- **Clock.** A bid reads `clock_timestamp()` after locking the property row and is admitted only
  before `ends_at` (`AUCTION_CLOSED` otherwise).
- **Anti-sniping** (EXP-HOUSES-01 §11.4): any admitted bid in the last 15 minutes sets `ends_at` =
  its time + 15 minutes (`HOUSEOWN0-RL-04`); Tibia has none.
- **Settlement** (a World job, §9):
  1. The first step locks the bidders' Character roots FOR SHARE in CharacterId order, then the
     property row, reads `clock_timestamp()` and proceeds only after `ends_at`. It re-reads every
     bid (at most 256) and excludes each whose subject fails any eligibility guard at that moment,
     ignoring its max. The first remaining bid by max (then the tie rule) wins, at the price
     computed from the remaining maxima. From the winner's escrow it burns the price and the rent
     (`HOUSE_PRICE`, `HOUSE_RENT`, owner answer H1) and returns the rest. The state becomes `OWNED`
     with `paid_until` = +30 days, and the slot is taken. With no valid bid, the house returns to
     `VACANT` and nothing is charged.
  2. Release steps return the escrow of every other bid, excluded ones included, at most
     `HOUSEOWN0-RL-06` (100) bids per step, keyed by (auction, step).
- **Replay.** Each bid, raise or lower is its own command with an occurrence bound 1:1 to its
  CommandRef and a SHA-256 request binding; settlement steps are keyed by (house, auction, step), so a retry replays its outcome.
- **Credits.** Escrow returns are value already owned: like MARKET-0's Market credits, they may
  exceed `BANK0-RL-01` up to the hard ceiling 9,000,000,000,000,000 (`HOUSEOWN0-RL-13`, the same
  value as `MARKET0-RL-10`). Whichever of HOUSE-1 and MARKET-1 lands first adds that ceiling to the
  balance CHECK; the other reuses it.

## 5. Rent (HOUSE-1)

- **Amount:** the `rent_gold` of the catalogue revision active at the due date (the first month
  uses the auction's snapshot); **cadence:** 30 days in advance (`HOUSEOWN0-RL-07`), keyed by
  (house, period number).
- **Collection** (a World job): at `paid_until`, the owner's (Account, World) balance is debited
  `rent_gold` (`HOUSE_RENT`, BURN) and `paid_until` moves 30 days from the previous due date.
  Coins in the backpack are never used (Tibia takes the bank).
- **Grace:** an insufficient balance sets `grace_until` = due date + 7 days (`HOUSEOWN0-RL-08`)
  and records a warning, shown on login until a mail system exists; collection is retried daily
  and once more at `grace_until`. A late payment keeps `paid_until` counted from the original due
  date. After a failed last attempt, eviction starts (§7).
- **Ban:** an eviction bans every character of the Account on that World from bidding for 30 days
  (`HOUSEOWN0-RL-09`, `game_account_house_bans`). Tibia's ban is Account-wide; per World is a
  declared difference, because the slot and the value are World-scoped.
- A lapse of Premium changes nothing (EXP-HOUSES-01 §10.1).

**Amendment (pending on acceptance of MAIL-0; `OTERYN_GAME_MAIL0_PARCELS_AND_LETTERS_DECISION_2026-09-30.md` §10).** The
owner admitted system letters (MAIL-0 Q1a): the step that sets `grace_until` also mints one stamped
rent warning letter into the owner's Inbox, keyed by (house, period), as Tibia's "warning letter
in inbox". The login warning stays until MAIL-SYSTEM-1 ships.

## 6. Moving out (HOUSE-1)

- The owner sets a date 1 to 30 days ahead (`HOUSEOWN0-RL-10`); the state becomes
  `MOVE_OUT_PENDING`; the owner may cancel before the date. At the date, §7 runs; no ban and no
  rent refund.
- A due rent inside the notice period is still collected. If its grace ends before the move-out
  date, the eviction wins and the ban applies.

## 7. Disposition: move-out and eviction (HOUSE-1)

This is the EXP-HOUSES-01 §12.3 and §14.7 ordering.

1. **Fence.** One transaction leaves `OWNED` or `MOVE_OUT_PENDING` for `DISPOSITION`, creates the
   disposition operation (`game_house_dispositions`: house, cause `MOVE_OUT`, `EVICTION` or
   `CATALOGUE_RETIREMENT` (§3; no ban), the
   property revision) and sets `content_fence_operation`. A trigger on the `HouseInterior` table
   refuses every insert, delete or update of that house while the fence is set, except through the
   disposition functions: the fence does not rely on a runtime remembering to check it. The same
   transaction records the fenced full content set: its count and a SHA-256 over the sorted
   ItemInstanceIds.
2. **Steps** (DUR-03 §34): each moves at most 100 items, in ItemInstanceId order, from
   `HouseInterior` to the `CharacterInbox` of each item's reclaim subject (never the owner by
   default), retiring the provenance. A step is keyed by (operation, step) and replays. Inbox
   deliveries are never refused, and a subject's root always exists, so no item needs other
   custody. Nothing is destroyed, and nothing stays for the next owner (a declared difference from
   Tibia's oversized furniture, required by EXP-HOUSES-01 §12.3).
3. **Release**, in one transaction after the last step has proven the house empty and the moved
   set equal to the fenced set: ACL rows removed, owner and slot released, the ban written for an
   eviction, the fence cleared, the state `VACANT`.

The steps run through SECURITY DEFINER functions; the runtime role gets EXECUTE on them and no
direct grant on the house tables (HOUSE-CUSTODY-0 §3.5).

**Storage budget** (EXP-HOUSES-01 §19). A house interior holds at most `HOUSEOWN0-RL-14` (2,000)
items, containers' contents included; the placement path (HOUSE-RUNTIME-0) refuses more with
`HOUSE_STORAGE_FULL`. Tibia bounds a house only by its tiles; this is a declared Oteryn bound
(`PARITY_PENDING`). It also bounds one disposition's Inbox deliveries (MARKET-0 §5).

## 8. Ground items on house tiles (the HOUSE-CUSTODY-0 §3.5 gate)

- Players cannot reach house tiles without house access, and HOUSE-1's tile guard refuses a new
  Ground row on a house tile in every channel. So no player-dropped Ground item can appear there.
- Map-authored items on house tiles are never pickupable (ADR-0021 §4.4).
- Settlement also checks that no Ground row lies on the house's tiles in any channel; if one does
  (from before the guard existed), the settlement waits and retries after the next `WorldReset`,
  which retires it.

## 9. World jobs and serialization (HOUSE-1)

- Settlement, release, rent, move-out and disposition steps are World jobs, run by any channel
  process of the World: candidates picked without a lock, then locked in the order below and
  re-checked; one house per transaction, at most `HOUSEOWN0-RL-11` (100) per pass; idempotent per
  key. Jobs take the recovery fence and admission relations, no session fence.
- **Lock order:** the operation occurrence; the Character roots (the acting Character's fence and
  root for a command, FOR UPDATE; the bidders' roots FOR SHARE in CharacterId order for
  settlement); the property row; the auction and its bids by id; the slot rows by AccountId; the ban
  row; for a disposition step, the items by ItemInstanceId and the Inbox counters by CharacterId;
  then the balance rows by `account_id`.
- **Fence.** Every `HOUSE_INTENT` command takes the composition decision rule 2 session fence
  (recovery fence, admission relations, the acting Character's session generation and guards) before
  the property row; World jobs take only the recovery fence and admission relations.
- No `CharacterRevision` advance: houses, bids and ledger entries are not Character state.
- **Ledger** (BANK-0 §3 amendment): a house operation table (`game_house_operations`, keyed by the
  command occurrence or the job step key); a ledger entry references exactly one of a bank
  operation, a fee record (BANK-FEE-0), a Market operation (MARKET-0) or a house operation; kinds
  `HOUSE_BID_RESERVE`, `HOUSE_BID_RELEASE`, `HOUSE_PRICE` and `HOUSE_RENT`; a deferred guard per house
  operation: its ledger deltas plus the change of bid escrow plus the burns sum to 0. Price and rent
  are bank-only, as Tibia takes them: they are not BANK-FEE-0 fees and do not use its coins-first
  rule. They are burns of house shapes, not of bank shapes, so BANK-0 §5's "no burn in any bank
  shape" still holds.
- **Classes (§17):** reservations and releases are `TRANSFER` value lines; price and rent are one
  `BURN` value line each under the house variants of `FeeBurnCause` (owner answer H1); disposition moves are item
  `TRANSFER` lines. One house event per transaction, retention under BANK-RET-0's economy profile.
- **Evidence** (EXP-HOUSES-01 §24), in the house event: the bid subject; each exclusion and its
  reason; the recomputed winner and price; the eligibility version and inputs; ownership and slot
  transitions; the fence acquisition, its fenced set and its release; every ACL revision.
- **Conformance** (EXP-HOUSES-01 §25): HOUSE-1's tests cover scenarios 5-8, 10, 11, 19-21, 23-25,
  28, 34, 35, 40 and 43; the rest belong to the runtime, Residence and Bazaar decisions.

## 10. Access list (HOUSE-ACL-1)

- `game_house_acl_entries`: (house, role `SUBOWNER` or `GUEST`, CharacterId) and (house, door
  position, CharacterId) for door lists; `acl_revision` on the property.
- The owner edits subowners, guests and door lists; a subowner edits guests only
  (EXP-HOUSES-01 §16.2). Each edit carries the expected `acl_revision`; a stale revision is
  `STALE_REVISION` and writes nothing.
- Entries name characters of the same World only; at most `HOUSEOWN0-RL-12` (200) per list. Guild
  and wildcard patterns wait for guilds.
- No spell edits the list (EXP-HOUSES-01 §17). Entry, door and kick checks belong to the house
  interior runtime, which reads the list at its revision.

## 11. Wire (HOUSE-WIRE-1)

- **Capability `HOUSE_V1`**; its number, two command types and one domain are reserved on #162 at
  allocation.
- **`HOUSE_QUERY`:** houses by town with state, current price, `ends_at` and rent; the character's
  own bid (its own max included) and house; the ACL of a house the character owns or subowns.
- **`HOUSE_INTENT`** (a oneof; an empty oneof is `REJECTED`): `bid {house, max}`,
  `move_out {date}`, `cancel_move_out`, `acl_set {role, character_ids, expected_revision}`,
  `door_set {door_position, character_ids, expected_revision}`. Results: `OK`,
  `INSUFFICIENT_FUNDS`, `NOT_ELIGIBLE`, `NOT_PREMIUM`, `SLOT_TAKEN`, `BID_EXISTS`, `BANNED`,
  `TOO_LOW`, `AUCTION_CLOSED`, `AUCTION_FULL`, `NOT_OWNER`, `STALE_REVISION`, plus the common
  results.
- The panel works anywhere; it needs no position in the house.

## 12. Rows (registered by the children before implementation)

| Row | Value |
|---|---|
| `HOUSEOWN0-RL-01` eligibility level | 20 (tunable) |
| `HOUSEOWN0-RL-02` auction length | 7 days from the first bid |
| `HOUSEOWN0-RL-03` bid increment | 1 gold; no reserve price |
| `HOUSEOWN0-RL-04` anti-sniping window | 15 minutes |
| `HOUSEOWN0-RL-05` bidders per auction | 256 |
| `HOUSEOWN0-RL-06` items or bids per step | 100 |
| `HOUSEOWN0-RL-07` rent period | 30 days, in advance |
| `HOUSEOWN0-RL-08` grace | 7 days, retried daily and at its end |
| `HOUSEOWN0-RL-09` eviction ban | 30 days, Account + World |
| `HOUSEOWN0-RL-10` move-out notice | 1 to 30 days |
| `HOUSEOWN0-RL-11` houses per job pass | 100 |
| `HOUSEOWN0-RL-12` entries per ACL list | 200 |
| `HOUSEOWN0-RL-13` balance hard ceiling for escrow returns | 9,000,000,000,000,000 |
| `HOUSEOWN0-RL-14` items per house interior (containers' contents included) | 2,000 |
| `DUR03-RL-03-HOUSE` value lines per transaction | 200; each ledger entry and each escrow change is one line (MARKET-0's rule) |
| Bid, raise or lower | 0 items, 2 value lines (reserve or release, escrow change), 1 event |
| Settlement first step | 0 items, 4 value lines (escrow fall, price, rent, return), 1 event |
| Rent charge | 0 items, 1 value line (`HOUSE_RENT`, its own burn line), 1 event |
| Release step | 100 bids, 200 value lines (escrow fall and return per bid), 1 event |
| Disposition step | 100 items, 200 location lines, 0 value lines, 1 event |

## 13. Rejected options

- **Account-only bids.** EXP-HOUSES-01 §11.2 binds each bid to its future owner Character.
- **Escrowing only the leader.** An unbacked bid could set the price (EXP-HOUSES-01 §11.3, §11.5).
- **Leaving oversized items for the next owner.** EXP-HOUSES-01 §12.3 forbids silent gifts.
- **Delivering evicted items to the owner.** EXP-HOUSES-01 §14 sends each to its reclaim subject.
- **A fence checked only by the runtime.** A trigger enforces it in the database.
- **Tibia's three houses per Account.** The accepted slot allows one per (Account, World).
- **House spells for the list.** EXP-HOUSES-01 §17.
- **Auctions before the interior runtime.** Players would pay for a house they cannot enter.

## 14. Owner questions (answered)

Owner answers, verbatim record on #162 5913348961: H1 "tak z konta" (yes, from the bank); H2 a.

**H1. Admit the house auction price and rent as gold sinks?** D178 needs an owner decision for every
new fee source (asked together with MARKET-0 Q1). a) Yes, as in Tibia (recommended); b) no.

**H2. Houses before Premium exists?** EXP-HOUSES-01 §10 (owner-accepted) binds active Premium at
acquisition, and the Game has no Premium source yet. a) Supersede §10 until Premium is delivered:
anyone who meets the eligibility of §4 may bid; b) keep §10: no house acquisitions until Premium
exists; Residence (not built yet) is the free housing path (recommended, since §10 is binding
anti-speculation).

## 15. Decision test

- **Must decide now:** YES. The owner asked for house ownership now; the interior runtime decision
  will read §7's fence and §10's list.
- **Minimum sufficient:** one property, tile and slot table, one auction with escrowed proxy bids,
  rent, one disposition path, flat ACL lists.
- **Superseding evidence:** owner answers; the interior runtime decision may refine the fence.
- **Deliberately not decided:** interior runtime, entry and doors; beds; Residence; guildhalls;
  Bazaar; ACL patterns; direct transfer between players.

## 16. Before-freeze checklist

1. **Contract amendments:** EXP-HOUSES-01 §26 and HOUSE-CUSTODY-0 §4 pointers, written "pending on
   acceptance of HOUSE-OWN-0". BANK-0, MARKET-0 and gold fee §4.4 edits follow as in the header.
2. **Serialization:** §9's lock order; World jobs lock rows before re-checking them.
3. **Restart:** every state and step is durable and keyed; ambiguous outcomes reconcile by key.
4. **Typed references:** HouseId, AccountId, WorldId, CharacterId, door position, occurrence,
   TransactionId.
5. **Wire:** §11, capability `HOUSE_V1`.
6. **Split work:** one house per transaction; at most 100 items or bids per step.
