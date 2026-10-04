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
`RESTRICTED_PLAYER_LINKED`, a **P90D** finite ceiling (7,776,000 elapsed seconds from the
immutable trusted-server envelope timestamp, never recomputed), the legal hold as an explicit
case-scoped exception that returns to the original expiry without resetting the clock, deletion at
expiry with no indefinite anonymized or pseudonymous copy, and case-scoped redacted export that
does not extend retention. None needs an owner answer: no duration exceeds the 90-day ordinary
ceiling and no purpose goes beyond proof, reconciliation and bounded support and security
investigation.

| Profile id | Packet | Purpose | Notes |
|---|---|---|---|
| `ECONOMY_LEDGER_RETENTION_V1` | BANK-RET-0 | `ECONOMY_LEDGER`: bank balances, ledger entries and coin lines | for BANK-1's event type 3; no market analytics, public history, detector or AI use |
| `DUR03_ONE_ITEM_DURABLE_AUDIT_RETENTION_V2` | BANK-RET-0 | the V1 purpose plus the bank part of a fee (the `FEE_DEBIT` value line) | `policy_revision` 2; every other field is V1's; see §2 |
| `MARKET_ECONOMY_LEDGER_RETENTION_V1` | MARKET-RET-0 | `ECONOMY_LEDGER`: market operations, escrow, matching, fees, Inbox delivery | for MARKET-1's market event; no market analytics, public price or trade history, balancing, detector or AI use |
| `GUILD_ACTIVITY_RETENTION_V1` | GUILD-RET-0 | `GUILD_ACTIVITY`: the member activity log (read by current members of that guild only, per-guild isolated, within 30 days), plus proof and reconciliation of membership, rank, guild bank and guildhall operations | the P90D ceiling is above the 30-day member read window (`GUILD0-RL-10`); a guild bank entry is also within `ECONOMY_LEDGER` for proof only |

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
