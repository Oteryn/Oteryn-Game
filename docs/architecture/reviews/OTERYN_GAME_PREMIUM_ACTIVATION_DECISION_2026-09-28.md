# Premium activation decision

- Decision: `PREMIUM-ACTIVATION-V1`
- Status: **CANDIDATE with owner decisions D69-D76 taken (§2)**. Acceptance requires exact-head
  validation, independent review and protected integration.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.1)
- Profile: `Oteryn Reference` (Global Tibia at 2026-09-27, D33)
- This is the explicit, product-specific Premium activation decision that the architecture README
  (:167) and SPELL-D5 require. It resolves D66 and D67 ("wait for Premium") and SPELL-D5a.
- Consumes without redefining: `PROD-ENTITLEMENTS-01` (Game consumer contract, `ACCEPTED`,
  implementation `NOT_STARTED`), FND-04, DUR-02
- Owner decisions posted: #162 5871244356 (D69-D71), 5871433126 (D72-D75)
- Admission baseline: `main@117a899`
- Runtime, migration, protocol, Platform and production authority: **NONE**
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## 1. Question

The owner wants Premium now. What does Premium give in the first step, how does Game learn that an
account has it, and what happens when it expires during play?

## 2. Owner decisions

| # | Decision | Owner choice (2026-09-28) |
|---|---|---|
| D69 | The real mechanism: Platform grants Premium time as a commercial entitlement; Game enforces it under `PROD-ENTITLEMENTS-01`. First grants come from an owner, operator or test grant with no payment. | "Nadawane bez płatności" |
| D70 | First benefits: promotion, the 3 Premium blessings, soul 200, Premium areas and spells. | all four |
| D71 | Expiry follows Global. | "Jak w Global" |
| D72 | Soul 200 belongs to promoted characters only. | "Tylko z promocją" |
| D73 | On expiry: Premium spells and Premium-area entry stop at use, at once; promotion suspension and relocation out of a Premium area happen at the next login; no forced logout. | "Przy użyciu + przy logowaniu" |
| D74 | Soul above the new maximum after demotion is kept until spent. | "Zostaje do zużycia" |
| D75 | Yalahar is a free city. | "Darmowy" |
| D76 | Refines D73 for promotion, to satisfy `PROD-ENTITLEMENTS-01` §12: promotion benefits (soul maximum, regeneration, the death XP reduction) are checked at each use, so they stop at once when Premium expires; only the displayed vocation and title change at the next login. | "Korzyści od razu, tytuł przy logowaniu" |

## 3. Facts

**PROVEN**

