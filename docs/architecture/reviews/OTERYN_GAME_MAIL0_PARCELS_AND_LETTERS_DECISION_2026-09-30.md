# MAIL-0 Parcels and letters

- Decision: `MAIL0-PARCELS-AND-LETTERS-V1`
- Status: **CANDIDATE**. Acceptance needs exact-head validation, independent review (persistence,
  protocol, security and privacy) and protected integration. It builds on DEPOT-0, MARKET-0
  (`CharacterInbox`), HOUSE-OWN-0, CHAT-0, ITEM-MOVE-WIRE-0 and -1 and MAP-WIRE-1, and integrates
  after them. Owner questions Q1 to Q3 (§14) are answered (2026-09-30, #162): system letters are
  delivered (Q1a); a junior sends and receives letters only, no parcels (Q2b); a parcel addressed
  to a junior returns `UNKNOWN_RECIPIENT` (Q3a).
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: the owner's direction to build the mail system now, with full Tibia Global parity
  (2026-09-30); HOUSE-OWN-0 §5's "shown on login until a mail system exists"
- Builds on: the scope matrix row "Mail/parcels: World service/domain, World, idempotent
  transactional, cross-channel delivery, shared"; DUR-03 §5.2, §10, §16, §17, §28, §32, §38, §39
  and §46; DEPOT-0 (one world-wide depot, 2b); MARKET-0 §5 (`CharacterInbox`, its counter, its
  out-shapes); HOUSE-OWN-0 §5 and §7; BANK-0 §4.3 and §4.4; the composition decision §3 and its
  Market amendment; CHAR-NAME (`0022`); CHAT-0 §5; ITEM-MOVE-WIRE-0 §4.1; ITEM-MOVE-WIRE-1 §5;
  MAP-WIRE-1 §6; WO-0 (`container_fixture`); B3 (`GAMEITEM01-PLACEMENT-DEPTH` 1); FND-ID-01
  (`MailId`); D208 (new value sources need an owner decision); owner rule 5905825574 (Global parity)
- Amends, each pending on acceptance of MAIL-0, in this PR: DUR-03 §38 (the mail row); MARKET-0
  §5 (mail deliveries and parcel trees in the Inbox); HOUSE-OWN-0 §5 (the rent warning letter);
  CHAT-0 §5 (the mail notice); the composition decision (before its §7).
- Runtime, migration and production authority: NONE. Each child needs its own #162 allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Implementation brief

| Child | Worker | Builds | Depends on |
|---|---|---|---|
| MAIL-CONTENT-1 | content lane | parcel, letter, label and stamped definitions (§3.1), the stamp rule, the base map's mailboxes as `container_fixture` bindings, the system letter texts | WO-1; MAP-LOAD-1 |
| MAIL-TEXT-1 | hard, security and privacy review | the item text store and the text write transaction (§4) | ITEM-MOVE-1; MAIL-CONTENT-1 |
| MAIL-1 | hard, persistence review | mail operations, letter posting, the stamp transform, the mail event, the Inbox mail ceiling and the posting rate (§5, §6, §8) | INBOX-1; ITEM-MOVE-2a; MAIL-TEXT-1; MAIL-CONTENT-1 |
| MAIL-WIRE-1 | impl, protocol review | capability `MAIL_V1`, the `MAILBOX` destination, the text view and write command, the mail notice (§9) | ITEM-VIEW-1; MAP-WIRE-2; MAIL-1 |
| MAIL-PARCEL-1 | hard, persistence review | parcel posting as a container tree, parcel trees in the Inbox, the parcel-child out-shape, the junior parcel refusal (§7) | MAIL-1; the bags child of BAGS-0 |
| MAIL-SYSTEM-1 | hard, persistence review | system letters minted into the Inbox, first the rent warning (§10) | MAIL-1; HOUSE-1 |

BAGS-0 (containers with contents in the main backpack, B3 RL-05 above 0) is not written yet. A
parcel cannot be filled without it, so it is the next decision parcels need. Later, each with its
own decision: house mailboxes (Store upgrades, with the house interior runtime), posting from the
ground, Store-bound items, nested bags inside parcels.

## 1. Question

How does a player send a letter or a parcel to another character of the same World, and how do
system messages reach a player?

## 2. Facts

**PROVEN**

- The scope matrix: mail is a World service, idempotent and transactional, delivered across
  channels. FND-ID-01 names `MailId` as a Game identity (UUIDv7).
- DUR-03 §38: DUR-03 owns mail item custody and conservation; the mail domain owns address and
  delivery. §10: moving a container changes only the root's location. §16.2: `PRESERVE_INSTANCE`
  is one input, one output, under a versioned rule. §39.1 admits one-item shapes only.
- DEPOT-0 (2b): one depot per Character, reachable from every town. MARKET-0 §5: the
  `CharacterInbox` family (Character + World, whole items without contents), its counter
  (`MARKET0-RL-06`, 100,000, refusing only new offers and accepts), and two out-shapes. Other
  decisions' deliveries are never refused.
- HOUSE-OWN-0 §5: an unpaid rent records a warning "shown on login until a mail system exists".
- B3: `GAMEITEM01-PLACEMENT-DEPTH` is 1; only empty containers enter the main backpack.
  PLAYER-TRADE-0, MARKET-0 and HOUSE-CUSTODY-0 refuse containers with contents.
- `0022`: one global name namespace keyed by `name_key`; the root holds the name, the World and the
  lifecycle. No operation changes them yet (BANK-0), but rename and terminal deletion are planned
  (`0022`) and World transfer is admitted (Character Authority contract §9). BANK-0 §4.3 resolves a
  transfer recipient through it; CHAT-0 §5 answers an unknown name and a name of another World alike
  (`NOT_ONLINE`).
- GAME-INTERACTION-01 successor §19.4: durable writable text has no owner and stays blocked until
  a named accepted contract exists. The DUR-03 maxima decision excludes free text from audit.
- MAP-WIRE-1 §6: base items the client can act on carry a per-session `map_item_handle`, mapped to
  (`overlay_incarnation`, position, `placement_key`, `tile_revision`). WO-0: mailboxes are
  `container_fixture` world objects.
- Content: `i3503` parcel (container, capacity 10), `i3504` stamped parcel, `i3505` letter,
  `i3506` stamped letter, `i3507` label, `i3501` and `i3508` mailbox, `i23399`-`i23402` house
  mailboxes; none is materializable yet and no `readable` semantics are set. 21 NPC trade services
  sell a parcel (15 gold), a letter (8) and a label (1); no NPC buys them.

**CIPSOFT_OFFICIAL** (the Tibia manual, `world.md` §5.2.3, `houses.md` §5.7.3, `controls.md`)

- Post offices sell parcels and letters. The sender writes the recipient's exact name on a label
  and puts the label inside the parcel; posting is placing the item on a mailbox; delivery reaches
  the recipient's depot from any town. An invalid address makes the item reappear on the mailbox.
- The Inbox receives parcels and letters, house-loss items and Market purchases.
- Letters and labels record the last editor's name and the edit time, visible to the recipient.
- Unpaid rent: "owner gets a warning letter in inbox" with a one-week grace.
- Junior accounts: no house, no Market selling, no bank transfers. Whether other limits exist is
  open (`world.md`, open questions).

**OTS_HYPOTHESIS_ONLY** (Canary `04b83b51`)

- Only a parcel or a letter without a Store owner can be posted (`mailbox.cpp:142-144`). A tile
  with a mailbox passes a dropped item to it (`tile.cpp:1695-1699`).
- The recipient is the first line of the letter's text, or of the first label in the parcel with
  text, trimmed; no town is read (`mailbox.cpp:121-140`).
- The recipient may be offline; the item moves into its Inbox with no limit, then becomes item id
  + 1 (stamped), keeping a letter's text, writer and date (`mailbox.cpp:91-116`). An online
  recipient next to a depot sees "New mail has arrived." (`player.cpp:1589-1593`).
- A label holds 79 characters, a letter 1,999 (`items.xml:10974-10989`).

## 3. Items and mailboxes (MAIL-CONTENT-1)

### 3.1 Definitions

- Parcel and stamped parcel: containers of 10. Letter and label: `readable`, `writable`,
  `write_policy: rewrite`, `max_characters` 1,999 and 79 (`MAIL0-RL-01`, `-02`). Stamped letter:
  `readable`, not writable. All five definitions (parcel, stamped parcel, letter, label, stamped
  letter) are pickupable and materializable; MAIL-CONTENT-1 must list the stamped letter explicitly,
  since MAIL-SYSTEM-1 MINTs it directly (§10).
- **Stamp rule.** A versioned content rule maps parcel to stamped parcel and letter to stamped
  letter (DUR-03 §16.2, `PRESERVE_INSTANCE`). Only an unstamped parcel or letter can be posted, so a
  received parcel must be repacked, as in Tibia.
- Letter, parcel and label stay on sale at the existing post office NPCs (NPC-0); no new source.

### 3.2 Mailboxes

- The base map's mailboxes (`i3501`, `i3508`) are bound as `container_fixture` world objects with a
  mailbox binding. The binding holds nothing: a mailbox is never a location (§5).
- House mailboxes (`i23399`-`i23402`) are Store upgrades inside houses and wait for the house
  interior runtime.

## 4. Writable text (MAIL-TEXT-1)

MAIL-0 is the named owner of durable writable text for the letter and label definitions only.
Other writable items (books, blackboards) keep GAME-INTERACTION §19.4's blocker.

- **Store.** `game_item_texts`: `item_instance_id` (PK, FK to item instances with RESTRICT), text,
  `writer_character_id` (NULL for a system letter), `written_at`, `text_revision`. The text belongs
  to the item, so it travels with every TRANSFER and survives the stamp (the identity is kept).
- **Text.** UTF-8, at most the definition's `max_characters` in Unicode scalar values and 4 bytes
  each; line feeds allowed; other control characters `REJECTED`. An empty text deletes the row.
- **Write.** One transaction under the composition rule 2 fence and `character_root`: the item is a
  direct entry of the writer's main backpack (any reachable item after BAGS-0), writable, and not
  stamped. It replaces the text and sets the writer and time. At most `MAIL0-RL-06` (2) writes per
  character per second.
- **Class.** A typed item state change outside DUR-03 value audit (DUR-03 §39). It moves no value
  and needs no audit line. No `CharacterRevision` advance (rule 1).
- **Privacy.** Text is player content, as private chat is: never in logs, audit events or
  analytics (FND-02 §20). It is shown only to a session that reaches the item. Text rows of retired
  items are deleted daily.
- **Market.** An item with text is not in its default state, so MARKET-0 §3.1 already refuses it.

## 5. Posting a letter (MAIL-1)

- **Command.** Command 9 with the source handle of a main backpack direct entry and the new
  destination `MAILBOX {map_item_handle}` (§9). The handle must resolve on this channel to a
  mailbox-bound placement that the overlay does not hide (DUR-03 §32, MAP-WIRE-1 §6).
- **Reach.** The drop reach of ITEM-MOVE-WIRE-1 §5: within 15 tiles, same floor, visible, with line
  of sight (a throw, as in Tibia; `PARITY_PENDING`).
- **Address.** The first line of the letter's text, trimmed. It must pass `game_character_is_name`;
  its `name_key` resolves through the `0022` reservations to a root of the sender's World. The town
  line is ignored: under 2b every town opens the same Inbox. A second line is never checked.
- **Refused, writing nothing** (the item stays in the backpack):
  - no text or an empty first line: `NO_ADDRESS`;
  - an unknown name, a name of another World, or a recipient failing §6's root check:
    `UNKNOWN_RECIPIENT`, one answer for all, so a reply never reveals where a name exists (CHAT-0's
    rule);
  - not an unstamped letter or parcel, a corpse or Ground source, or an equipped item:
    `NOT_MAILABLE`;
  - a recipient Inbox at the mail ceiling (§8): `RECIPIENT_INBOX_FULL`;
  - above the posting rate (§8): `EXHAUSTED`.
  Tibia drops a misaddressed item on the mailbox tile; keeping it in the backpack is a declared
  difference (`PARITY_PENDING`) that creates no Ground item.
- **Allowed recipients.** Any character of the World: oneself, another character of the same
  Account, offline characters, characters on another channel. Mail moves no bank value, so
  BANK-0 §4.3's same-Account refusal does not apply. A junior character (BANK-0 §4.4) sends and
  receives letters (owner Q2b); parcels are refused to and from it (§7).
- **Delivery is instant**, in the posting transaction, as in Tibia and Canary:
  1. the letter leaves the sender's backpack entry and becomes a new `CharacterInbox` entry of the
     recipient (TRANSFER);
  2. the same item takes the stamped letter definition (TRANSFORM, `PRESERVE_INSTANCE`); its text,
     writer and time are kept.
- **Operation.** `game_mail_operations`: `mail_id` (`MailId`, UUIDv7), the occurrence bound 1:1 to
  the CommandRef, the TransactionId, sender and recipient CharacterIds, the root item, the mailbox
  `placement_key`, the item count, and a SHA-256 binding of request and outcome. The same
  occurrence replays the first outcome; a changed binding conflicts (the BANK-0 §3 pattern).
- **Notice.** After commit, the posting node sends a mail notice through the CHAT-2 World relay
  (§9). It is best effort: a lost notice changes nothing, since the mail is in the Inbox.
- **Revision.** No `CharacterRevision` advance for either Character (rule 1).

## 6. DUR-03 lines, cause and lock order (MAIL-1)

- **Lines.** Letter posting: one TRANSFER (two location lines: the backpack entry out, the Inbox
  entry in) and one TRANSFORM line on the same item. No value line, no MINT, no BURN.
- **Cause.** Closed `MailCause {Post {mail_id} | ParcelChildOut {occurrence} | SystemLetter
  {kind, occurrence_key}}`, each keyed by its occurrence.
- **Conservation.** Per mail operation: the set of ItemInstanceIds before equals the set after;
  units per item are unchanged; the only definition change is the stamp rule's.
- **Fences.** Rule 2 for the sender with its pending CommandRef. The recipient is not fenced: no
  runtime owns its Inbox, and its Inbox counter row lock serializes Inbox writes (MARKET-0 §5).
- **Recipient root.** Every delivery into an Inbox (a posting, a system letter) locks the
  recipient's `character_root` `FOR SHARE` in the delivering transaction and checks, under that
  lock, that the root is live, is of the delivering World, and, for a posting, still has the
  addressed `name_key`. A rename, a terminal deletion or a World transfer updates the root, so it
  waits for the delivery or the delivery sees its result. A posting that fails the check is
  `UNKNOWN_RECIPIENT` and writes nothing (§5); a system letter mints nothing (§10). The
  reservation alone never proves the recipient: a released or held key may name a renamed or
  deleted Character.
- **Lock order** (composition rule 4, as MARKET-0 extends it): the recovery fence and admission
  relations; the mail occurrence; the sender's fence checks; the `character_root` rows by
  CharacterId (the sender's as rule 2 takes it, the recipient's `FOR SHARE`, one row when they are
  the same); the items by ItemInstanceId; the container-slot row; the Inbox counters by CharacterId.
- **Event.** One mail event per operation in a mail outbox, typed by its `MailCause`: `Post`
  carries `mail_id`, the sender CharacterId, the recipient, the item lines and the stamp;
  `ParcelChildOut` its occurrence, the Character and its lines; `SystemLetter` the shape of §10.
  No text. Retention under BANK-RET-0's economy profile (as HOUSE-OWN-0).
- **Supersession.** For the mail shapes only: the §39.1 exclusions of transform combined with
  transfer and of multiple touched items (parcels), and the §39.1 and §39.3 source and destination
  limits, within §11's rows. Every other obligation is unchanged.

## 7. Parcels (MAIL-PARCEL-1, after BAGS-0)

- **Tree.** A parcel is posted with its contents: at most `MAIL0-RL-03` (10) direct children,
  none with contents of its own. Nested bags inside a parcel are a declared difference
  (`PARITY_PENDING`) until BAGS-0 admits deeper nesting and this row is amended.
- **Address.** The first label among the parcel's children in entry order whose text is not
  empty; its first line resolves as in §5. No such label is `NO_ADDRESS`.
- **Juniors** (owner Q2b). A junior character (BANK-0 §4.4) neither posts nor receives a parcel.
  A junior sender's parcel is refused as `NOT_MAILABLE`; a parcel addressed to a junior recipient
  is refused, checked under §6's recipient root lock. Either refusal writes nothing and the parcel
  stays in the backpack. The sender of a parcel to a junior recipient sees `UNKNOWN_RECIPIENT`
  (owner Q3a, §14), so the reply never reveals that the character is a junior.
  Juniors still receive letters and system letters.
- **Shape.** One TRANSFER of the parcel root from the backpack entry to a new Inbox entry, and the
  stamp TRANSFORM of the root. The children keep their `Container {parent}` location (DUR-03 §10);
  they are locked and checked but not moved. The operation records the tree: its item count and a
  SHA-256 over the sorted ItemInstanceIds (the HOUSE-OWN-0 §7 idiom), and the commit checks both.
- **Inbox tree** (MARKET-0 §5 amendment). An Inbox entry may be a stamped parcel with children.
  The counter counts every item of the tree (1 + children), as the depot counts contents.
- **Out.** A child of an Inbox parcel moves as one item to a new main backpack entry (a new
  one-item TRANSFER, the depot view open). An empty stamped parcel leaves by MARKET-0's shapes. A
  parcel with contents leaves whole only by BAGS-0's tree shape; never into a depot box until bags
  in boxes are decided.

## 8. Anti-abuse (MAIL-1)

- **Posting rate.** At most `MAIL0-RL-04` (1) posting per character per second (an Oteryn bound;
  Canary has none, `PARITY_PENDING`).
- **Inbox mail ceiling.** A posting is refused when the recipient's Inbox counter plus the
  posted items would exceed `MAIL0-RL-05` (50,000). Canary has no limit. Without one, mail could
  push a victim's counter past `MARKET0-RL-06` and lock its Market (MARKET-0 §5). Half the Market
  ceiling keeps the Market usable. Other deliveries stay never refused.
- **Cost.** Every posting consumes a bought letter or parcel (8 or 15 gold at NPCs), so a flood
  pays for every item.

## 9. Wire (MAIL-WIRE-1)

- **Capability `MAIL_V1`**, requiring capability 4 and `MAP_STATE_V1`; its number, one command
  type and one state domain are reserved on #162 at allocation. Without it, a mailbox handle is not
  a destination and USE on a letter is `NOTHING_TO_USE`.
- **Command 9** gains `MAILBOX {map_item_handle}`. Results: `OK`, `NO_ADDRESS`,
  `UNKNOWN_RECIPIENT`, `RECIPIENT_INBOX_FULL`, `NOT_MAILABLE`, `EXHAUSTED`, plus the existing ones
  (`TOO_FAR`, `BLOCKED`, `STALE`). At most 4 bytes.
- **Text view.** USE field 2 on a letter, label or stamped letter the session reaches opens it as a
  non-durable view in the new domain `ITEM_TEXT`: handle, text, `max_characters`, writable, writer
  name (the current name, never the CharacterId) and time. One text view at a time; it closes like
  a container view.
- **Command `ITEM_TEXT_WRITE {handle, text}`**: results `OK`, `NOT_WRITABLE`, `TOO_LONG`,
  `EXHAUSTED`, plus the common results.
- **Mail notice.** CHAT-2's relay gains one payload kind: a mail notice keyed by the recipient
  CharacterId, with no text. The node holding the recipient's session shows "New mail has
  arrived." when the character stands next to a depot locker (Canary parity).
- **Inbox view.** MARKET-0 §10's Inbox view shows stamped parcels as containers; opening one
  shows its children in domain 11, and command 9 takes a child as source (§7).

## 10. System letters (MAIL-SYSTEM-1)

- System letters are delivered (owner Q1a): a system letter is a stamped letter minted into the
  recipient's Inbox: one MINT line under `MailCause::SystemLetter {kind, occurrence_key}` with a
  text row whose writer is NULL (shown as "Royal Tibian Mail"). The text is a content template
  with its parameters. `SystemLetterKind` is closed: each kind is requested only by the step its
  owning decision names, and a new kind needs an amendment of this section.
- **Operation.** `game_mail_system_letters`, a row type of its own, never a
  `game_mail_operations` row: `kind` (closed `SystemLetterKind`), `occurrence_key` (typed per
  kind), the TransactionId, the recipient CharacterId, the outcome (closed: `DELIVERED` with the
  minted ItemInstanceId, or `RECIPIENT_UNAVAILABLE` when the §6 recipient root check fails, minting
  nothing), the template id and revision, and a SHA-256 binding of request and outcome. The
  primary key is (`kind`, `occurrence_key`); the same occurrence replays the first outcome and a
  changed binding conflicts. It has no `MailId`: a `MailId` names a player posting only. It has no
  sender CharacterId: its sender kind is `SYSTEM`.
- **Event.** `SystemLetter`: `kind`, `occurrence_key`, sender kind `SYSTEM` with no sender
  CharacterId, the recipient, the outcome and, when delivered, the MINT line. No text.
- **First kind: the rent warning.** HOUSE-1's step that sets `grace_until` also mints
  `HouseRentWarning`, whose `occurrence_key` is (HouseId, rent period), so a retry mints nothing
  twice.
- System letters are never refused by the mail ceiling; at most one per house and period bounds
  them (the HOUSE-OWN-0 §7 rule for deliveries).
- Until MAIL-SYSTEM-1 ships, the login warning of HOUSE-OWN-0 §5 stays.

## 11. Rows (registered by the children before implementation)

| Row | Value |
|---|---|
| `MAIL0-RL-01` letter text | 1,999 characters, 7,996 bytes |
| `MAIL0-RL-02` label text | 79 characters, 316 bytes |
| `MAIL0-RL-03` parcel contents | 10 direct children, none with contents |
| `MAIL0-RL-04` postings per character | 1 per second |
| `MAIL0-RL-05` recipient Inbox counter after a posting | 50,000 |
| `MAIL0-RL-06` text writes per character | 2 per second |
| Letter posting | 1 item, 2 location lines, 1 transform line, 0 value lines, 4 work units, 1 event |
| Parcel posting | 11 touched items (1 moved, 10 checked), 2 location lines, 1 transform line, 0 value lines, 14 work units, 1 event |
| Parcel child out | 1 item, 2 location lines, 3 work units, 1 event |
| System letter | 1 item, 1 location line (MINT), 2 work units, 1 event |
| `DUR03-RL-03-MAIL` value lines | 0 |

Participants per transaction: two Characters at most. The event payload ceiling is measured by
MAIL-1 against the audit envelope before implementation.

## 12. Rejected options

- **Delayed delivery.** Tibia and Canary deliver at once; a queue adds a custody family and a job.
- **A mail custody family between mailbox and Inbox.** One transaction moves the item; DUR-03 §34
  is not needed.
- **Reading the town line.** Under 2b every town opens one Inbox; the town cannot misroute mail.
- **Dropping a misaddressed item on the mailbox tile.** It creates a Ground item and a D191
  retirement for a typo; keeping it in the backpack is safer.
- **Flattening a parcel into loose Inbox entries.** The recipient would lose the parcel Tibia
  shows, and a parcel of 10 would take 11 entries.
- **A parcel-only nesting path before BAGS-0.** It would be a second, narrower bags design.
- **An unlimited Inbox for mail.** It lets any player lock another's Market.
- **Owning all writable text here.** Books and blackboards need their own reach and edit rules.

## 13. Owner-rule applications

Global parity (owner rule 5905825574) applied by the architect, each reversible by a later
decision:

- **Parity kept:** instant delivery to offline and other-channel recipients; the name as the
  only address; stamping, so a received parcel is repacked; the 79 and 1,999 character texts; the
  last writer and time shown; mail to oneself and to the same Account; the "New mail has arrived."
  notice next to a depot; the rent warning letter in the Inbox.
- **Declared differences** (`PARITY_PENDING`): a misaddressed item stays in the backpack; parcels
  hold at most 10 children without contents; the 1 per second posting rate; the 50,000 mail
  ceiling; posting only from backpack direct entries; juniors send and receive letters only (owner
  Q2b).

## 14. Owner questions

**Q1. May the game create system letters in the Inbox?** Tibia sends a warning letter when rent
is unpaid. A minted letter is a new item source, which needs an owner decision (as D208 did).
Letters have no NPC value. a) Yes, stamped system letters, first for rent warnings
(recommended); b) no, keep the warning shown on login.
Owner answer (2026-09-30, #162): a — yes, system letters are delivered (§10).

**Q2. May junior (starter-island) characters send and receive mail?** The manual lists the junior
limits (houses, Market selling, bank transfers) and not mail; Canary has none. Mail lets a main
character send items and coins to a new one, which BANK-0's answer b keeps from the bank. a) Yes,
as in Tibia (recommended); b) letters only, no parcels; c) no mail until it leaves the island.
Owner answer (2026-09-30, #162): b — a junior on the starting island sends and receives letters
only, no parcels (§5, §7).

**Q3 (answered). What does the sender see when a parcel is addressed to a junior?** Q2b refuses it
(§7). A distinct result tells any player that the named character is a junior; `UNKNOWN_RECIPIENT`
hides it but tells the sender that the name does not exist, while a letter to the same name is
delivered. a) `UNKNOWN_RECIPIENT`, as CHAT-0 hides where a name exists; b) a new result
`RECIPIENT_CANNOT_RECEIVE_PARCELS`. The refusal itself does not wait on Q3; MAIL-WIRE-1 registers
the answer.

