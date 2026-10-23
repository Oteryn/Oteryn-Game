# VIP-2 — VIP presence on the World relay (consolidated task card)

```yaml
task_id: VIP-2
title: "VIP-2: VIP presence on the World relay"
mode: IMPLEMENT
status: waiting
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 5
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 Epic 5 row VIP-2 (row 114); App. 7 VIP-*; §5 Wave 5"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_VIP0_VIP_LIST_DECISION_2026-09-30.md
  - docs/architecture/SOCIAL_PRESENCE_AND_CONTACT_CONSENT_OWNER_BASELINE.md
base_branch: main
branch: null
pr: null
class: "hard (security, privacy)"
needs_migration: true
needs_registry: true
needs_content_regeneration: false
merge_authority: CP_ONLY (builder never marks ready)
```

Tracking audit (CARDS-AUDIT-1, 2026-10-23, `origin/main@ac3f7fbc6`): **OPEN**. This supersedes historical status/PR-wait notes below; acceptance and scope are unchanged.

Merged evidence: none. No VIP presence/relay implementation from this card is retained on main. Evidence: `crates/protocol-oteryn/src/capability_tables.rs`.

Remaining: Current VIP owner, presence relay/login/logout feed, wire/session state and watcher/reconnect qualification remain. `pr: null` means this audit identifies no canonical unmerged candidate; merged slices are linked above.

Reference implementation (pre-ADR-0022, do not copy paths or structure): draft #2164, branch skeleton/vip-2

Status note: the draft (33 files, ~13k lines) built a presence owner, relay kind 3
(PresenceHint), the VIP-1b production schema, the `VIP_V1` wire, a session client store and a
harness pane on the pre-ADR-0022 `apps/game-server/src/...` layout, with CP widening (D1435,
D1444) to make VIP one mechanic in one PR. Its numbering (capability 31, command 31, state
domain 21, migration `0130_vip_lists.sql`) is void: 0130 is reserved by MAP-PICKUP-1 and all
ids are allocated fresh. It is closing; the mechanic is re-implemented fresh on the current layout.

## Goal

VIP online presence over the World chat relay, gated by per-Character consent and
`vip_status` (hide my status) per VIP0 §5.1; VIP-1b production list/consent schema; `VIP_V1`
wire, client store and harness pane.

## Dependencies (re-verified on origin/main 59785d76, 2026-10-22)

