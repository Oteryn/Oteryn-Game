# HIRE-1 Hirelings as a Store-delivered, house-bound NPC product

- Decision: `HIRE1-HIRELING-STORE-PRODUCT-V1`
- Status: **CANDIDATE**. Acceptance needs exact-head validation, independent review (persistence, economy and protocol) and protected integration. Owner acceptance is the owner's to give; none is recorded here.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: owner item 7 of 2026-10-15 (#1622 comment 6187301378): "HIRE-1 goes ahead and is not deferred. A brief is needed. Delivery follows Tibia: the hireling is a Store product, wired once the Store contract lands." Plan row 90 (`OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md`, batch HIRE-1).
- Builds on: HOUSE-OWN-0 and HOUSE-RUNTIME-0 (house ownership and interior; accepted), NPC-0 (NPC runtime and trade; accepted by ACCEPT-BATCH-2026-10-15), SUPPLY-STASH-0 (Steward), BANK-0 and BANK-FEE-0 (Banker), PREMIUM_ACTIVATION (Store entitlements pattern), and the Platform Store contract (Oteryn-Platform PR #1625, **pending**).
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`
- Runtime, migration, wire and production authority: NONE. Delivery is blocked by the Platform Store contract; the Game side can be written against its interface (§4).

## 1. What a hireling is (reference)

From `docs/reference/tibia-manual/products.md` §Hirelings and `world.md`:
- NPCs placed in houses; maximum 10 per character. Gender and name are chosen at purchase; names follow the Tibia Rules and a violation gets a random replacement.
- The base **Hireling Apprentice** greets guests and sells basic items.
- **Apprenticeships** unlock services: **Banker** (bank and guild bank access), **Trader** (city trade goods), **Steward** (stash access), **Cook** (food preparation, fee-based). All hirelings of one character share the unlocked services.
- **Hireling dresses** are cosmetic outfits; ownership limits outfit allocation (two hirelings need two copies of a dress).
- **Hireling services:** name change and sex change, visible only to a player who owns hirelings.
- A hireling is sold through the Store as house furniture ("NPCs can be purchased via the Store as hireling-type furniture for houses").

Upstream behaviour (Canary reference, `data/scripts/actions/items/hireling_lamp.lua`, `data/modules/scripts/gamestore/catalog/house_hirelings.lua`):
- Purchase delivers a **lamp** item bound to a hireling record. Using the lamp spawns the hireling and consumes the lamp.
- The lamp works only inside a house **owned by the user**, not on a door tile, and not on a tile that already holds a hireling.
- Skills (Banker, Trader, Steward, Cook), name change and sex change are separate Store offers. Store prices in the upstream catalog are Tibia Coin amounts; Oteryn prices are the Platform's decision, not this brief's.

## 2. Rules

**H1. A hireling is a Store product.** The Game never sells one for gold or in an NPC dialogue. It receives a **Store delivery** for a character: `HIRELING` (a new hireling lamp), `HIRELING_SKILL` (an apprenticeship), `HIRELING_NAME_CHANGE`, `HIRELING_SEX_CHANGE` or `HIRELING_DRESS`. Each delivery is an idempotent grant keyed by the Platform's delivery id, consumed once, exactly as the Premium entitlement consumer (PREM-1) consumes entitlements.

**H2. Records.** A hireling row belongs to the character (owner), not to the house: `hireling_id`, `owner_character_id`, `name`, `sex`, `outfit`, `state` (LAMP, PLACED), `house_id` and `tile` when PLACED. The apprenticeship set is per **character** (shared by all its hirelings, as the manual says), a bitset of the four services. At most 10 hirelings per character; an 11th delivery is held by the Platform contract and refused with a defined code, never silently dropped.

**H3. Lamp and placement.** A delivered hireling is a LAMP item in the character's inbox (the CharacterInbox of INBOX-1, as MAIL/Market deliveries do) and a durable hireling row in state LAMP. Using the lamp places the NPC: one transaction retires the lamp (a retirement) and sets the row PLACED on the target tile. Placement is refused when the actor is not in a house they own, on a door tile, or on a tile that holds a hireling. Removing the hireling (a dialogue option) returns it to a LAMP in the owner's inbox. The row, never the NPC entity, is the durable truth.

**H4. House-bound.** A PLACED hireling exists only while its house is owned by the owner. When the house is lost (rent unpaid, moved out, auction, HOUSE-OWN-0), every hireling in it returns to a LAMP in the owner's inbox in the same transaction as the house eviction, like the house items' custody rule of HOUSE-CUSTODY-0. A hireling does not wander and has no combat or pathing; it stands on its tile and is not attackable (an NPC, NPC-0 rules).

**H5. Job set.** The hireling NPC offers, by dialogue and trade, only what its owner has unlocked:
- **Apprentice (always):** greets guests (a voice line) and sells basic items from a fixed catalogue (NPC-0 trade, content).
- **Banker:** the owner's bank (BANK-0 balance, deposit, withdraw, transfer) and the guild bank where the owner has rank; fees as in BANK-FEE-0. Guests get no bank access.
- **Trader:** the city trade goods catalogue (equipment, tools, potions, runes, wands, rods), the same NPC-0 trade writer with a hireling catalogue.
- **Steward:** access to the owner's stash (SUPPLY-STASH-0: stow, withdraw, list). The same stash, not a copy; the same limits.
- **Cook:** food preparation for a fee. The fee is paid through the bank-first fee path (BANK-FEE-0); the food is minted. The recipe list is content (CRAFT-1 conversions are a related table; Cook rows live in the hireling catalogue, not in CRAFT-1).
Guests (other players in the house) may use the Apprentice and the Trader. Owner-only services (Banker, Steward) refuse a guest.

**H6. Name and sex.** A hireling name is validated by the character-name rules of the account decisions (Tibia Rules) and a violation is replaced by a deterministic placeholder; change is by Store delivery H1 and cancellable when the name is unavailable (the manual). Sex changes the hireling outfit family; the outfit is preserved otherwise.

**H7. Dresses.** A dress is an outfit entitlement per character with a count; allocating a dress to a hireling consumes one copy of that dress for as long as the hireling wears it (two hirelings, two copies). Exact allocation UI and copy accounting are wire detail for HIRE-WIRE-1.

**H8. Refusals** are fixed typed codes; no gold is ever taken by the Game in H1 to H7 (the Cook fee excepted, H5).

## 3. Persistence and economy

- Every created lamp is a MINT with a `HirelingDelivery` cause bound to the delivery id (single mint per delivery id); every lamp use or removal is a retirement or a mint with a `HirelingPlacement` cause. Conservation holds: a hireling is one row plus at most one lamp item.
- Hireling rows are character-scoped persistent state. They are not part of the house item custody family, but the H4 return runs in the house loss transaction under the HOUSE-OWN-0 lock order; the lock-order rule of that decision is amended in HIRE-1's own packet, not here.
- No hireling state advances `CharacterRevision` except the lamp in the inbox (an item).

## 4. Delivery contract (blocked)

The Platform Store contract (Oteryn-Platform PR #1625) is pending; its S-1 to S-9 questions are still open with the owner (item 12). This brief therefore defines only what the Game needs from it:
- a signed, idempotent delivery message per character with a delivery id, a product kind (H1) and a payload;
- a delivery status callback (applied, held, refused with code);
- no price, currency or payment data in the Game.

Until the contract is accepted and merged, HIRE-1 children that touch delivery wait. The packets that do not (the hireling record, the placement and return, the NPC jobs by owner-grant) can be written against a stub that grants by a test entitlement.

## 5. Packets (proposed)

1. **HIRE-DATA-1** (hard, persistence review): H2, H3, H4 tables and transactions, the lamp mint/retire, the house-loss return. Needs HOUSE-1a.
2. **HIRE-NPC-1** (impl): the hireling NPC entity, content for the Apprentice catalogue and voice lines (NPC-0).
3. **HIRE-JOBS-1** (hard, economy review): Banker, Trader, Steward and Cook per H5, on the accepted bank, stash and trade writers.
4. **HIRE-WIRE-1** (protocol, Reg): the outfit window for hirelings, dress allocation, name and sex change dialogs; a contract candidate of its own.
5. **HIRE-STORE-1** (hard, security and economy review): the delivery consumer of H1, **after** the Platform Store contract is accepted.
Order: HIRE-DATA-1, HIRE-NPC-1, HIRE-JOBS-1, HIRE-WIRE-1, then HIRE-STORE-1. The plan lists HIRE-1 in Wave 5 after HOUSE-1a; nothing here changes that.

## 6. Open points (not decided)

- Whether the Cook fee and the Trader catalogue use the Tibia amounts of the reference: content, set in the content packet.
- Store product pricing and currency: the Platform's, and the owner's S-1 to S-9 answers.
- Whether a guest may use the Steward in a guildhall: not in the reference; decided with GUILDHALL-1.

## 7. Handback

```yaml
result: CANDIDATE
durable_decision_ref: docs/architecture/reviews/OTERYN_GAME_HIRE1_HIRELING_DECISION_2026-10-15.md
resource_values_changed: false
production_authority_changed: false
cross_repository_authority_changed: false
implementation_may_resume: false
required_fresh_allocation: true
required_independent_review: "persistence, economy, protocol; security for HIRE-STORE-1"
blocked_on: "Oteryn-Platform PR #1625 (Store contract) for HIRE-STORE-1 only"
```
