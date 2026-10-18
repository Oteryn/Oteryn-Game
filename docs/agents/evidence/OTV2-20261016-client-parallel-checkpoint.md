# Client parallel continuation checkpoint — 2026-10-16

Task OTV2-20261009-client-game-ui; issue1927; draft PR1942.
Canonical branch feat/client-completion-20261009, predecessor6d1a7b1fc0a06eb3b54d89b1651f0de97c061795.
This is retained evidence, not server allocation or a new architectural mandate.

## Current bounded authoring batch

Owner explicitly requested maximum parallel workers. Root remains the ONLY canonical
writer/publisher; five isolated sparse proposal worktrees and one read-only server
trace/reviewer were used. Root serialized integration and owns the sole Rust/native
build slot. No worker published, built, accessed a host, changed accounts or server code.
PR was explicitly returned to AUTHORING before this batch. Routing packet validated
ALLOW with distinct branches/worktrees and a shared build lease.

- Session-to-HUD conversion excludes own actor by identity through WorldEntities::others().
  A real Session::admit regression retains a different Player on the same tile.
- Both battle consumers must use exact admitted membership, current floor, viewport
  and streamed roof visibility; visible Player/NPC/zero-health rows are not confused
  with attack permission. No row-click attack semantics are invented.
- Equipment unavailable is not empty; capability4 backpack absence/empty/populated
  and empty open-container states are distinct. Inert paper-doll controls are disabled;
  existing move/use menus and session-generation fences remain unchanged.
- Graphics displays read-only backend and adapter from the renderer actually held by
  the native shell. Refresh/clear on redraw covers login, HUD, recreation and settings audit.
  Saved engine intent cannot replace actual identity; engine selection remains disabled.
- Help Info shows compiled RELEASE_ID as well as package version.

Validation and native qualification for the successor are recorded in the active
task and PR after execution; no predecessor proof qualifies a changed binary.

Local batch validation: full client239/230/5, renderer41, strict all-target Clippy,
workspace-boundaries, full workspace fmt, governance and124 governance tests pass. The actual
Session fixture is RED under old iter() (own actor present), GREEN under others().
Initial authoring errors (unsupported tokio macro, old egui CentralPanel API,
missing test import) were corrected without dependency/API expansion. Full UI
then exposed two tooltip-test interaction-order failures and missing engine-choice
probe; actual hover now respects egui's post-click100ms guard, and the engine
semantic contract checks visible Unknown rather than a fictitious saved choice.
No forced-visible tooltip, delay override or relaxed enabled/membership check.
Independent review caught a second battle consumer in the sidebar; it now uses
the same predicate with its own actual rendered-content test.

Owner-PC readiness read: prior test24116 again has one live7172 connection;
it was left untouched while ordinary source/build work proceeds. Owner answered
the batched question and explicitly approves normal close after successor build.
No new owner question remains. Exact native successor still requires qualification.

Read-only next-batch finding: ChatRoomSet is actual open rooms, not a feature/private
availability bit. Closed rooms may remain selectable for Open channel, but all send
paths should preserve draft/history when that authoritative room is closed.
Current server denies non-Say intents; no relay/private functionality is claimed.
Keep this independently reviewable consumer repair out of the present five-finding batch.

## Last qualified predecessor (historical, not successor evidence)

6d1a7b1f Windows1.95: client236/227/5 + Session88, strict Clippy/fmt/release exit0.
Immutable exe SHA256 3bfb7422a0370d4f73d9fa47ad1a2bdebf4d78f28091c5f8ed1df1c3358ccbbb.
DX12 settings audit exit0:20pages/78 independently rehashed captures/0invalid.
Audit manifest SHA256 f48ff4ae1f35fffbbe1295f4ca368337d5520117daae65f86b6f1f5dd3cf823e.
Normal isolated QA browser PKCE→both character rows→explicit Client Probe Redcde
selection→ServerAccepted real map→received Local chat→physical default-profile D/A
movement and server map return→profile Ctrl+L server-committed logout→welcome/TCP0.
Own vitals absent, inventory/container/chat/entities present; capabilities4,6,7,13,17,18,21.
Equipment capability12 is NOT selected; inventory presence is not populated-equipment proof.
Receipt SHA256 474a0ea83b0d62de29c992c0cc2f9b67ae226856e3bc778edb518e418cf54fdf.
The owner-PC normal process24116 was last observed at welcome after logout.

