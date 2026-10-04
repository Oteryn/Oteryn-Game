# ECON-RET-0 and BANK-RET-0 economy and guild event retention

- Decision: `ECONRET0-ECONOMY-GUILD-EVENT-RETENTION-V1`
- Status: **CANDIDATE**. Acceptance needs exact-head validation, the privacy review (Codex, final
  frozen head) and protected integration.
- Packets: ECON-RET-0 (`OTV2-20261004-econ-ret-0`: GUILD-RET-0 and MARKET-RET-0, from
  `OTERYN_GAME_ARCH_BATCH_PREMIUM_SOCIAL_PACKETS_2026-10-04.md` §2.1) and BANK-RET-0
  (`OTV2-20261004-bank-ret-0`, from `OTERYN_GAME_ARCH_BATCH_ROOT_PACKETS_2026-10-04.md` §2.3 and
  §1.7). One PR, because both packets edit only `retention_profiles` of
  `docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json`.
- Builds on: GUILD-0 §4.4 and `GUILD0-RL-10`; MARKET-0 (event under MARKET-RET-0); BANK-0 §3 and
  §5; BANK-FEE-0 §4.3; the DUR-03 and Character retention profiles.
- Runtime, migration, wire and production authority: NONE. No event type, code or schema is added.

## 1. Profiles registered