Owner answer (2026-09-30, #162): a — `UNKNOWN_RECIPIENT`; junior status stays hidden (§7). No new
result is registered.

## 15. Decision test

- **Must decide now:** YES. The owner asked for mail now, and HOUSE-OWN-0 waits for it to send
  rent warnings.
- **Minimum sufficient:** one command destination, one text store for two definitions, one
  operation table, one transaction per posting, one stamp rule; parcels reuse bags.
- **Superseding evidence:** official mail limits or junior rules.
- **Deliberately not decided:** house mailboxes, posting from the ground, Store-bound items,
  nested bags in parcels, other writable items, delayed or returned mail.

## 16. Before-freeze checklist

1. **Contract amendments:** DUR-03 §38, MARKET-0 §5, HOUSE-OWN-0 §5, CHAT-0 §5 and the composition
   decision, each written "pending on acceptance of MAIL-0" in this PR.
2. **Serialization:** one transaction per posting; §6's lock order; the recipient's
   `character_root` `FOR SHARE` lock and its Inbox counter row lock.
3. **Restart:** operations, texts and Inbox entries are durable; replay by occurrence.
4. **Typed references:** `MailId` (player postings only), CharacterId, WorldId, `name_key`,
   `placement_key`, `map_item_handle`, ItemInstanceId, occurrence, the system letter
   (`kind`, `occurrence_key`), TransactionId.
5. **Wire:** §9, capability `MAIL_V1`.
6. **Split work:** one posting per transaction; at most 11 items touched.