## Server-owner / architect request

# CP/architect handoff: real own vitals for no-vocation admissions

Evidence-only packet from read-only worker `vitals_producer_trace`; no server write allocation, deployment, DB mutation, schema or contract amendment is requested by this file.
Source inspected: client candidate `6d1a7b1fc0a06eb3b54d89b1651f0de97c061795`.

## Proven source chain and remaining live unknown

- `apps/game-server/src/durability/character_build.rs:92-99`: an absent durable build row has the chain seed `none`, zero magic and minimum skill levels. This is deliberate, not a corrupt Knight row.
- `apps/game-server/src/gameplay_transport/spell_character_facts.rs:68-116`: root identity/world/account/revision and progression validation occurs first. `:90-94` returns `NoVocation` before progression-required class parsing or maxima calculation. Existing tests `:234-274` deliberately admit that state even without a progression row.
- `apps/game-server/src/gameplay_transport/mod.rs:4650-4668`: the no-vocation result becomes `facts=None`; `:4841-4861` creates `PlayerSpellState` only for `Some(facts)`.
- `apps/game-server/src/gameplay_transport/actor_spell.rs:1004-1012`: `observe_vitals` returns the state from `ChannelSpellStates::get`, or `None`. The getter `:824-833` additionally checks the actual runtime player/session binding.
- `apps/game-server/src/gameplay_transport/connection.rs:1471-1475,1525-1536`: the initial snapshot only includes ACTOR_VITALS if the authority supplied a value. Domain 3 is not capability 1 (Bestiary).
- `apps/game-server/src/gameplay_transport/actor_movement.rs:49-62,159-174`: no-state admission has an explicit existing baseline movement path. Successful movement does not imply the vitals owner was initialized.
- Native observation supplied by lead: QA `Client Probe Redcde` enters, moves, chats and logs out; selected caps `[4,6,7,13,17,18,21]`; vitals absent while other projections present. Its live vocation/progression have NOT been proven. Source chain is a sufficient explanation only if that actor takes this branch.

## Accepted owners and content, not a new design mandate

- `docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json:467-485`: domain 3 ACTOR_VITALS is owned by current ChannelRuntime, with existing v1 snapshot/delta payloads.
- `docs/architecture/OTERYN_PLAYER_SPELL_CAST_WIRE_AND_VITALS_CONTRACT_CANDIDATE_V1.md:173-185,253-258`: accepted SPELL-D2/D5 assigns current/max pools to runtime actor and Character-owned level/vocation + accepted content; clients do not supply these inputs. A new actor initializes at its maxima; existing actors must not be refilled by reconnect.
- `tools/content-schema/spell-authoring/samples/vocation-vitals-candidate-2026-09-28.json:10` already includes `none` with its provenance. HP is `5*level+145`, MP `5*level+50`, free-account soul maximum 100. These are content-derived values, not permission to assume an unknown actor's level. Reading that existing profile needs no substitute class.
- `docs/architecture/reviews/OTERYN_GAME_CHAR_VITALS0_DURABLE_VITALS_DECISION_2026-10-13.md:50-89` (ACCEPTED) owns capture/restore under CHAR-POSITION fencing: paired HP/MP, same ordered writes, existing actor preserved, new actor capped restoration, no new wire message.
- Historical CHAR-VITALS-1 record lists `actor_spell.rs`, admission composition, last-position projection and durable tests as owned files, but its stale local lifecycle does not allocate a successor or release those paths. CP must resolve live ownership and allocate the repair.

## Why simply reading the `none` curve is insufficient

`CharacterCastFacts` (`apps/game-server/src/spell/cast.rs:83-92`) requires a concrete `Vocation`; `Vocation` (`spell/mod.rs:73-102`) has no no-vocation variant. `PlayerSpellState::new` (`spell/cast.rs:241-279`) simultaneously installs pools, caster facts, cooldowns, monk/condition state and speed; `caster()` (`:498-523`) then projects those facts as actual cast authority. `load_owned_build` (`:188-205`) requires exact vocation and magic equality. Substituting Knight is both false data and potentially expanded abilities; merely adding a class variant without auditing these consumers is not a safe two-line repair.