- `PROD-ENTITLEMENTS-01` consumer contract (candidate header, accepted through PR #20):
  - Platform is commercial authority; a Game record is only evidence about it (§2.1).
  - Game owns gameplay enforcement (§2.2).
  - A durable monotonic consumer fence and a finite producer authority interval (§3 option C, §6).
  - Every surface (fresh admission, reconnect, running session) declares `REQUIRE_CURRENT` or
    `ALLOW_PRODUCER_STALE` before activation (§9).
  - Losing Premium never blocks base login (§10).
  - A reconnect re-checks and cannot restore an expired benefit (§11).
  - No forced logout; checks happen at gameplay decision points; a teleport or stat change needs its
    own gameplay transition contract (§12).
- Global (research sources in #162 5871433126):
  - promotion: level 20, a one-time 20,000 gp fee, Premium; suspended when Premium lapses, restored
    with Premium at no fee; its benefits include soul 200, faster regeneration and the −30% death
    penalty reduction;
  - Premium blessings: Premium is needed only to buy them from their NPCs; held blessings are kept
    until death;
  - Premium areas: everything reachable only by boat or flying carpet; free: Thais, Carlin,
    Venore, Ab'Dendriel, Kazordoon, Yalahar;
  - Premium spells: the per-spell Premium column of the official library.
- The spell schema has `requirements.premium` and a Premium cast check (`OTERYN_SPELL_AUTHORING_SCHEMA_V1.md`).
- GAME-CHAR-01 Stage B left promotion under lapse `UNKNOWN`, recommending that the promotion state
  is kept and its benefits derive from the current entitlement (DELTA_02 E1). D73 and D76 decide this.

**UNKNOWN**

- Global mid-session expiry and soul clamping (decided by D73 and D74 as declared policy).

## 4. Decision

### 4.1 Premium status

- Game's Premium status for an account is derived only from Platform's Premium entitlement, through
  the `PROD-ENTITLEMENTS-01` consumer fence. Game never grants, extends or stores Premium as its
  own authority.
- Surface policies (§9 of the consumer contract):

| Surface | Policy | Effect |
|---|---|---|
| Fresh admission | `REQUIRE_CURRENT` for benefits | login always succeeds (§10); benefits apply only if Premium is current |
| Reconnect or recovery | `REQUIRE_CURRENT` | an expired Premium never returns with a reconnect (§11) |
| Running session, at use | `REQUIRE_CURRENT` with the producer authority interval | a Premium spell cast or a Premium-area entry is refused once Premium is no longer current |

- The first grants (D69) are Platform-side operator or test grants. Game treats them like any other
  Premium entitlement.

**Amendment (pending on acceptance of PREMIUM-DELIVERY-0, #1369; consumer contract §9).** The
surface table is consumer policy revision `premium-surfaces-1`, bound to product
`oteryn.premium_time` version 1 (PREMIUM-DELIVERY-0 §4). A class other than `CURRENT_AUTHORITY`
reads as Free. The degraded (Free) behaviour of each benefit is owned by the child that builds it:
PREM-2 (promotion benefits and soul), PREM-3 (areas), PREM-4 (spells), PREM-5 (NPC services).

### 4.2 Promotion (D70, D72, D73, D76)

- Promotion is durable Character state: `promoted` plus provenance. It is bought once at a
  promotion NPC (level 20, 20,000 gp, Premium current) through the NPC service child.
- **Amendment (pending on acceptance of WHEEL-0; `OTERYN_GAME_WHEEL0_WHEEL_OF_DESTINY_DELIVERY_DECISION_2026-09-30.md` §6.2; owner answer W1 a,
  #162 5917665342).** Until PREM-1 delivers `premium_current`, the Premium condition of the purchase and of
  the benefits is not applied; from then on it applies, and a promotion bought before is kept
  under D73 and D76. PREM-2's "progression readiness" dependency is #1143, completed.
- A promotion **benefit** applies only while the durable `promoted` state is set and the
  account's Premium is current at the moment of use (D76, consumer contract §12). Each benefit is
  checked at its own authoritative point:
  - soul maximum 200 (D72): at each soul gain or regeneration step (§4.3);
  - the promotion regeneration values: at each regeneration step;
  - the −30% death XP reduction: at the death transaction (first player death decision).
- The **displayed** promoted vocation and title follow the same rule but change only at login:
  a suspension or restoration of the name shows at the next login (D73). No gameplay effect
  depends on the displayed name.
- The durable `promoted` state never changes because Premium lapsed; renewal restores the benefits
  at once and the name at the next login, with no fee.

### 4.3 Soul (D72, D74)

- Maximum soul is 200 while the character is promoted and Premium is current, checked at each soul
  gain or regeneration step, and 100 otherwise (D72, D76).
- After a demotion, current soul above the new maximum stays until spent; regeneration never raises
  soul above the maximum. Nothing is clamped.

### 4.4 Premium blessings (D70)

- Buying Wisdom of Solitude, Blood of the Mountain or Heart of the Mountain from their NPCs requires
  current Premium.
- Held blessings are durable Character state (first player death decision §4.6) and are kept until
  death, whatever the Premium status.

### 4.5 Premium areas (D70, D73, D75)

- Premium areas are content: a Premium flag on regions of the world, with the six free cities
  (Yalahar included) and their areas free.
- Entering a Premium area (a step, boat, carpet or teleport into it) is refused unless Premium is
  current at that moment.
- At login, a character whose Premium is not current and who stands in a Premium area is placed at
  their home town's temple, or at the Thais temple if the home town is a Premium city (tibia.com
  manual). This is the gameplay transition that consumer contract §12 requires for a relocation.
  **Amendment (pending on acceptance of PREMIUM-DELIVERY-0):** the relocation runs only when the
  entitlement itself has ended: `premium_entitlement_ended(account, now)` (PREMIUM-DELIVERY-0 §6)
  is true. A lapsed lease never relocates, although consumer §7 and §8.3 classify a passed
  `authority_valid_until` as `EXPIRED`: when evidence is unavailable, stale or past its lease, the
  character stays where it is, and Premium-area entry and other benefits are refused until evidence
  is current again.
- A character already inside when Premium expires mid-session is not moved until the next login.

### 4.6 Premium spells (D70, D73)

- A spell with `requirements.premium = true` casts only while Premium is current; the check runs at
  cast time in the existing cast order.

### 4.7 Outfits

D61 stays: in V1 every unlocked outfit, addon and mount is usable by everyone. A later decision may
apply the Premium gate to cosmetics.

## 5. Delivery

| Child | Scope | Owner | Depends on |
|---|---|---|---|
| PREM-1 | Premium entitlement consumer fence and surface policies (§4.1), and the snapshot client of PREMIUM-DELIVERY-0 | Game | `PROD-ENTITLEMENTS-01` consumer contract; FND-04 admission; PREMIUM-DELIVERY-0 (#1369) |
| PREM-P | Platform producer: Premium entitlement with operator or test grants, no payment, and the snapshot endpoint of PREMIUM-DELIVERY-0 | **Platform lane, separate repository and authority** | Platform producer contract; PREMIUM-DELIVERY-0 |
| PREM-2 | Promotion state, effective promotion at login, soul maxima (§4.2, §4.3) | Game | PREM-1; progression readiness; spell P3b-2 vitals |
| PREM-3 | Premium-area flags, entry refusal, login relocation (§4.5) | Game (content, Movement) | PREM-1; content pipeline |
| PREM-4 | Premium spell cast check (§4.6) | Game | PREM-1; spell P3b-2 |
| PREM-5 | Promotion and Premium blessing NPC services | Game | NPC service owner; PREM-2 |

**Amendment (pending on acceptance of WHEEL-0; `OTERYN_GAME_WHEEL0_WHEEL_OF_DESTINY_DELIVERY_DECISION_2026-09-30.md` §6.2; owner answer W1 a,
#162 5917665342).** PREM-2 depends on progression readiness (#1143, completed) and spell P3b-2
vitals, not on PREM-1; the promotion service of PREM-5 depends on the NPC service owner and PREM-2,
not on PREM-1. Whichever of PREM-1 and PREM-2 (PREM-5 for the purchase) lands later wires
`premium_current` into the promotion purchase and benefits (§4.2) before Premium is recorded as
delivered. PREM-3, PREM-4 and the Premium blessing service of PREM-5 keep their PREM-1 dependency.

## 6. Rejected options

- **A Game-side "everyone is Premium" switch.** The owner chose the real mechanism (D69).
- **Forced logout on expiry.** It violates consumer contract §12.
- **Changing the durable promotion on lapse.** Global restores the promotion with no fee.

## 7. Decision test

- **Must decide now:** YES. The owner wants Premium now; D66, D67 and SPELL-D5a wait on it.
- **Minimum sufficient:** four benefits, one consumer fence, checks at existing decision points,
  one login transition.
- **Superseding evidence:** a Store purchase path (§32), houses, Premium cosmetics gating, VIP.
- **Deliberately not decided:** payment, the Store, houses, boats and carpets beyond area entry,
  Wheel of Destiny, the exact Premium region map (content).

## 8. Handback

```yaml
result: RESOLVED_WITH_OWNER_DECISIONS
owner_decisions: [D69, D70, D71, D72, D73, D74, D75, D76]
durable_decision_ref: docs/architecture/reviews/OTERYN_GAME_PREMIUM_ACTIVATION_DECISION_2026-09-28.md
resource_values_changed: false
production_authority_changed: false
cross_repository_authority_changed: false   # PREM-P needs its own Platform authority
implementation_may_resume: false
required_fresh_allocation: true
required_independent_review: "exact-head independent review (entitlement consumer, admission, relocation)"
implementation_lanes: [PREM-1, PREM-2, PREM-3, PREM-4, PREM-5, "PREM-P (Platform)"]
required_revalidation:
  - "PREM-1: login succeeds without Premium; a reconnect never restores an expired benefit; a stale or replayed producer decision is rejected by the fence"
  - "PREM-2: promotion benefits (soul maximum, regeneration, death reduction) stop at the first use after lapse and return at the first use after renewal; the displayed vocation and title change at the next login; no fee; soul above max is kept and never regenerates above it"
  - "PREM-3: entry into a Premium area is refused once Premium is not current; login outside Premium relocates to the right temple; a mid-session expiry moves nobody until the next login"
  - "PREM-4: a Premium spell fails at cast time once Premium is not current"
remaining_unknowns:
  - the Premium region map and temple positions (content)
next_action: "#162 validates this exact head, routes the independent review, integrates it, then allocates PREM-1; the owner arranges the Platform lane for PREM-P."
```