| Dependency | State |
|---|---|
| VIP0 acceptance | ACCEPTED (#2209) |
| VIP-1 (list/consent store) | MERGED (#2143) — but see note below |
| CHAT-2B relay (and CHAT-2A) | MERGED (#2227 / #2076) |
| LOGOUT-WIRE-1 (real logout path for the offline hint) | MERGED (#2127) |
| PREMIUM-1 (100-entry Premium limit) | MERGED (#2110) |
| HOUSE-RUNTIME-1b (named-House session fence) | BLOCKING — re-implementation pending (draft #2165 closing); House watchers stay `AuthorityRejected` until then |
| List/consent relay hint (new relay kind byte) | BLOCKING for that slice only — CP/owner decision |

VIP-1 note: #2143 added `apps/game-server/src/domain/vip.rs` and `apps/game-server/src/durability/vip.rs`
(commit 4e0d9f2c1); the ADR-0019 carve (#2270) removed both and no `vip` module exists under
`crates/` on main. Only `apps/game-server/src/lifted_tests/durability_vip_tests.rs` remains,
mounted `#[cfg(test)]` from `apps/game-server/tests/guild_postgres.rs` against
`crate::durability::vip`. At allocation, restore the VIP-1 store from 4e0d9f2c1 into its
ADR-0022 layer (domain → `crates/gameplay/domain`, store → `crates/gameplay/durability`) as the
first commit of this task, or report it as a separate P1 if the CP prefers.

## Code locations (ADR-0022; verified on origin/main 59785d76)

Existing owners to reuse (do not fork):

- `crates/gameplay/social/src/chat/relay/mod.rs` (+ `tests.rs`) — World relay; presence is a new
  relay kind on this owner.
- `crates/gameplay/social-rules/src/relay.rs` — relay rules/bounds (CHAT0-RL-09 outbox budget).
- `crates/gameplay/dispatch/src/gameplay_transport/chat_relay.rs` (+ `chat_relay/tests.rs`) —
  sender, listener, RL-10/RL-11 meter.
- `crates/gameplay/dispatch/src/gameplay_transport/chat_intent.rs` — `VipRuntime` field only.
- `crates/gameplay/dispatch/src/gameplay_transport/logout.rs`, `logout_postgres_tests.rs` —
  offline hint on the real logout path.
- `crates/gameplay/durability/src/durability/fresh_admission.rs`, `world_party.rs`,
  `house_interior.rs` — admission/session state and House scope readers.
- `crates/gameplay/dispatch/src/gameplay_transport/connection.rs`, `capabilities.rs`,
  `capabilities_tests.rs` — wiring only.
- `crates/protocol-oteryn/src/lib.rs` — `pub mod` + registry constants (wiring only).
- `crates/session/src/lib.rs` — gated command route and default offer (wiring only; store in a
  new file if the crate allows).
- `tools/synthetic-client-harness/src/live/{model,controller,tests}.rs`,
  `tools/dev-client/src/lib.rs` — harness pane / `vip()` accessor.
- `apps/game-server/migrations/0122_chat_relay_leases.sql`, `0046_world_parties.sql` —
  existing schema patterns (immutable).
- `apps/game-server/src/lifted_tests/durability_vip_tests.rs` — VIP-1 PG cases (see note).

Proposed new files (one mechanic per file, layer per ADR-0022):

- `crates/gameplay/domain/src/domain/vip.rs` (new; restored VIP-1) — L2 list/consent types.
- `crates/gameplay/durability/src/durability/vip.rs` (new; restored VIP-1) + `vip_tests.rs` (new).
- `crates/gameplay/durability/src/durability/vip_presence.rs` (new) — L4
  `read_current_vip_presence`, `refresh_batch_current`.
- `crates/gameplay/social/src/vip/presence.rs` (new) + `presence_tests.rs` (new) — L3
  `PresenceOwner`, coalescing outbox, announce rule.
- `crates/gameplay/dispatch/src/gameplay_transport/vip_intent.rs` (new) — VIP command handler
  and state-domain deltas only.
- `crates/protocol-oteryn/src/vip.rs` (new) + `vip_tests.rs` (new);
  `docs/contracts/protocol-oteryn/v1/vip_v1.proto` (new).
- `docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json`, `docs/contracts/RESOURCE_LIMITS_REGISTRY.json`
  rows (VIP0-RL incl. RL-11) — capability, command and state-domain ids: next free at
  allocation time (main's highest capability is 30).
- `apps/game-server/migrations/<next>_vip_lists.sql` (new) — VIP-1b production schema and
  grants; migration number: next free number at allocation time (only one migration PR in the
  queue).

## Accepted design (VIP0 §5.1, social presence baseline, as built in the draft)

Privacy (fail closed):

- Consent is per inviter Character, never per Account. HIDDEN (`vip_status = 2`) shows OFFLINE
  with no notice.
- ONLINE only when the target session state is 1/2 on the same World. No Channel, instance,
  node, position or Account identity leaks to the watcher.
- ONLINE confirmation lifetime 10 s (VIP0-RL-16), reconcile every 60 s (RL-09). Listener down
  → UNCONFIRMED and VIP writes refused with `VIP_UNAVAILABLE`.
- Death, grace and channel/World transfer are not a logout.
- House watchers: `AuthorityRejected` until HOUSE-RUNTIME-1b supplies the session fence.

Presence owner:

- `PresenceOwner` with `read_current_vip_presence` and bounded `refresh_batch_current`
  (100 entries; query `LIMIT 101` detects overflow).
- Pending updates dequeue by lowest revision; bounded per-Character coalescing outbox inside
  the CHAT0-RL-09 budget. Announce only when a read confirms the hinted transition.
- `write_bound` / `write_bounded`: every frame bounded by the ONLINE deadline.
- `VipRuntime::detach` on lose-control.

Wire and session:

- Handles numbered from `connection_generation << 32`; keep the newest 64 retired bindings;
  replay compares the receipt digest, mismatch → `CONFLICTING_COMMAND`.
- Field 4 `confirmed_for_ms` (1..=10000; owner sign-off). Golden vectors: ONLINE
  `08 02 10 02 20 90 4e`; OFFLINE + announce `08 02 10 01 18 01`.
- Payload bounds: intent 536, result 4, entry 566, consent 5502, snapshot 62935, status 18.
- Limits: 20 entries free / 100 Premium; 8 groups (3 built-in + 5 custom).
- Client `VipConfirmations`; `SessionEvent::VipExpired` on lapse. Capability in the default
  offer (owner sign-off).

## Review dispositions (from the draft)

- CP D1435/D1444 widened the scope so presence, VIP-1b schema and wire land as one mechanic.
- Owner signed off the default offer and the `confirmed_for_ms` wire change.
- Migration renumber churn (0124 → 0130) is void; allocate once at slot time.
- Deferred, not regressions: list/consent relay hint (new kind byte, CP/owner decision); F2
  snapshot retry; node scheduler for `refresh_batch_current` in
  `apps/game-server/src/node/serve.rs`.

## Remaining work

Everything on the current layout: restore VIP-1 store, migration, presence owner and relay
kind, logout/login feed, wire + registry + resource limits, session store, harness pane, PG
and end-to-end watcher cases. Deferred items above stay out unless the CP allocates them.

## Acceptance criteria

- [ ] HIDDEN and non-consented targets never appear ONLINE; no Channel/instance/node/position/Account leak.
- [ ] ONLINE expires after 10 s without confirmation; listener loss → UNCONFIRMED, writes refused.
- [ ] Death/grace/transfer emit no OFFLINE; real logout emits exactly one OFFLINE hint.
- [ ] Command replay returns the first receipt; differing digest → `CONFLICTING_COMMAND`.
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.
- [ ] Canonical PG cases show real database-start markers.
- [ ] Structure: one mechanic per module file in its ADR-0022 layer.
- [ ] Structure: new tests in their own child files (not appended to existing files).
- [ ] Structure: edits to `mod.rs`/`lib.rs`/`connection.rs`/`capabilities.rs` are wiring-only.
- [ ] Structure: no file over ~1000 lines grows by more than ~150 lines.

## Inseparable bundles

- migration + VIP store + durability (list, consent, `vip_status`)
- relay kind + presence owner + logout/login feed
- wire + registry + resource limits + capability + handler + session store + tests

## Validation

```bash
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked -p oteryn-gameplay-social -p oteryn-gameplay-durability vip
cargo test --locked -p oteryn-game-server --test durability_postgres vip -- --test-threads=1  # PG 17.6
cargo test --locked -p protocol-oteryn vip
cargo test --locked -p oteryn-synthetic-client-harness
python3 tools/agents/validate_governance.py
```

## Reference evidence

- VIP0 §5.1, VIP0-RL-09/-11/-16; CHAT0-RL-09; social presence and contact consent baseline
- VIP-1 source at commit 4e0d9f2c1 (#2143)
- Completion plan §4, §5 (Wave 5), §7; coordination #2131 (D1435, D1444, D1453)