The missing-progression case also needs an explicit accepted bootstrap source. `durability/character_progression.rs:479-491` defines D88's fenced initial level-1 state, but its presence is not proof it was run for this live actor. Actual ordering is material: `gameplay_transport/mod.rs:4070` invokes `initialize_first_entry` (which reads cast facts), then `:4112` invokes `admit_progression_session` only for a positioned/reconciled actor. That function (`:1933-1974`) returns immediately if the active World generation has no progression section; otherwise `character_progression_binding.rs:171-179` initializes a missing row only at Character revision 1, using the existing fenced D88 path. Accepted `docs/architecture/reviews/OTERYN_GAME_ARCH_PROGRESSION_SOURCE_PACKETS_2026-10-05.md:279-339` owns this binding/initialization and explicit unbound cases. Thus owned progression can be absent during first-entry facts loading yet become available before input eligibility. Do not silently hardcode level 1 for arbitrary no-vocation characters. CP/architect should confirm a safe explicit readiness/initialization ordering under these accepted owners, including a World without progression content and a root past revision 1 with no row. Never mutate a durable row during a projection read.

## Smallest architectural choice to confirm

Keep ONE authoritative actor pool state and revision in the current Channel owner. Separate the readiness of the actor's real vitals from the optional readiness of its class-specific cast inputs. An explicitly typed no-vocation/vitals-ready state must be observable and captured without manufacturing `CharacterCastFacts`; entering a cast/rune/stance/monk/training path must still require its existing class readiness. Prefer a bounded extraction of common pool inputs/state with optional cast facts inside the existing owner, NOT an additional UI-only or parallel server cache of displayed HP.

This is an architectural choice for the owning server lane to approve and implement, not a new contract declared by this packet. A vitals-only diagnostic projection that never participates in damage or persistence is NOT a completed repair. Conversely, blindly making the state visible to all current `states.get` consumers would activate unrelated class behavior. Review all getters and distinguish pool access from cast-capable access.

## Exact files and scope boundaries for allocation

Mandatory source seams:
1. `apps/game-server/src/gameplay_transport/spell_character_facts.rs`: consistent Character-owned vitals readiness/maxima, explicit no-vocation handling, preserved root/progression fences.
2. `apps/game-server/src/spell/cast.rs` and, only if a typed fact representation requires it, `apps/game-server/src/spell/mod.rs`: separate pool readiness from concrete caster identity; keep one revision and pool truth.
3. `apps/game-server/src/gameplay_transport/actor_spell.rs`: actual runtime/session binding; initialization, observation, damage/heal/death/respawn, capture and reconnect; class-restricted getters remain fail-closed.
4. `apps/game-server/src/gameplay_transport/mod.rs`: admission integration with fresh authority, existing lock ordering and restoration; do not gate core vitals on optional casting.
5. `apps/game-server/src/gameplay_transport/last_position.rs`: capture actual pools under existing ordered fence; preserve distinction `Some(None)` (seen at maxima) vs `None` (unavailable).

Read/ordering seam, change only if explicitly allocated: `apps/game-server/src/gameplay_transport/character_progression_binding.rs` and its existing PostgreSQL support scenarios. Its accepted root-at-revision-one initializer is not a generic backfill permission.

Required consumer sweep / allocate only concrete affected edits: `actor_movement.rs` (baseline movement must survive), `ordinary_combat.rs` (targetability/damage must not become fake or invulnerable), and spell/condition/food regeneration/party/stance/entitlement/death consumers of `PlayerSpellState`/its facts. Do not pre-allocate the whole server as speculative work. `connection.rs` already sends actual Some(vitals); transport changes should be unnecessary beyond regression fixtures. No registry/proto/migration/schema/Platform/client synthetic-pool change is indicated by this defect.

## Required focused RED -> GREEN and authority regressions

