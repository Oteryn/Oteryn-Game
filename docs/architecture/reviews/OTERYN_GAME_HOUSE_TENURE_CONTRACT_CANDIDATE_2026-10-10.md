# HOUSE-TENURE-0 House tenure contract candidate

- Decision: `HOUSE-TENURE0-CONSOLIDATED-HOUSE-TENURE-V1`
- Status: **CANDIDATE**. Docs only. It binds the accepted house contract by reference and
  re-decides one accepted rule: owner answer Q2a lets Aleta spells edit house lists. It
  supersedes every accepted clause that forbids that (§10 Q2a lists them), and needs the review
  that supersession requires. It also
  records the live gap on `main`, names one storage conflict and asks the owner the questions
  where the HOUSE-1 control-plane order (#1622) differs from the accepted contract (§8). The
  owner answered them on 2026-10-10 (D972, §10).
- Task: `OTV2-20261010-house-1-contract` (HOUSE-1, control plane #1622)
- Live base: `main` at `348b2b76`
- Builds on (all on `main`):
  - EXP-HOUSES-01 (owner-accepted), with §10 superseded by HOUSE-OWN-0 H2a;
  - HOUSE-OWN-0 (ACCEPTED by ACCEPT-SOCIAL-MAP-0, #1771): ownership, auction, rent, grace,
    eviction, move-out, ACL, wire and rows;
  - HOUSE-CUSTODY-0 and HOUSE-RUNTIME-0 (ACCEPTED by #1771); BED-0 (CANDIDATE);
  - SOCIAL-MAP-PACKETS-1 (#1773) §1.3, §1.4, §1.9, §2.3 and §2.4: the HOUSE-1a and HOUSE-1b
    packets;
  - ARCH-HOUSE-RT-INBOX-PACKETS-1 (#1776) and its fix (#1780): HOUSE-RUNTIME-1a/1b/1c, INBOX-1;
  - the House catalogue owner contract (995 records, `rent_gold`, doors by position);
  - BANK-0 and BANK-FEE-0 (bank-only price and rent);
  - the Tibia manual notes `docs/reference/tibia-manual/houses.md` §5.7.2 and §5.7.3;
  - House client 15.33 source audit (PR #1946, open; evidence only): the 995 static records
    match 15.30, one layout (Lakeside Mansion, 55015) changed.
- Amends on acceptance (owner answer Q2a, §10), and nothing else:
  - EXP-HOUSES-01 §17, §25.21, §27.8 and the §32 line `ACL PLAYER UX`;
  - HOUSE-OWN-0 §10 (the bullet "No spell edits the list") and §13 ("House spells for the list").
  Runtime, migration, registry, protocol and production authority: NONE.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## 1. Question

The control plane asked for a contract for house ownership, rent, buying, access lists,
persistence ownership, failure modes, conformance and a slice plan. What does the Game still have
to decide, and what does it only have to build?

## 2. Finding

**PROVEN.** Every topic of the order is already contracted and accepted (§3). The mechanic is
missing on `main` because its packets are not built, not because it is undecided (§4). Writing a
second contract would create a competing authority. This candidate therefore:

1. indexes the binding clauses per topic (§3);
2. records what `main` has and lacks (§4);
3. collects failure modes and conformance per slice (§5, §6);
4. gives the critical path (§7);
5. records one storage CONFLICT that blocks HOUSE-1a and ACL-1 as written (§4.2) and the
   order-versus-contract differences (§8) as owner questions.

## 3. Contract by topic (binding sources)

| Topic | Binding clause | Content |
|---|---|---|
| Identity | HOUSE-OWN-0 §3; catalogue §2.1 | property keyed by (WorldId, house key) = `HouseId`; World-global, one state on every Channel; `private_house` and `shop` only; guildhalls by GUILD-0 |
| Ownership | EXP-HOUSES-01 §7, §8; HOUSE-OWN-0 §3 | owner `HouseId -> CharacterId`; one personal housing slot per (Account, World); states `VACANT`, `AUCTION`, `OWNED`, `MOVE_OUT_PENDING`, `DISPOSITION`, `RETIRED` |
| Acquisition | EXP-HOUSES-01 §11; HOUSE-OWN-0 §4; packets §1.4 | public proxy auction only: 7 days from the first bid, escrow of max + first rent from the bank, 15 minute anti-sniping, settlement re-checks eligibility (level 20, not junior, slot empty, no ban; Premium per §1.4) |
| No direct market | EXP-HOUSES-01 §11.6 | no player-to-player sale at launch; a direct market needs owner supersession |
| Rent | HOUSE-OWN-0 §5; owner answer H1 | catalogue `rent_gold` every 30 days in advance, debited from the (Account, World) bank balance as `HOUSE_RENT` burn; coins in inventory or depot are never used |
| Grace and eviction | HOUSE-OWN-0 §5, §7; EXP-HOUSES-01 §12 | 7 day grace, retried daily and at its end; eviction under a database-enforced content fence; items to each item's reclaim subject's Inbox; 30 day ban per (Account, World) |
| Move-out | HOUSE-OWN-0 §6 | notice 1 to 30 days; cancellable; no ban, no refund |
| Access lists | EXP-HOUSES-01 §16, §17; HOUSE-OWN-0 §10 | OWNER, SUBOWNER, GUEST, door lists; revisioned edits (`STALE_REVISION`); 200 entries per list; GUI and Aleta spells write one list (§10 Q2a) |
| Entry, kick, leave | HOUSE-RUNTIME-0 §5; HOUSE-OWN-0 §11 amendment | entry checks read the ACL at its revision; `kick`, `leave` intents |
| Wire | HOUSE-OWN-0 §11 | capability `HOUSE_V1`, `HOUSE_QUERY`, `HOUSE_INTENT` (bid, move_out, cancel_move_out, acl_set, door_set, kick, leave) and their result codes |
| Persistence ownership | ADR-0004; HOUSE-OWN-0 §7, §9; EXP-HOUSES-01 §22 | Game PostgreSQL is the only authority for property, bids, escrow, rent, ACL and interior; writes through SECURITY DEFINER functions; runtime gets no direct grant on house tables; Platform holds no house truth; Atlas may only consume a Game-owned projection |
| Fencing | HOUSE-OWN-0 §9 | every `HOUSE_INTENT` takes the session-generation fence of the acting Character; World jobs take the recovery fence and admission relations only; fixed lock order |
| Numbers | HOUSE-OWN-0 §12 | `HOUSEOWN0-RL-01` to `-15` |

## 4. Live state on `main` (348b2b76)

### 4.1 Present and absent

| Piece | State | Evidence |
|---|---|---|
| House catalogue (995 records) | on `main` | `content/houses/houses-*.json` (PROVEN) |
| HOUSE-CUSTODY-1 interior custody and provenance | on `main` | migration `0025` (PROVEN) |
| House spell ACL, ownership row, editors | on `main` | migration `0038`, `durability/house_spell_acl.rs` (PROVEN) |
| World house instance binding | on `main` | migration `0047`, `durability/world_house_instance.rs` (PROVEN) |
| SCOPE-HANDOFF-1 house scope | on `main` | migration `0074`, `house_scope/` (PROVEN) |
| BANK-1 balance and ledger | on `main` | migrations `0071`, `0072`, `durability/bank.rs` (PROVEN) |
| INBOX-1a CharacterInbox | on `main` | migration `0076` (PROVEN) |
| HOUSE-RUNTIME-1a, 1b, 1c | absent | no migration, module or task record (DERIVED) |
| PREM-WIRE-1, CHAR-POSITION-1, MAP-OVERLAY-1c | absent | no task record or merged PR found (DERIVED) |
| HOUSE-1a, HOUSE-1b, HOUSE-ACL-1, HOUSE-WIRE-1 | absent | no `game_house_properties`, no `HOUSE_V1` (PROVEN) |
| BANK-NPC-1 (bank deposit by NPC dialogue) | absent | named in NPC packets only (DERIVED) |

### 4.2 CONFLICT: two owners for house ownership and ACL

**PROVEN.** Migration `0038` (candidate HOUSE-ALETA-1, spell track) created
`game_house_ownership` (owner CharacterId, `ownership_revision`, acquisition receipt; control
role write, runtime read), `game_house_acl` (`list_id` -1 guest, -2 subowner, positive = door;
at most 100 members) with editors and receipts. Migration `0047` references
`game_house_ownership` by foreign key. Its header says the owner authorized Aleta spells alongside
the GUI (`OTERYN_WORLD_HOUSE_INSTANCE_CANDIDATE_V1.md`).

**PROVEN.** The accepted packets plan a different owner: HOUSE-1a creates
`game_house_properties` with owner, state and `acl_revision`; HOUSE-ACL-1 creates
`game_house_acl_entries` with 200 entries per list. HOUSE-OWN-0 §10 and EXP-HOUSES-01 §17 say no
spell edits the list. Both key the house the same way: `house_key` is the catalogue
`identity.key` (`oteryn:content.house.<slug>`, catalogue §2.1), which is the house key of
HOUSE-OWN-0 §3. The conflict is the competing tables, not the key. No accepted document names
`0038` or `game_house_ownership`.

**Consequence.** Building HOUSE-1a and HOUSE-ACL-1 as written leaves two ownership truths and two
ACLs in one database, which breaks "one World-global property state per `HouseId`"
(EXP-HOUSES-01 §4.1). HOUSE-1a must not be allocated before owner question Q1 (§8) is answered,
and not before the exit gate of ARCH-HOUSE-RT-INBOX-PACKETS-1 §1.5 holds.
The resolution (owner answer Q1a, §10) keeps one owner per fact:

- `game_house_properties` (HOUSE-1a) is the property and owner authority. `0038` and `0047` do
  not stay unchanged. `game_house_ownership.owner_character_id` is `NOT NULL`, and the `0047`
  instance binding has a foreign key to that row and an immutability trigger, so the row can be
  neither deleted nor left naming a former owner. HOUSE-1a therefore migrates the pair. Either the
  binding's foreign key moves to `game_house_properties`, or the ownership row stays as the
  binding anchor and its owner becomes nullable with release in the same transaction. Whichever
  it picks, `house_spell_acl.rs` reads its owner from the property authority. A `VACANT` or
  released house then grants no owner access, and no former owner keeps rights.
- HOUSE-ACL-1 migrates `game_house_acl` to the accepted ACL model, not just its bound, and creates
  no second ACL table. The accepted model has:
  - doors keyed by catalogue position `[x,y,z]` (catalogue §2.2), not by numeric `list_id`;
  - one property-wide `acl_revision` (HOUSE-OWN-0 §10), not one revision per list;
  - guild entries and character exclusions (HOUSE-OWN-0 GUILD-0 amendment);
  - 200 entries per list (`HOUSEOWN0-RL-12`).

  Existing rows move to that shape. The GUI and Aleta then write one list through one revision.

## 5. Failure modes (consolidated)

| Case | Required outcome | Source |
|---|---|---|
| Bid without funds for max + rent | `INSUFFICIENT_FUNDS`, nothing written | HOUSE-OWN-0 §4 |
| Second bid on the (Account, World), slot taken, ban active | `BID_EXISTS`, `SLOT_TAKEN`, `BANNED` | §4, §5 |
| Bid after `ends_at` | `AUCTION_CLOSED` (clock read after the property lock) | §4 |
| Leader ineligible at settlement | excluded; winner and price from remaining bids; no valid bid = `VACANT`, no charge | §4; EXP-HOUSES-01 §11.5 |
| Crash or retry in settlement or release | keyed step replays; one owner, one payment | §4, §9 |
| Rent short | grace + warning; a payment in grace keeps the original period; no double charge | §5 |
| Grace expires | eviction under fence; ban; house `VACANT` | §5, §7 |
| Writer races the eviction | trigger refuses interior writes while the fence is set | §7 |
| Crash mid-disposition | keyed steps resume; moved set equals fenced set before release | §7 |
| Stale ACL edit | `STALE_REVISION`, nothing written | §10 |
| Stale session generation on a command | refused by the fence before the property row | §9 |
| Catalogue revision retires an owned house | reset preflight refuses; operator disposition first | §3 |
| Authority outage | housing mutation fails closed | EXP-HOUSES-01 §20, §25.25 |
| House won before HOUSE-1b exists | first pass charges once, no eviction for the gap | packets §1.3 |

## 6. Conformance (EXP-HOUSES-01 §25) per slice

| Slice | Scenarios |
|---|---|
| HOUSE-RUNTIME-1a/1b/1c | 1-4, 26 (interior parts); 19 (1b revocation and revalidation of present occupants); 25 (interior parts) |
| HOUSE-1a | 5, 6, 7, 8 (two physical houses), 28, 34, 35; 25 (settlement fails closed when Character or Premium authority is out); plus the packet §2.3 tests |
| HOUSE-1b | 10, 11, 40; 25 (rent, move-out and eviction fail closed); the voluntary relinquishment and eviction parts of 43; plus the packet §2.4 tests. It supplies the disposition primitive that 23, 24 and the rest of 43 use |
| HOUSE-ACL-1 | 19 (the revision bump and stale-edit refusal), 20; 21 as amended by §10 Q2a; 25 (ACL edits fail closed) |
| HOUSE-WIRE-1 | 20, 21 and 25 at the wire |
| Later decisions (Residence, Bazaar, transfer, Character lifecycle, World transfer) | 9, 12-18, 22, 23 (Character finalization), 24 (World transfer), 27, 29-33, 36-39, 41, 42; 43 for Bazaar buyer release, Residence replacement, physical house to Residence, Character deletion and World transfer |

## 7. Slice plan and critical path

1. HOUSE-RUNTIME-1a, then 1b (needs MAP-LOAD-1), then 1c (needs CHAR-POSITION-1): enter and
   furnish a house.
2. PREM-WIRE-1: the Premium gate the settlement calls.
3. Owner answer Q1 (§10) and the §1.5 exit gate (HOUSE-RUNTIME-1b and 1c both accepted with every
   exit-dependent item met, not merely merged), then **HOUSE-1a**: properties, tiles, slot,
   auction, escrow, settlement (hard worker; persistence, economy and security review).
4. **HOUSE-ACL-1** and **HOUSE-1b** in parallel (HOUSE-1b also needs MAP-OVERLAY-1c): ACL;
   rent, grace, move-out, eviction, catalogue revisions.
5. **HOUSE-WIRE-1**: `HOUSE_V1`, house list, bid, move-out, panel, rent warning at login.
6. Playability: a player can bid only with gold in the bank. **BANK-NPC-1** (or a bank wire) is a
   playability dependency, not a contract dependency: HOUSE-1a reads BANK-1's balance and
   needs no NPC. It must merge before HOUSE-WIRE-1 is called playable.
7. Later, each with its own decision: direct transfer (Q3), Residence, Bazaar disposition, beds
   (BED-0), guildhalls (GUILDHALL-1), Lakeside Mansion 15.33 layout (only if a 15.33 map update
   is authorized).

The NPC track has no house role: neither Tibia nor the accepted contract sells a house through an
NPC (houses.md §5.7.2; EXP-HOUSES-01 §11). See Q4.

## 8. Owner questions

**Q1. One owner for house ownership and ACL (§4.2).**
a) HOUSE-1a's `game_house_properties` is the authority and keeps `game_house_ownership` in step in
the same transactions; HOUSE-ACL-1 adopts `game_house_acl` (bound raised to 200) instead of a new
table (recommended: one truth, no rewrite of merged code);
b) HOUSE-1a migrates `0038`/`0047` onto new tables and retires them (more churn, same result);
c) keep both stores (rejected: two truths per `HouseId`).

**Q2. Aleta spells for house lists.** `0038` says the owner authorized Aleta alongside the GUI;
EXP-HOUSES-01 §17, §25.21, §27.8, §32 and HOUSE-OWN-0 §10, §13 (all accepted) forbid spell
edits, and no supersession is recorded.
a) Record the authorization as an EXP-HOUSES-01 §17 supersession under §30: GUI and Aleta write
the same revisioned list (recommended: Tibia parity, code already on `main`);
b) keep §17: Aleta stays unavailable and `0038`'s editor path is removed later.

**Q3. Direct house transfer between players** (Tibia §5.7.2 b: owner sets date, target and price;
target accepts and pays from the bank at server save). EXP-HOUSES-01 §11.6 defers it.
a) Keep deferred until HOUSE-WIRE-1 is playable, then a HOUSE-TRANSFER-0 decision (recommended:
smallest path to a playable house);
b) write HOUSE-TRANSFER-0 now as a §11.6 supersession, built after HOUSE-1b.