All four are new immutable ids with every `required_profile_fields` entry. Each has privacy class
`RESTRICTED_PLAYER_LINKED`, a finite ceiling counted in elapsed seconds from the immutable
trusted-server envelope timestamp and never recomputed (**P30D** = 2,592,000 s for the bank,
market and guild profiles; V2 keeps V1's P90D, see §2), the legal hold as an explicit
case-scoped exception that returns to the original expiry without resetting the clock, deletion at
expiry with no indefinite anonymized or pseudonymous copy, and case-scoped redacted export that
does not extend retention. Owner decision D503 5a: each profile gets the shortest duration backed
by an accepted requirement. BANK-0, BANK-FEE-0 and MARKET-0 state no retention duration longer
than 30 days (BANK-0 and BANK-FEE-0 only defer retention to BANK-RET-0; MARKET-0 states 30-day
offers and leaves the event to MARKET-RET-0), so none exceeds 30 days. No purpose goes beyond
proof, reconciliation and bounded support and security investigation.

| Profile id | Packet | Purpose | Notes |
|---|---|---|---|
| `ECONOMY_LEDGER_RETENTION_V1` | BANK-RET-0 | `ECONOMY_LEDGER`: bank balances, ledger entries and coin lines, and the stash, house and mail events (P30D) | for BANK-1's event type 3 and the stash, house and mail events (§1.1); no market analytics, public history, detector or AI use |
| `DUR03_ONE_ITEM_DURABLE_AUDIT_RETENTION_V2` | BANK-RET-0 | the V1 purpose plus the bank part of a fee (the `FEE_DEBIT` value line) | `policy_revision` 2; every other field, P90D included, is V1's; see §2 |
| `MARKET_ECONOMY_LEDGER_RETENTION_V1` | MARKET-RET-0 | `ECONOMY_LEDGER`: market operations, escrow, matching, fees, Inbox delivery (P30D) | for MARKET-1's market event; no market analytics, public price or trade history, balancing, detector or AI use |
| `GUILD_ACTIVITY_RETENTION_V1` | GUILD-RET-0 | `GUILD_ACTIVITY`: the member activity log (read by current members of that guild only, per-guild isolated, within 30 days; P30D), plus proof and reconciliation of membership, rank, guild bank and guildhall operations | P30D equals the member log window (`GUILD0-RL-10`); a guild bank entry is also within `ECONOMY_LEDGER` for proof only |

### 1.1 Designated consumers of the economy profile

Three accepted decisions already bind their event to BANK-RET-0's economy profile, so its purpose
and its operational reader (`authorized bank, stash, house and mail operations`) cover them. Each
stays P30D, because none of the three states a retention duration:

- stash events: SUPPLY-STASH-0 §7 (`OTERYN_GAME_SUPPLY_STASH0_SUPPLY_STASH_DECISION_2026-10-03.md`
  line 238, "under the `ECONOMY_LEDGER` purpose that BANK-RET-0 defines");
- house events: HOUSE-OWN-0 §17 (`OTERYN_GAME_HOUSE_OWN0_HOUSE_OWNERSHIP_DECISION_2026-09-30.md`
  line 268, "retention under BANK-RET-0's economy profile");
- mail events: MAIL-0 §11 (`OTERYN_GAME_MAIL0_PARCELS_AND_LETTERS_DECISION_2026-09-30.md` line
  230, "Retention under BANK-RET-0's economy profile (as HOUSE-OWN-0)").

Each event type is still registered by its own child (STASH-1, HOUSE-1, MAIL-1). The reader set
widens by role names only; no export or access path is added here.

The authoritative bank, ledger, operation, coin-line, offer, escrow and guild tables are game
state, not event retention, and are never deleted by these profiles.

## 2. Boundaries kept

- `DUR03_ONE_ITEM_DURABLE_AUDIT_RETENTION_V1` and `CHARACTER_AUTHORITY_DURABLE_AUDIT_RETENTION_V1`
  are unchanged byte for byte; the change to the registry is a pure insertion after them.
- Event type 2 stays bound to V1 at schema revision 1. V2 is only registered. Binding every type-2
  producer to V2 is GOLD-FEE-ACT-2's activation row (#1746), after the reviewed activation
  boundary of ROOT-PACKETS §1.7; existing events keep their original profile id.
- No event type is added. BANK-1 binds `ECONOMY_LEDGER_RETENTION_V1` to type 3, MARKET-1 binds the
  market profile, and GUILD-1 binds the guild profile when each registers its event.
- A profile is never revised in place after first admission; a change is a new id with a positive
  `policy_revision` (`retention_policy`).

## 3. Options considered

Common trade-off: a longer ceiling holds player-linked data longer; a shorter one cuts privacy
exposure but expires the proof sooner. Owner decision D503 5a picks the shortest duration backed
by an accepted requirement.

| Question | Option | Verdict |
|---|---|---|
| Duration | **P30D** for the bank, market and guild profiles | **Chosen** (D503 5a), justified per profile in §4. |
| Duration | P90D, the repository's ordinary ceiling (Character and DUR-03 profiles) | Rejected for these three: no accepted requirement in BANK-0, BANK-FEE-0 or MARKET-0 needs more than 30 days. V2 keeps P90D only because it is V1's value and a successor changes nothing but the purpose. |
| Duration | More than P90D, or unbounded | Rejected: unbounded retention is forbidden (`ordinary_unbounded_retention_forbidden`) and no requirement asks for it. |
| Bank | Reuse the DUR-03 V1 profile | Rejected: its purpose excludes the economy, so a bank event would be admitted outside its purpose. |
| Bank | Revise V1 in place | Rejected: a profile is immutable after first admission (`in_place_policy_change_after_admission: FORBIDDEN`). V2 is a new id with `policy_revision` 2, registered only; V1 and every admitted event keep V1. |
| Bank, market | One shared economy profile | Rejected: the purposes differ (balances and coin lines against offers, escrow and matching), and one id would force a joint change later. |
| Guild | Reuse the bank profile | Rejected: guild events serve a member-visible log and need a per-guild member reader (`GUILD0-RL-10`), which a bank reader set must not get. |
| Guild | Let any guild member read the whole event | Rejected: a multi-guild event holds other guilds' entries (GUILD-0 §4.4, ANL-01 §16), so the member read is per guild and redacted. |
| Packaging | Two PRs | Rejected: both edit `retention_profiles` of one file with one writer. |

Risks and mitigations:

- **A duration or purpose that is too narrow.** A requirement found later that needs a longer
  duration is met by a new immutable id with a positive revision, for future admission only.
  Existing events keep their original id and are not extended.
- **A member reader that leaks another guild's entry.** The reader rule is in the profile and is
  GUILD-1's acceptance test.
- **V2 registered but not active.** Type 2 stays bound to V1 until GOLD-FEE-ACT-2 (#1746), so a
  rolling deploy never mixes profiles.

## 4. Why each profile must be decided now, and why its duration

Each duration is justified separately.

- **Bank (and the stash, house and mail consumers), P30D.** BANK-0 §5 and BANK-FEE-0 §4.3 only
  defer the bank event's retention to BANK-RET-0 and state no duration; §1.1 shows the stash,
  house and mail contracts state none either. The authoritative balance, ledger, operation and coin-line tables are
  never deleted (BANK-0 §3), so the event is a proof copy, not the record. No accepted
  requirement needs more than 30 days, so the shortest backed duration applies.
- **DUR-03 V2, P90D (unchanged).** V2 is V1 plus the bank part of a fee. Its duration is V1's by
  the successor rule and is not re-decided here; shortening it is a separate reviewed profile.
- **Market, P30D.** MARKET-0 fixes the offer lifetime at 30 days (`MARKET0-RL-05`) and bounds
  ended offers (`MARKET0-RL-07`); it names the event's retention as MARKET-RET-0 and states no
  longer need.
- **Guild, P30D.** GUILD-0 §4.4 and `GUILD0-RL-10` state a 30-day member log. That is the only
  accepted requirement, so retention equals the read window.

Why each must be decided now:

- `ECONOMY_LEDGER_RETENTION_V1`: BANK-1 cannot register event type 3 without it, and the production
  rule requires a registered profile (`production_requires_registered_profile`).
- `DUR03_ONE_ITEM_DURABLE_AUDIT_RETENTION_V2`: GOLD-FEE-2's phase 1 must read and verify `(2, V2)`,
  and GOLD-FEE-ACT-2 needs it to exist. A fee whose `T < F` stays refused until then.
- `MARKET_ECONOMY_LEDGER_RETENTION_V1`: MARKET-1 binds it when it registers the market event, and
  MARKET-0 names it as a precondition.
- `GUILD_ACTIVITY_RETENTION_V1`: GUILD-1 (also gated on PREM-WIRE-1, #1743) binds it, and GUILD-0
  §4.4 leaves the activity log's retention to GUILD-RET-0.

## 5. What this unblocks

BANK-1 (and through it BANK-NPC-1, STASH-1 and HOUSE-1), GOLD-FEE-ACT-2, GUILD-1 (then
GUILD-BANK-1 and GUILDHALL-1) and MARKET-1. Without these ids the chains cannot register their
events.

## 6. What becomes costly later

A wrong profile found after the first event is admitted cannot be fixed in place. It needs a
successor id, a reviewed activation boundary and an unchanged original for every admitted event, as
V2 shows. An event type admitted without a profile has no lawful retention, so the producer would
have to stop. Deciding the purposes, ceilings and readers now, before a writer exists, is the
cheapest point.

## 7. Evidence that permits supersession

- A privacy review that refuses a profile, a purpose, a duration or a reader set.
- A cited line of BANK-0, BANK-FEE-0, MARKET-0 or GUILD-0 that states a concrete longer need.
- A requirement that needs a duration above P30D, or a purpose beyond proof, reconciliation and
  bounded investigation, with an owner answer (a new profile id).
- A measured need from BANK-1, MARKET-1 or GUILD-1 (for example a reader the guild log requires).
- A different activation boundary for type 2 decided in GOLD-FEE-ACT-PACKET-1 (#1746).

## 8. Deliberately not decided

- Any event type, event code, payload schema, table, migration or runtime.
- The activation of V2 for type 2 (GOLD-FEE-ACT-2).
- The binding of each profile to its event type (BANK-1, MARKET-1, GUILD-1).
- The guild member reader's API and its access test (GUILD-1) and the support export procedure.
- Any change to the Character or DUR-03 V1 profiles.