- A real no-vocation admission with accepted Character facts emits existing domain-3 vitals from the owner; source-derived maxima change with owned level, not client input. Test actual first-entry + snapshot, not only a handcrafted codec value.
- Missing progression/bootstrap readiness follows the explicitly confirmed owner rule; classed missing progression, malformed/foreign/future progression, unknown vocation, changing root and cross-account/world identity remain unavailable/fail-closed.
- No-vocation still cannot cast class spells, rune paths requiring readiness, class stance, monk powers or mana-training merely because HP now exists. Wrong or stale caster facts cannot bypass the class gate.
- Existing present actor on reconnect/retry/new-session recovery retains damaged HP, spent MP, cooldowns/conditions and monotonic state revisions; a stale GameSession, replaced actor, generation or Channel/World cannot observe/mutate another binding.
- Actual damage, healing, death exclusion and respawn are consistent with observed pools (not separate display state); baseline movement and blocking remain intact. A rejected cast has no pool/revision change.
- Saved HP/MP restore only on a newly created actor positioned from the same World row, capped to owned maxima; no resurrection from health 0 and no refill of existing state. Missing/NULL pair, half-pair corruption and foreign-World rows retain accepted dispositions.
- Last-position capture remains one same-owner step and existing session-generation/lease/write-sequence-fenced upsert; vitals-only changes persist, stale/lower/equal keys and foreign sessions refuse, crash/restart preserves accepted state, dead actors are not captured.
- Full exact-head independent server review is required by `apps/game-server/AGENTS.md` for touched authority/persistence seams; no approval, review or deployment is implied by client PR #1942 or this packet.


## Superseded active record, preserved verbatim

The following record is the exact pre-batch active file at6d1a7b1f.
Its pending/Oracle/predecessor statements are historical and superseded by the
qualified checkpoint above, not current failure claims.

# OTV2-20261009 — complete native game client

```yaml
task_id: OTV2-20261009-client-game-ui
issue: 1927
pr: 1942
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
branch: feat/client-completion-20261009
owner: root
```

## Authority and publication state

### World-input companion repair — 2026-10-16 / AUTHORING

7d71a035 fixes character clipping/ZIP; CI supply-chain now passes116942718400.
World job116942718319 and the unchanged local release compiler both reject
stale9d8ccdae inputs after the Cargo.lock patch. Root missed that companion
check before freezing. Explicitly return to AUTHORING, refresh only inputs_digest
to the compiler-derived5ff3f898, then double-build the unchanged7429f13b bundle.
No World content, identity, start, validator or protection change. This is a
material CI repair, not a retrigger. Earlier qualification does not qualify
the successor. Owner has now confirmed the a534 Enter World click; selected
identity remains unbound. Test13344 closed normally by approved WM_CLOSE,
without force. The isolated QA browser again confirms both known characters.

### Launcher / dependency repair — 2026-10-16 / AUTHORING

a5342336 was published and qualified on Windows: client236/226/5 + Session88,
strict Clippy/fmt/release pass. Immutable executable SHA256
da4586db56ef0cbe2e0d1173d6a900b8009f1132b1867d3ea6942749d248d5ef.
DX12 audit validates20 pages/78 captures/0 hash mismatches; the PowerShell
wrapper subsequently failed on a null ExitCode. Manifest success is established,
but native audit exit0 is not claimed. These results do not qualify this successor.

Normal QA token exchange and owner-character read passed on a534. The native
capture showed only Testini, whereas the Platform operator read model has both
QA characters AVAILABLE. A standalone exact-egui0.35 probe and a failing actual
launcher/list regression establish the cause: fixed_size(width,0) clips the
scroll viewport to64px and hides the second row. This supersedes any suggestion
that the one-row capture proves API/projection loss. Public edge routes to the
real authenticated Platform handler; no static owner-character fixture was found.

PR1942 was explicitly returned from FREEZE_SHA to AUTHORING before edits. The
repair bounds only panel width and lets contents determine height. Shared actual
panel/list rendering tests check fully unclipped names and real second-row clicks
in both languages at900x620/1280x720: regression RED, then GREEN. Portable launcher
tests now run on Linux as well as Windows, without enabling a Linux native shell.