**Q4. Buying through an NPC or a command** (the #1622 order says "buying via NPC/command").
a) Keep the accepted auction through `HOUSE_INTENT` and the panel; no NPC sale (recommended:
EXP-HOUSES-01 §11 and Tibia parity);
b) add a fixed-price purchase of a `VACANT` house as a §11 supersession;
c) a test-world-only operator command to grant a house, never on production Worlds.

**Q5. Rent source** (the order says "bank/depot gold").
a) Bank only, as owner answer H1 already recorded (recommended);
b) bank, then depot coins (a new value path; supersedes H1).

## 9. Decision test

- **Must decide now:** only Q1 blocks the next build (HOUSE-1a); Q2-Q5 confirm or change the
  scope of later slices.
- **Minimum sufficient:** no new contract; an index, one conflict and five questions.
- **Deliberately not decided:** everything accepted in §3; Residence, Bazaar, beds, guildhalls.

## 10. Owner answers (D972, 2026-10-10)

The owner chose `1a 2a 3a 4a 5a`:

- **Q1a.** `game_house_properties` (HOUSE-1a) is the property and owner authority. It keeps
  `game_house_ownership` in step in the same transactions. HOUSE-ACL-1 adopts `game_house_acl`
  as the one ACL store, raises its member bound to 200 (`HOUSEOWN0-RL-12`) and creates no
  `game_house_acl_entries`. HOUSE-1a and HOUSE-ACL-1 migrate the `0038`/`0047` tables to the
  accepted model as §4.2 states. HOUSE-1a may be allocated once this record is on `main` and the
  exit gate of ARCH-HOUSE-RT-INBOX-PACKETS-1 §1.5 holds. That gate needs HOUSE-RUNTIME-1b and 1c
  accepted with every exit-dependent item met. Both are absent today (§4.1).
