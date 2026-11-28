# Wheel presentation v1

Existing capability148 adds command6119 as a Channel-only read; no capability ID is added. the existing161 allocation/reset writer, occurrence binding, paid action interval and durable receipts remain unchanged. No new economy, assets, migrations or combat mechanics are introduced.

## Request

nonce:u64 nonzero, current Wheel revision:u64, current Atelier revision:u64,36 little-endian slot values:u16, then four vessel discriminators0/1 with a16-byte nonzero identity after each1. Maximum164bytes and no trailing data.

## Result

The reply echoes the exact request behind a u16 byte-length prefix. Every text field is u16-length UTF8 up to256bytes without controls. Header: catalogue revision@source-candidate-SHA, vocation, spendable point budget:u32, ring-unlock point budget:u32, strict booleans removal-permitted/allocation-eligible/draft-valid, and a reason string. Exactly36 nodes follow: domain:u8, capacity:u16, minimum ring-unlock points:u32, unlock count:u8 (0..8), sorted unique zero-based unlock slots:u8 (0..35), name text, per-point configured lines, full-slot configured lines. Exactly four domain-ordered revelations follow, each with domain:u8, name and three ascending stages containing threshold:u32 and configured-effect lines. Current and draft configured-effect line lists finish the reply. A line list is count:u8 (0..128) followed by texts. Maximum reply131072bytes; canonical all-five-vocation tests prove every actual node, link and perk fits with no truncation.

A single existing-owner transaction reads actual current progression, promotion, Premium permission, Wheel currency, Atelier grades/vessels, task promotion points, redeemed promotion scrolls and the admitted pilgrimage track. The exact budget reader is shared with the original writer: Level-earned points (`max(0, level - 50)`), task promotion points, earned pilgrimage points and redeemed scroll points form the ring-unlock budget. Qualifying Grade-IV modifiers are added only to the spendable budget. This preserves the existing writer's semantics, including earned pilgrimage points without adding a new vocation restriction. Premium-latch-before-root lock order, account recheck, Character/session/lease/scope/node fencing and revision consistency are retained. The read creates no receipt, cooldown, reservation, state row, ledger or item change. Actor/connection/lease/runtime ownership is rechecked after awaits. Removal permission calls the same combat-lock/PZ helper as the original writer; dead, reserved or pending-commit actors cannot submit changes.

Metadata uses the canonical WheelRuleset topology and pinned WheelGemData names. Every number in a configured-effect preview comes from existing derive_runtime_effects, WheelStages and contribute_gems; no donor description or duplicate bonus table is used. Per-point/full-slot components and revelation stages are potential configurations. Conditional perks are named as condition-dependent available configurations; combat and spells still check their actual live conditions. Invalid drafts have an explicit real-owner refusal, and the writer checks its action interval again before applying anything.

The original6x6 diagram layout draws only supplied unlock links. Clicking a node and the accessible slot selector use the same painted-request nonce fence. Point edits are local; Save requires a reply matching the exact current revisions, points and vessels. Reset first clears the draft and obtains a new validated preview, then explicit Apply reset sends original161 action2. Stale replies, scope changes and end paths invalidate previews; native window epochs prevent an old close from affecting a replacement window. Original161 explicit Retry reuses the same unresolved occurrence; no mutation is retried automatically. Unsupported scopes or missing optional capability show unavailable presentation without inventing eligibility.

## Inspect another Character (6378)

Capability 148 gates this request. Its payload is one 16-byte Character UUIDv7.
A well-formed request returns `Rejected` with the literal UTF-8 text
`Sorry, not possible.` regardless of the target's existence, privacy, or activity.
Malformed requests have an empty rejection. Use 6119 for one's own Wheel.
[Crystal rejects foreign Wheel requests](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/src/game/game.cpp#L11325-L11342).

## Gift of Life status (6379)

Capability 148 gates an empty request. An accepted response is the current
revelation stage (one byte, 0 through 3) followed by a little-endian u64 remaining
online cooldown in milliseconds, rounded up. Stage zero denotes inactive. An
unavailable timer or a stale actor/session yields an empty rejection. The client
can use the stage and timer for a passive action-bar assignment. Assignment is a
client preference. This query cannot activate Gift of Life, which triggers in
the existing damage owner.
[Crystal cooldown status](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/src/creatures/players/wheel/player_wheel.cpp#L1646-L1668).