Exact-head CI job116934077632 rejects yanked zip9.0.1, used only by client options
archives. Upgrade upstream to9.0.4; Cargo changes only its version/checksum. Stored,
flat, bounded single-entry checks, duplicate/encryption/path refusal, atomic export
and import-to-draft behavior remain unchanged. Local cargo-deny passes all checks;
no advisory allow-list, trust or CI-policy relaxation. Full successor qualification
and fresh same-head CI/native evidence are still required.

A later a534 entry attempt reached ticket/Gateway/TLS and UnexpectedEof before
admission. Its operator/selected character is UNKNOWN: root did not click Enter
World. Injected input stopped; the owner was asked about concurrent interaction.
No ServerAccepted or correct-QA gameplay/chat/movement/logout acceptance follows.
No server/config/DB changes or forced client termination were performed.

### Playable continuation — 2026-10-16 / AUTHORING

e3ca05b1 was published and frozen for qualification, then explicitly returned to
AUTHORING after a verified profile-consumer defect: main's raw Ctrl+L/Q probe
bypassed profile changes while the observed Logout row remained unsupported.
The first regression run now fails2/4 for exactly that missing resolver action
and acceptance of extra modifiers. The repair connects client.logout through
the existing per-chat/profile resolver and native general-action consumer;
dialogs, inactive windows, wrong contexts/modifiers and OS repeats do not logout.
The raw utility probe is no longer used by the native router.

Prior e3ca Windows source qualification passed235/226/5+Session88, strict Clippy,
full fmt and release; executable SHA256
d9989e0567dec155d5498df3996b6362e93a2c591e638759407ec7ae82d9c178.
It does NOT qualify the successor. Git's initial CRLF checkout failed the raw
checksum/fmt preparation; preserved LF worktree bytes match published hashes
and pass. No policy/formatter weakening. Old39716 had already exited before
the owner-approved normal-close driver checked it; no WM_CLOSE/force was sent.
Read-only Platform projection currently confirms BOTH QA characters AVAILABLE;
do not infer that the earlier one-row native capture is current projection truth.

Owner resumed real gameplay work after the admitted Windows session showed blank
vitals/equipment and a chat refusal. Preserve these as distinct observations:
map/Play transition and live TCP were observed after Oracle deployed24fc332f
(includes domain2 fix4689d21f), but the actor of Enter World and the native
character identity were not conclusively bound. An isolated ordinary QA portal
sign-in confirms both Testini and Client Probe Redcde exist on that QA account;
the native selection capture showed only Testini. Do not repeat the earlier
claim that the missing HUD values prove every client consumer is broken.
Private receipt admission-20261016/acceptance-result.json has SHA256
e866c6298c5e5b97a5a86b83546edd692de7863837a5c13f64258a69eedee9f8.
No further injected input after the unexpectedly observed Play transition.

This batch normally merges accepted main24fc332f without rewriting this branch.
Resolve map/receipt/hotkey changes together with LOGOUT-WIRE-1, retaining
capabilities18 and21, paced movement, map USE and terminal/input fences. A committed
logout must rebuild the actual launcher UI, not leave an offline scene posing as
a live session. The logout chord wakes idle play, works with ordinary chat focus,
and stays blocked by dialogs/inactive windows. Unsupported logout does not kill
an older session; a refused exchange still publishes the projections it received.

Seed HUD projections synchronously from the admitted Session before scheduling
play; distinguish an empty admitted inventory/chat from a missing projection.
Add deterministic unpolled-task and terminal/logout-wake regressions. Log only
selected capability IDs and projection-presence booleans at the handoff; do not
log identities, items, resource values, chat, tokens, tickets or grants. This is
startup correctness and diagnostic evidence, not proof that Oracle supplies
vitals, initial items, equipment capability12 or a working CHAT runtime. New
Rune-use refusal variants get explicit non-success feedback without activating22.

Upstream accepted World digest7429f13b and its content remain unchanged; reconcile
only inputs_digest to the merged indexed inputs and repeat the double-build pin
check. Native qualification and real projection/movement/chat acceptance for this
successor remain PENDING. Preserve the currently open client until the owner
answers the batched session-close question; never terminate it blindly.