- **Q2a.** Superseded under EXP-HOUSES-01 §30, as far as they forbid Aleta list edits:
  - EXP-HOUSES-01 §17, §25.21, §27.8 and the §32 line `ACL PLAYER UX`;
  - HOUSE-OWN-0 §10 ("No spell edits the list") and §13 ("House spells for the list").

  Aleta spells and the GUI write the same revisioned list with the same permissions (EXP-HOUSES-01
  §16.2), and a stale edit returns `STALE_REVISION`. Everything else in those clauses stays
  binding:
  - the GUI/panel stays a supported player path;
  - no other text command administers an ACL;
  - local client manipulation still cannot change the effective ACL (§25.20);
  - entry, door and kick checks stay with the interior runtime.

  Scenario 21 becomes: an Aleta edit goes through the same authority, revision and permission
  checks as the GUI, and no other text command mutates an ACL.
- **Q3a.** Direct transfer stays deferred (EXP-HOUSES-01 §11.6) until HOUSE-WIRE-1 is playable.
  It then needs its own HOUSE-TRANSFER-0 decision.
- **Q4a.** Acquisition is the accepted auction only, through `HOUSE_INTENT` and the panel. There
  is no NPC sale and no grant command.
- **Q5a.** Rent comes from the bank only (owner answer H1). Coins in a depot or inventory are
  never used.

Q4a and Q5a also fix HOUSE-1a's scope. Acquisition is the auction only. Escrow (max + first rent)
and the first `HOUSE_RENT` burn come from the bank only (packets §1.3). No later answer reopens
HOUSE-1a's acquisition transaction.
