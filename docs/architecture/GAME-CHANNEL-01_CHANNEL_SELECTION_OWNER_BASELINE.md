# GAME-CHANNEL-01 — Channel Selection Owner Baseline

- Status: **OWNER_ACCEPTED PARTIAL BASELINE**
- Owner decision date: 2026-07-22
- Repository recording date: 2026-08-11
- Gate: `GAME-CHANNEL-01`
- Scope: login/world/character-selection channel UX only
- Source type: `USER_SOURCE`
- Full `GAME-CHANNEL-01` status: **NOT ACCEPTED**
- Does not authorize: runtime implementation, in-world switching, channel orchestration, PvP policy, anti-hopping policy, capacity thresholds, event/reward policy or production rollout

## 1. Purpose

This document persists a product-owner decision that predates the current `GAME-CHANNEL-01` pre-decision dossier and was not yet recorded in canonical repository architecture.

It intentionally freezes **only the already-decided selection UX**. It must not be used to infer answers to surrounding channel-policy questions.

## 2. Decision timing record

### Must this decision be recorded now?

**YES — because the owner already made it and downstream design must stop treating this narrow scope as open.**

This task does not create a new decision. Recording the 2026-07-22 decision now is necessary because the current `GAME-CHANNEL-01` pre-decision dossier still presents the player-visible/default channel-selection model as an owner choice. Leaving the accepted selection UX outside the repository would allow a later task to ask the owner again or to choose a conflicting separate-login/manual-only model.

### Concrete downstream work constrained by this baseline

The partial baseline must be consumed before these scopes freeze their relevant UX/flow behavior:

- completion of the selection/visibility portion of `GAME-CHANNEL-01`;
- channel-selection/admission UX in `ALPHA-CLIENT-01` or equivalent native-client work;
- any multichannel admission/selection flow used by `VSL-MULTICHANNEL-01`;
- any login/world-selection design that could otherwise model channels as separate login-server identities.

It does **not** block single-channel admission, movement, combat, persistence or recovery slices that do not expose a player-facing multichannel selector.

### What becomes expensive if recording is delayed?

- client navigation/state may be designed around separate per-channel login-server concepts;
- world/character selection may omit the accepted automatic/recommended path or manual `Change Channel` affordance;
- tests may encode the wrong entry flow and later require rewrite;
- future architecture discussions may repeatedly spend owner attention on an already-decided question;
- routing/admission UX may couple channel identity to authentication more strongly than intended.

### Evidence that could justify superseding this baseline

Because this is an owner product decision, evidence may justify **proposing** a superseding decision but cannot silently supersede it. Relevant evidence could include:

- usability/playtest evidence that the selection model causes material player confusion or friction;
- measured party/social co-location failures showing the recommended/manual presentation is inadequate;
- measured queue/capacity behavior that makes the current selection presentation misleading;
- accessibility or platform UX constraints;
- security/abuse evidence showing the manual selection affordance creates an unacceptable risk that cannot be solved by server-authoritative eligibility;
- an explicit later product-owner strategy change.

Any replacement must name this baseline and the exact superseded scope. Implementation convenience alone is not supersession evidence.

## 3. Owner-accepted decision

### USER_SOURCE — accepted 2026-07-22

Oteryn uses **one Oteryn login flow**. Channels are not represented to the player as separate login servers requiring separate authentication/server-login flows.

Within the character/world selection experience:

- the player can see/select channel context rather than treating each channel as a separate game server/login destination;
- the normal path offers an **automatic/recommended channel**;
- the player retains a manual **`Change Channel`** choice.

This is the accepted product-selection model for the declared scope.

## 4. What this resolves in the pre-decision dossier

For this narrow scope, this owner baseline takes precedence over any option/recommendation wording in:

- `GAME-CHANNEL-01_PREDECISION_ANALYSIS.md`;
- `GAME-CHANNEL-01_PREDECISION_CAPACITY_TRIGGERS_ADDENDUM.md`.

The following questions are therefore **not open anymore**:

1. whether Oteryn should expose every channel as a separate login server — **NO**;
2. whether the default user path should require manual channel choice only — **NO**;
3. whether the client may provide an automatic/recommended channel choice — **YES**;
4. whether the player retains a manual channel-selection affordance — **YES**, represented by `Change Channel` in the accepted concept;
5. whether channel context belongs to the world/character selection flow — **YES**.

The parent dossier's broader recommendation of a “soft-visible sticky channel” model remains only partially resolved: the selection-flow visibility/default choice is accepted here, while in-session visibility/stickiness and switching mechanics remain open below.

## 5. Explicitly unresolved

The 2026-07-22 owner decision does **not** by itself establish:

- whether `Change Channel` executes directly while the character is actively in-world or first returns through a safe selection/session transition;
- the exact legal state required to change channel;
- party/friend automatic co-location semantics;
- whether a party leader/member can request `join their channel` while already in-world;
- anti-hopping eligibility/fencing or cooldown duration;
- PvP/combat restrictions beyond constraints already accepted elsewhere;
- channel capacity/create/drain/remove thresholds or orchestration;
- boss/raid/world-event runtime scope or reward eligibility;
- spawn/resource multiplication policy beyond already accepted system ownership;
- queue priority/fairness;
- recovery/failover behavior beyond accepted FND-04 and multichannel safety baselines;
- exact channel labels, numbering or final UI layout.

These remain for the owning `GAME-CHANNEL-01`, PvP/content, `PERF-01`, `OPS-CHANNEL-01` and UX contracts.

## 6. Architecture compatibility

This owner decision is compatible with the accepted architecture because:

- `WorldId` and `ChannelId` remain distinct identities;
- Platform/Gateway authentication remains a single control-plane flow rather than one identity silo per channel;
- selecting/recommending a channel does not make the client authoritative for admission;
- Gateway/World Registry/session admission still decides whether the selected/recommended channel is healthy, compatible and has capacity;
- manual choice cannot bypass FND-04 lease/session authority, capacity, compatibility, combat, transaction or other accepted safety rules;
- choosing another channel never changes the character's logical world/profile family.

No new wire/API/runtime shape is selected by this UX baseline.

## 7. Required downstream behavior

Future `GAME-CHANNEL-01` and client/admission design must preserve the following observable product intent:

```text
Oteryn login
-> authenticate once through the normal Oteryn/Platform flow
-> character/world selection
-> show/use channel context
-> offer automatic/recommended channel
-> retain manual Change Channel choice
-> server/Gateway remains authoritative for whether the requested channel can be admitted
```

A future UI may refine exact labels/layout, but it must not silently turn channels into separate login-server identities or remove the accepted recommended/manual-choice model without a superseding owner decision.

## 8. Acceptance boundary

This document is **accepted only for the declared partial scope**.

It does not make `GAME-CHANNEL-01` as a whole `ACCEPTED`. The remaining owner-decision packet from the pre-decision dossier still includes switching safety/placement, party co-location, anti-hopping, event/reward scope, PvP implications, social continuity, recovery presentation and capacity lifecycle semantics.

Any later decision that conflicts with this selection baseline must explicitly identify the superseded scope rather than silently drifting through client implementation.