CI diagnosis at24fc332f: game-gate PASS; Agent governance job116886359002 fails
because six other active packets name merged PRs2003/2116/2016/2006/2045/2224.
Route their archive/lifecycle cleanup to the owning control plane; do not weaken
the validator or edit unallocated task records to hide the failure.

Execution driver correction: the first local routing packet was rejected for
empty optional tool fields and a non-Z timestamp; a merge command was mistakenly
started before checking that result. No further source edit followed until the
corrected packet validated ALLOW. Preserve that failed preflight as a tooling
error, not successful gate evidence. Subsequent execution requires ALLOW.

Draft PR #1942 remains in AUTHORING. Root is the sole branch publisher. Do not
freeze or qualify a candidate until all owned changes are committed, merged
normally with current `origin/main`, validated and reviewed as one exact SHA.
Before publication, fresh-read the remote branch head and use the guarded
publication-integrity helper with that exact predecessor and an external recovery
bundle. Never force, reset, rebase, amend or publish reference assets/private
account data. No production deployment, account mutation or database reset is in
scope.

The owner requires the Tibia Global client to be matched 1:1 and explicitly does
not count a control that merely serializes a value as implemented behavior.
Reference facts are retained in `apps/client/UI-REFERENCE-COVERAGE.md`,
`UI-FIDELITY.md`, `UI-REFERENCE-OBSERVATIONS.md` and
`UI-REFERENCE-SETTINGS.md`. Original captures remain private compatibility
evidence. Missing engine/server mechanisms must remain visibly unavailable and
must not be claimed as complete.

## Retained implementation history

The preceding settings, map, hotkey, receipt and qualification checkpoints are
preserved verbatim in [the retained task snapshot](../../evidence/OTV2-20261016-client-task-history.md).
Use UI-FIDELITY.md and the per-lane active records for exact historical sources,
Windows hashes and unresolved acceptance. Earlier native evidence never qualifies
this successor. Do not recreate completed settings pages or discard their behavior.

## Known completion gaps

- Original Help source and a live SaveFile dialog confirm ZIP export/import
  filters. AUTHORING replaces fixed-path JSON with native ZIP selection and
  atomic Oteryn-only export, with cancellation/error/round-trip regressions.
  ZIP contents remain unknown: the probe produced no archive and the original
  reference process disappeared; do not restart/sign in or copy global identity
  configuration. Schema-only metadata and private evidence are recorded in
  UI-REFERENCE-EXTRACTED.md. Continue offline/independently authored work.

- Spell/Object Custom Hotkeys need authoritative spell/inventory models and the
  observed picker compositions; the existing incomplete editors must not be
  enabled.
- Many General actions and panel interiors still lack genuine consumers or
  authoritative projections. Diagonal movement remains visible but disabled
  because the accepted movement wire has only four cardinal directions; the FPS
  toggle does not yet supply the label's lag indicator.
- Mouse preset/loot semantics, most Console and Game Window filters, graphics
  engine selection/AA visual parity, non-ambient effects, all audio families,
  Misc/Gameplay options and automatic screenshots remain pending.
- Action-bar production assignment sourcing remains absent; Clear/profile-remove
  confirmation semantics and edge-master behavior are not reference-proven.
- HUD condition projection/ordering, native visual comparison, populated module
  states and platform-specific graphics/audio choices remain open.
- Graphics Engine and Sound Device need real platform/runtime enumeration; their
  recorded static rows stay disabled and empty rather than hardcoding one probe
  machine. Factory-default state still lacks independent reference evidence.
- Secure remembered sign-in remains OFF/unwired. Actual sign-in stays system-
  browser PKCE. `protocol-oteryn`, multichannel WorldId/ChannelId separation and
  session-generation fencing remain invariant.

Presence, cardinality, saved intent, generated mockups and static reachability do
not establish 1:1 behavior. Continue in module-sized batches with shared validation.

## Oracle QA checkpoint — 2026-10-15

39ae passes native OAuth/Gateway/TLS; rejects unsolicited map domain2.
Next: owning server MAP producer repair; QA identity/E2E remains unproven.
Evidence/recovery:
../../evidence/OTV2-20261015-client-oracle-login.md.
