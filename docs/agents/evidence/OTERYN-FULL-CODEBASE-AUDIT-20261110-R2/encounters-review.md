# Encounters — complete package source review

Base: `4f0b6c62a789489fca6c37dd36d5917d096f328e`.

Status: **REVIEWED**. All **93 tracked package files / 22,624 lines** were read substantively, including the manifest, module boundaries, implementations and existing test bodies. `encounters-coverage-validation.json` checks every recorded full-file range against the immutable Git blob and unchanged source checkout. Hashing is evidence preservation, not the review method.

## Entry points and execution boundaries

The package contains pure source-specific rules, native physical-state adapters, source-qualified callback scheduling, and a local combat assembly. Those are separate obligations. Returning a descriptive effect, constructing a source descriptor or reading a fixture does not establish shipping map admission or an executed durable effect.

The complete actual callers `crates/gameplay/dispatch/src/gameplay_transport/endgame_entry.rs` and `boss_activity.rs` were also read. The prerequisite adapter validates current Character, Session, connection, lease and scope authority, then reads progression and quest state. Boss activity checks the capability, current controlled actor, qualified room and content pins, allocation and lever positions. Its participant loop checks endgame prerequisites for **every** member before constructing the native handoff request. The one-member pure `enter` call is therefore not evidence that other party members bypass access checks. Defeat and timeout commands request action from the authoritative owner; they do not supply proof of a kill or elapsed deadline.

## Native owner and state-transition review

| Area | Substantive checks and result |
|---|---|
| Endgame admission | Unique bounded roster, source-specific level/access/chain gates, canonical aliases and retry policy; native transfer, leases, revisions and transactions remain in the actual handoff owner. |
| Bone Overlord | Exact five-profile registry and map placement association; four-cage shared health, bounded non-overlapping instance registration, equal staged damage, projected-death phase tracking and phylactery lock. Existing source explicitly identifies PROJECT behavior. |
| Crystal deaths | Full activation/server/client/frame/map pins and current work stamp; intrinsic summon exclusion; physical death projection before separate callback results; bounded Rum-to-Weak-Spot secondary deaths; Tentacle callback requires the real Map binding; Herald grants use actual top damage and durable quest ownership. |
| Boreth / Rum / Tentacles | Administrative removal avoids death rewards; exact source identity, position, generation and replay guards; Rum freezes bounded target/draw results; Tentacle paths and final-step behavior were traced without inventing map authority. |
| Threshold healing / Icicle | Exact source profiles and occurrences, current actor qualification, health thresholds, delay/cooldown state, bounded owner records and canonical state mutations. Delayed threshold work is dispatched independently of a new successful chance roll. |
| Smelly Cheese | Exact source and self-AI proposal; separate 500ms damage and 700ms presentation; one-use timer pulses and monotonic occurrence; real map/Earth combat facts, PZ/visibility/immunity protection and preparation before mutation. Ambiguous top-creature stacks are explicitly refused. |
| The Welter | Exact three-profile/source registration, full content pins, nonzero monotonic occurrence, same-floor eligible-prey census, allocation before native consumption, capped healing and no synthetic death reward. The dedicated two-second driver and receipt-backed speech were read; its existence alone does not establish production activation. |
| Time Guardian / seven callbacks | Exact source definitions and canonical AI proposal; prepared profile bundle before physical phase change; current pins/epochs; 30-second return timer retained until successful commit. Other map-dependent cases return descriptive pending work. Single-actor profile swap is an explicit PROJECT policy. |
| Cast callbacks | Nine fixed sources, typed child effect/formula conversion, one to three bounded stages, monotonic parent occurrence, cloned timer admission, current-world damage and separate administrative-removal results. No general Encounter interpreter is claimed. |
| Spawn callbacks | Sixteen fixed source adapters, current map position/terrain providers, child policy and health membership, intrinsic versus masterless lifecycle, placement preparation before mutation, per-child results and cleanup, successful-cast counters and source-specific cooldowns. |
| Native combat assembly | Loader membership before registration; current source roles and target inheritance; bounded profile shape; child dependency closure; summon registration cleanup; independent melee/callback/condition/movement work; exact native-condition replay binding; pacing only after an actual move. Unsupported and deferred effects remain visible results. |

The speech mailbox was reviewed through encoding, enqueue and drain. It uses exact historical speaker identity/generation, fixed source text, current recipient Session/control and hearing checks, bounded per-recipient queues and a drop marker. This is transient delivery, not a durable notification guarantee.

## Pure source rules and policy qualifications

All Rotten Blood rule and test files were read, including area counters, party hazard caching, Bakragore essences and echoes, Chagorz agony, Elder target priority, Ichgahal/Murcion/Vemiath waves, pillar lifetime and mushroom fuse/explosion behavior. Draw validation, stable ordering, elapsed-time boundaries, success-only state changes and source-defined limits were assessed. Existing tests explicitly identify the accepted PROJECT Gloom damage difference; it is not reported as a defect merely because donor arithmetic differs.

Primal Menace scheduling was checked for minimum participating-player hazard, interval scaling, strict deadline comparisons, bounded placement attempts, warm-up effects, delayed spawn and clone-before-publication on invalid draws. The actual dispatch tree contains a separate Primal owner; deployment was not inferred.

Soul War taint teleport rules were read through zone/safe-zone selection, stable farthest-target choice, selected-target cooldown, delayed destination validation and expiry. Goshnar's Greed, Oberon and the summer source descriptors were also read in full, including their tests and declared source restrictions. Pure effects and retained definitions remain distinct from physical or durable execution. Searches that locate only definitions/tests receive no independent production-reachability credit.

## Findings and verification

**No additional confirmed finding** was established by this complete package pass. Existing cross-module findings remain in the central register; this result does not remove unresolved runtime questions or assert that every source descriptor is active.

Six existing endgame-entry tests passed through a scratch `rustc --test` harness that imports the **unchanged original source** by absolute path. The exact command, source digest and captured results are in `encounters-entry-unchanged-tests.json`. No replacement implementation or stub was used.

Other test bodies were reviewed but not executed. Several native tests explicitly require a retained eleven-document capture, qualified manifest or physical fixture. No full Cargo build, PostgreSQL run, native client/server exchange, live configuration check or deployment was performed. The authored-attack histogram in an existing test is a source baseline, not a newly executed coverage result. These limits keep the package at **REVIEWED**, not **VERIFIED**.
