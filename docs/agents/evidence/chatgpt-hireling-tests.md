# Hireling verification notes

This file records the test boundaries of the E12 implementation. A test listed
below is an executable assertion, not a claim that the latest branch has passed
it. The coordinator records the actual PostgreSQL, compiler and lint results in
the pull request after running the synchronized worktree.

## Validation checkpoints — 2026-11-01

These are the coordinator-confirmed terminal results. Each belongs to its named
checkpoint; passing results from different trees are not one final-head run.
The investigation sections below retain the earlier failures and repair history.

| Checkpoint / scope | Recorded result |
| --- | --- |
| `PG24`: `hireling_postgres_house_position_ordered_capture_and_same_actor_resume` | PASS, 16.46 seconds. |
| `PG24`: `hireling_postgres_house_logout_final_capture_and_authenticated_fresh_relog` | PASS, 18.18 seconds, including the authentic command-zero admission acknowledgement repair. |
| `PG25`: `hireling_postgres_store_named_house_lamp_entitlement_and_replay` | PASS, 15.72 seconds, after preserving the authenticated capability selection through the native handoff. |
| Latest focused Entities package rerun | 200 passed. The earlier invalid-UUID fixture was corrected using the existing native wire-ID pattern; no production validator was weakened. |
| Earlier complete 11-package run (`Unit11`) | 4,532 passed, one failed, 16 pre-existing ignored. Its one failure was the Entities fixture above. The later focused rerun does not turn this historical attempt into a green run. |
| `PG25`: full `hireling` filter with PostgreSQL 18.6 | **38 passed, zero failed, zero ignored**, 179.34 seconds of tests, 184 seconds overall. Exact inventory: **20 lifecycle PostgreSQL cases + ten Store PostgreSQL cases + eight unit cases**. All 30 PostgreSQL completion markers are present; this is not a 38-PostgreSQL-case suite. |
| `PG26`: full `hireling` filter after the final import/style repairs | **38 passed, zero failed, zero ignored**, 283.40 seconds of tests, 501 seconds overall. The full 164-line log matches all 38 expected names and contains 20 lifecycle plus ten Store PostgreSQL completion markers. |
| Full Dispatch rerun in the `PG25` script | **1,760 passed, zero failed, 14 pre-existing ignored**, 157.79 seconds of tests, 161 seconds overall. |
| Static checkpoint: `cargo fmt --all --check` | PASS, 13 seconds, before the subsequent import/style repairs. |
| Static checkpoint: architecture / workspace boundaries | PASS, 13 seconds. |
| Client UI `registrations` target | **Two passed, zero failed, zero ignored**, 0.03 seconds of tests, ten seconds overall. This is additional to the library-only counts below. |
| Final Clippy for the 11 packages | **PASS**, exit zero, 136 seconds, with all targets, all features and `-D warnings`. Remaining printed warnings came from vendored dependencies; the changed packages passed the requested gate. |
| Doctests for the 11 packages | **48 passed, zero failed, zero ignored**, 93 seconds overall. These are additional to the library and registration counts. |
| World-bundle `pin-check`, final-static checkpoint | **PASS**, exit zero, 29 seconds; 30,953,570-byte bundle. The exact inputs and bundle digests are recorded below. The data/source generators were unchanged from the previously passing Python checks. |
| Latest content REST filter | **123 passed, zero failed, zero ignored**, 2.19 seconds of tests, 110 seconds overall. |
| Latest `content_world_project_repository` target | **Five passed, zero failed, zero ignored**, 57.89 seconds of tests, 74 seconds overall. |
| Default-feature server smoke after the native import repair | **PASS**, exit zero, 99 seconds, with the actual server `process_start` event. No test-support feature was enabled to obtain this result. |
| Governance checkpoint before the result-only evidence update | **PASS: 22 documents / nine lanes and 129 tests**; the same checkpoint passed `git diff --check`. This is local governance evidence, not an exact-head CI result. |
| Final format and diff checks after `PG26` | **PASS**: format check, ten seconds; `git diff --check`, zero seconds. |

The complete `PG25` log was read and its 38 named results matched to the test
inventory. The coordinator recorded this terminal result on PR #2783 in comment
`6555428344`, then confirmed the full Dispatch result above and the complete
`PG25` script exiting zero after 507.93 seconds. The subsequent source cleanup
removed only the unused `FreshAdmissionAuthority` import; logic was unchanged.
The later static/content/smoke results above were independently observed; they
are not inferred from those test runs.

After the final import/style repairs, `PG26` independently passed the same
38-case inventory. The complete log has no missing or extra test names and all
30 PostgreSQL completion markers. The final qualification script exited zero
after 936.61 seconds, including the subsequent passing format and diff checks.

The release-compiler pin check recorded inputs digest
`944661593eecb7e2ac3649b65a32a55ee8b7316a4760ddf3e03d10a9faee6b34`
and bundle digest
`6edcee04dc8dacbb34f735570e32545ff20793f8da37bd1168f94e941fca8622`.

The first Clippy attempt had one `question_mark` style diagnostic after 138
seconds. Its equivalent-expression repair introduced no lint suppression, and
the final Clippy run above passed. The final three cleanup deltas also received
an internal focused review with no P0/P1 finding; that review is not a CI result
or an external review request.

The initial default build exposed stale domain metadata in the allocated build
slot: the native module was exported unconditionally, but the selected cached
artifact's dep-info omitted `domain/hireling.rs`. The coordinator refreshed only
the parent module's timestamp while holding the slot; its source SHA256 remained
`9648171b672e72cafb620cfe9a1b77863c21391438e349b9f20e43dad434499c`.
The resulting dep-info contains Hireling. That real rebuild then exposed a
production import through test-only root aliases. `source_progression.rs` now
imports `CastFactsLoad` and `load_character_cast_facts` directly from their native
`spell_character_facts` module. The root `cfg` boundary and features were kept
unchanged; the default Dispatch/server build and actual smoke subsequently passed.

The earlier package totals comprise 2,573 passing cases in nine unchanged
packages, 199 passing Entities cases and 1,760 passing Dispatch cases. Replacing
the old Entities result with its 200-pass rerun and retaining the confirmed
1,760-pass Dispatch rerun gives **4,533 passed and 16 ignored across the latest
package-specific library runs**. This is an aggregation of identified runs,
not a new full 11-package invocation or a count including registrations and
doctests. A library run without the PostgreSQL URL also returns successfully
from route-less PostgreSQL cases; actual database execution is proved separately
by the allocated PG run.

The published implementation commit is identified by draft PR #2783's head
metadata. These local checkpoint results do not assert exact-head CI
qualification. Accepted-scope code and its self/internal review are complete.
Node-boot remains BLOCKED by its missing pinned Platform checkout/runtime
prerequisites; smoke is not native-client or TCP+TLS journey evidence. Q17 remains
an unanswered Familiar scope decision. The instruction to finish and push does
not answer its A/B question, authorize a new companion implementation or establish
mechanic DONE status.

## Frozen baseline

The original implementation was tested in a separate detached worktree at
`ae4188a9c8d461acb152044814ea23075de02ed4` before the production replacement was
applied. The baseline exposed independent defects rather than relaxing the new
tests to match the old implementation:

- A native summon and its once-only durable replay succeeded.
- Query was rejected as `InvalidStoredState` by the original parameter guard.
- Dismiss deleted the row but returned the deleted coordinates as an active
  result, allowing the transport projection to recreate the NPC.
- The original House wire route rejected the summon command before the intended
  native placement flow. The House command router lacked the relevant lanes.
- The existing Bank context did not recognize the Hireling's actual dialogue
  catalogue.

The frozen fixture also surfaced two capability setup errors. Its bootstrap was
corrected to the existing registry dependencies (`165` requires `3` and `51`);
the baseline production implementation was not changed to bypass them.

## Current PostgreSQL suites

The lifecycle suite is registered in
`crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/hireling_lifecycle.rs`.
The Store suite is in the neighboring `hireling_store.rs` file.

| Boundary | Assertions and native owners |
| --- | --- |
| Native lamp custody | A genuine Inbox mint; actual named-House fence; wrong House, node and lease refusals; consume once; immutable replay; return through the same native Inbox owner. |
| Full Inbox and offline recall | Full custody refusal leaves the live identity and all items unchanged; later native recall uses independent current House authority and authentic original placement provenance; replay creates no second lamp. |
| Migration from `0405` | Preserve the existing placed Hireling and Banker unlock; backfill only a matching real receipt; preserve and report malformed legacy rows in the recovery quarantine; one malformed row does not block valid work. |
| ACL | An admitted guest can use the owner's unlocked service without acquiring the identity; the same unchanged session is refused after access revocation or a content fence; the service transaction holds the actual House property lock. |
| Literal USE and management frames | Door placement refuses without burning the Inbox lamp; real STEP enters the interior; USE, list, outfit, projection and return reach production owners. Replacing the owner refuses the lost started ingress; exact native receipt retries after position and identity changes do not repeat mutations. |
| Command `220` | Real query/summon/dismiss frames preserve their original replies after later state changes; projection follows current durable state. A later owner cannot reconstruct a started command namespace, while original native receipts remain replayable. |
| Bank | One persistent connection performs the real greeting and Bank dialogue, deposit/withdraw/transfer and exact-command replay. Transfer also proves both ledger sides, the named-House audit and native replay outside the wire cache. |
| Trader | A maximum-price sale has an atomic no-room refusal, then six crystal-coin stacks; direct native replay and wire duplicate do not pay twice. |
| Cook | Source-priced random/specific food uses the native fee and Inbox transaction. An exact three-child ingredient package reaches the backpack through the existing trade/container owner. Both replay once; the bank-only food case needs no backpack. |
| Steward | Real dialogue and stash frames use handles from the actual snapshot; native stow/withdraw and replay; unchanged-session ACL revocation blocks further work. |
| Gourmet | Real carried USE `471`, immutable native burn and receipt, shared cooldown, changed-input conflict, stale-writer refusal, decrement-only persisted condition, native runtime restoration and pending-commit reconciliation. |
| Furniture | A source-listed wrapped furnishing buys through NPC trade, retains its target provenance and uses the existing House wrap/unwrap owner; no-room, ACL refusal and old-command replay preserve custody. |
| Pending lamp | An actual committed or uncommitted frozen native plan resolves before new input; unavailable receipt proof closes the connection without a fabricated CommandResult or command advance; known refusal clears only its exact pending occurrence. |
| Store delivery | The separate suite covers entitlement and delivery branches, replay/refusal and a retry after a retained failed native preparation using a new real CommandRef. A further lifecycle case enters an actual named House before buying a lamp and skill; it proves the House Inbox audit, immutable receipt, one native mint and two Platform debits despite replay. |

## Q16: House position and lifecycle

`hireling_lifecycle/position.rs` adds five cases through the same fixture:

1. **Ordered capture and retained resume.** Real House STEP feeds the existing
   LastPosition owner. Cell and living vitals advance together; an unchanged
   capture does not advance the sequence; an earlier actual capture is
   superseded; a stale lease cannot write. A real signed `ClientResume` restores
   the same actor with the same next command ID and a newer connection generation.
2. **Wire logout and fresh relog.** Command `23` captures the current cell and
   vitals in the native terminal transaction. A signed fresh admission creates a
   new GameSession and actor generation and restores that cell. A second real
   terminal commit is observed before runtime retirement to prove that lost-ack
   reconciliation removes the still-present House actor and is idempotent.
3. **Process loss.** The fixture replaces both the entire carrier and its local
   admission owner. An entry receipt cannot reconstruct a started command
   namespace: that resume must refuse without creating an actor or changing the
   saved position. The existing authentic abandoned-controller release ends the
   old session; a genuinely new signed admission then restores the saved cell.
4. **Current ACL loss.** A real gourmet USE is committed before access is
   revoked. Further STEP and fresh placement are refused, but the exact living
   resident can still capture its state and log out. A known refused fresh
   source stays explicitly unpositioned, follows the existing disconnect/grace
   lifecycle and releases its claims. Restoring guest access permits a later
   genuine fresh admission.
5. **Current cell collision.** The native lamp owner places a Hireling on the
   current resident's cell. A retained actor's validation permits that source
   overlap. A new actor's restoration must refuse the currently occupied cell,
   including the native `-7` to runtime `+7` floor conversion, without moving the
   Hireling or manufacturing a fallback position.

The two refused fresh-source cases retain the existing real sixty-second grace
and durable clock. They use a bounded seventy-second test timeout. The fixture
does not edit a deadline, session state, interpretation or claim to accelerate
that lifecycle.

This scope does **not** claim seamless same-GameSession recovery after process
loss once its command namespace has started. The test explicitly preserves the
existing refusal boundary and requires a genuine terminal release and fresh
authentication.

## Fixture authority and content

The integration fixture qualifies the same full native input as its active Item
provider and asserts equality of their server artifact digests. It records that
artifact through `record_content_activation`, using the existing exact-scope
control grant and real server, client and frame digests. Its runtime pin carries
the real Bundle movement-map digest.

Before the House handoff, `initialize_admission_equipment` validates the original
committed FreshAdmission receipt and creates only the existing equipment owner's
empty seed. The House equipment reader remains read-only and does not manufacture
missing equipment state. The actual STEP, named-House membership, current ACL,
equipment, source content, collision and lease checks remain production gates.

At the coordinator's `PG9` checkpoint, the actual first House STEP passed those
gates and moved from `(32309, 32804, 7)` to `(32310, 32804, 7)` with its native
550-ms duration. That checkpoint confirms the fixture repair; it does not stand
in for the remaining lifecycle and service assertions.

## Source-content verification

The exact Crystal `201189f` capture, the shared native-profile merge and the
binding qualification checks are separate from runtime fixtures. The reported
verified pipeline checkpoint contained 2,104 active native profiles and emitted
4,712,466 bytes of compact canonical JSON, below the unchanged eight-MiB limit.
The coordinator reported nine profile tests, thirteen binding tests, exact
capture `--check` and wrapped-target generator `--check` passing.

Closed compatibility decisions preserve the five established native weights and
the `23721` movable fact approved in the pull-request answers. The raw capture
retains the differing Crystal values and source lines; every other unexpected
shared fact difference aborts generation. The lamp remains nonportable under
generic inventory admission and is admitted only through the explicitly bounded
Character Inbox destination. Preserved unsupported source offers retain their
typed runtime refusals.

Physical Trader admission also preserves the existing I05 mechanic boundary for
stealth, life and healing rings (`3049`, `3052`, `3098`). They may have genuine
carried item facts while the equipment policy, equipped view and timed budget
owners still refuse their unhosted effects. The original Crystal parity test is
retained, with a further native-content test proving both this separation and
continued support for time and dwarven rings. No source profile is deleted or
rewritten to satisfy the execution gate.


## Transferred-session control loss and recovery

The first actual House loss distinguishes the native transfer's entry marker
from a completed prior reconnect. Its `ScopeTransfer` history is backed by the
immutable native transfer chain and retains protection from any real prior
recovery. The origin fresh receipt and its transport reservation remain original
provenance; the owning-loss journal, current row update and reconnect records use
the actual destination GameSession. The additive loss encoding round-trips both
identities and keeps the old initial-session bytes unchanged.

Both CompleteReconnect phases independently prove the current named-House
session, claims, node, assignment and ACL. The existing origin publication
supplies compatibility and selected-capability policy, as in native transfer
recovery; it does not supply House ownership generation or replace the actual
House scope. No speculative House readiness record is created.

The ordered-position PostgreSQL case asserts actual destination receipt lookup,
loss codec round-trip and reconciliation, followed by two genuine control losses
and authenticated same-actor resumes. The second loss retains the first recovery
protection and advances its epoch. Both resumes preserve command continuity.
The same case verifies the native House PvP read with valid custody and explicit
refusals for the generic Channel API, a stale lease, another House and a stale
node proof. The coordinator reported both the complete ordered-position case
and the process-loss/fresh-admission case passing at `PG16`. The separate wire
logout, ACL and occupied-cell cases still required follow-up at that checkpoint.

## Bank dialogue and retained confirmation

The real Bank conversation uses the existing exact grammar: `deposit 10 gold`,
`withdraw 5 gold` and `transfer 3 gold to Bank Friend`. It asserts the native
deposit and withdrawal prompts before confirming them. The earlier fixture
omitted the `gold` token and therefore never reached the Bank writer; that
fixture defect was identified against `crates/gameplay/npc/src/bank_npc.rs`
without changing the production parser.

A further native-owner continuation seam retains an accepted resolved deposit,
then performs a genuine loss and authenticated same-session resume. The original
confirmation remains immutable. Its origin connection generation remains
provenance; the native operation separately records the current deciding
connection generation. The deferred House proof binds that generation to the
actual current session while preserving exact House, lease and scope checks.
The case rejects the old connection and a stale lease before committing once,
checks that only the confirmation's completion flag changed, and replays the
same receipt without another balance change or audit. This additional Bank
checkpoint passed as part of the complete Bank PostgreSQL case at `PG18`.

## Coverage audit of the session and authority changes

| Layer | Focused assertions | Observed checkpoint |
| --- | --- | --- |
| Foundation loss history | A transferred current identity preserves the original receipt and consumed recovery protection. Invalid transfer identity, entry/loss epochs and changed protection fail authorization, final validation and historical restore. Same-identity scope relabeling cannot lower its generation. | The focused `owning_loss_scope_transfer_preserves_origin_and_rejects_changed_history` unit passed at `PG17`. |
| Foundation terminal claims | Real transfer successors authorize a destination with an independent House generation. Origin or missing claims, an unproven same-identity House projection and a lower generation in the original Channel are rejected. | The new focused generation/terminal-claim unit passed at `PG18`. |
| Native carrier | Wrong session or owner generation cannot touch a House slot. Repeated loss/fence/reentry is idempotent, does not extend protection and does not admit the Channel API. An obsolete actor generation cannot remove its replacement or another resident. | Authored; final current unit checkpoint remains coordinator-owned. |
| Durable loss and reconnect | Actual destination journal and codec retain both real session identities. Two successive genuine resumes advance connection generation while retaining command continuity and the prior protection budget. Current House/lease/node read refusals do not alter state. | Complete ordered-position and process-loss cases passed at `PG16`. |
| Accepted native Bank work | Old connection and old lease are independent one-field negative fences before the money receipt exists. A separately current resumed fence completes the original pending occurrence once; its origin confirmation stays immutable. | The complete Bank case, including real dialogue and this native continuation, passed at `PG18`. |
| Obsolete commands and lost ingress | Wire duplicates and native receipt retries preserve original replies after newer identity/state changes. Replacing the process owner refuses a lost started ingress instead of constructing a new namespace. Pending-lamp ingress completion matches only the frozen original outstanding command. | Four focused native ingress units passed at `PG17`; the complete pending-lamp PostgreSQL scenario passed at `PG18`. |
| Current ACL and final retirement | Guest/service access is rechecked without changing the admitted session. The separate Q16 case preserves real final position and gourmet state after access loss, then retires the refused fresh source through ordinary grace. | Earlier service ACL coverage passed; the Q16 wire logout/refused-source tail remained under investigation at `PG17`. |
| Once-only native value and custody | Lamp, Cook food/package, Trader, Steward and Bank assertions count native receipts, items/value and audits, independently of the connection cache. | Both complete Cook PostgreSQL cases passed at `PG17`; this is not a claim that all service cases passed at that checkpoint. |

The additional Bank enqueue seam takes the production spell lane and holds the
current runtime NPC/content facts through native acceptance. It releases both
before closing the live duplex connection. The later rejection, commit and replay
checks run sequentially after authenticated resume, without an active serve
loop or deliberately competing database work. Each negative expects the exact
`AuthorityRejected` result; a `ServerBusy`/unavailable result cannot satisfy it.

## Fresh House admission participants and lost acknowledgement

The current follow-up composes the ordinary Character admission participants
while the newly authenticated source is still a real, active Channel session.
Equipment and starter owners use the original fresh request. The existing
position selector retains pending respawn, NPC arrival and tutorial precedence;
only its selected stored House proceeds to the native handoff. Progression,
Hazard, offline-training settlement, Prey initialization, quest/Wheel obligations
and automatic-spell repair complete under the source's actual authority before
it is retired. This does not publish a temporary Channel position.

After the native House handoff, data is installed against the current
destination session, transport, lease, named scope and actor. Lost-ack recovery
uses read-only native Hazard/Prey reads and Character-owned progression, quest,
Wheel, Atelier, Charm and daily-reward observations. It cannot repeat source
settlement or grant generic Instance mutation authority. The retired source's
four value maps are removed only after destination restoration; Premium retains
the actual successor before releasing the source reference. The ordinary resume
tail selects this read-only path only for an actual House actor. Fresh and ACK
recovery require those observations before admission is published. A retained
CompleteReconnect refresh keeps the existing optional-observation semantics:
failure cannot discard an already committed controller or invent a second loss.

The fresh-relog fixture now asserts the one authentic source offline-settlement
marker, absence of a copied destination marker, destination quest/Wheel/Atelier
views and removal of the source views. Before any destination command starts,
it discards only local observations and submits a genuine signed ClientResume.
It then compares the Character revision, native Hazard and Prey rows and receipt
counts to prove recovery did not execute another source write. Stale-lease and
relabeled-Channel reads must return `AuthorityRejected` without changing those
rows. These follow-up assertions were awaiting execution at the pre-recovery
checkpoint; the subsequent fresh-relog case passed at `PG24`, as recorded above.

The same case then creates a genuine native control loss before the first
destination command. Its authenticated resume must record CompleteReconnect
history, clear the actual retained loss and keep command ID one. The transfer
ACK owner checks for that loss while holding the runtime lock before its native
recovery commit, so an unstarted stream cannot bypass the existing loss owner.

The native carrier also has a focused retained-fresh-slot recovery assertion.
Only an authenticated admission-restore receipt can select that branch; it
finds the actual committed unpositioned source slot and uses the receipt's real
source lease. A wrong lease or existing write fence leaves the source
unpositioned. Success reuses the same slot and actor rather than allocating a
second actor or publishing a playable Channel fallback.

## Committed handoff data continuity

The final authored follow-up moves existing actor observations only after the
native custody successor is proved. Preparation and publication hold the same
runtime, spell-state and Attack owners and recheck the real lease increment and
scope change. Pending native work retains its original command and authority;
it must settle before handoff. Live actor vitals, stamina and Prey clocks,
Gourmet remaining time, Item-use cooldowns, Attack modes/defence/fight deadline,
proficiency data and the existing account/session observations follow the same
actor. Contextual targets and conversation windows close at the boundary. The
new session gets a fresh command namespace; only the two combat rate-observation
windows are carried from a genuinely retained admitted source.

The same token path handles an authenticated ACK recovery whose source actor
still exists after native commit. Cold recovery reads existing durable data.
Progression, quest, Wheel and Atelier observations retain their data and retry
deadlines in both real handoff directions. Actual House cadence defers the
Channel-only quest writer without discarding its deadline; genuine Channel
return rearms a missing deadline only for existing pending work. Surviving
Channel maps are not overwritten by repeated admission initializers.

The existing ordered-position PostgreSQL case contains the native exit/data
continuity assertions, including rejection before the actual handoff and with
the old source after it. At the remote-PC outage these changes, the retained
fresh-slot native test and the extra House Store case were authored but
unverified. The later `PG24`, `PG25` and Entities results are recorded above;
they do not replace qualification of the combined final candidate.

The Position-only fixture qualifies the original native training formula under
its existing `content-1` Character interpretation, matching the established
death/relog fixture pattern. The runtime pin, movement map, native provider and
durable content activation are built from that same input. Production source
formula bytes and production admission gates are unchanged.

The coordinator subsequently reported all 346 Foundation unit tests passing
at `foundation-units19`. This confirms that unit checkpoint only, not the newly
authored admission and recovery integration assertions above.

During reconstruction after the overnight scratch loss, the exact existing
quest admission owner exposed one additional duplicate refresh: the outer
Channel resume tail would replace the retry deadline after native transfer ACK
recovery had already carried or restored that observation. The coordinator
approved a one-condition guard excluding that already-completed transfer path.
Ordinary Channel resume keeps its existing refresh. The recovered composition,
including this correction, compiled at the coordinator's `PG23` checkpoint.
That aggregate finished with 35 passing and three failing cases; it does not
qualify the subsequent repairs.

## Native admission acknowledgement

`PG23` reached a genuine fresh House restoration and its authenticated ACK-loss
recovery, then exposed a transport failure. The native receipt's command zero is
server admission provenance. `connection::admit_frame` tried to encode that
value as a client `CommandResult`; the unchanged protocol encoder correctly
rejected zero after the native resume had already committed.

The authored repair requires command zero if and only if the native receipt is
an admission restoration, with an actual House destination. The projection owner
still independently validates the current session, transport, fence, House and
runtime lease. Resume writes only `ServerResumeAccepted` for that closed case;
it neither invents a client command nor advances the command or server sequence.
The existing House join/resync snapshot publishes the same qualified scope in
domain 51. Positive client-command acknowledgements and the protocol encoder
remain unchanged.

The existing fresh-relog regression asserts authentic admission provenance,
command zero, next command one, server sequence zero and exactly one admission
reply. It then exercises the original pre-command control loss and native
CompleteReconnect. No additional command stream is opened to make the test
pass. The coordinator ran the repaired fresh-relog case at `PG24`: PASS in
18.18 seconds. This result belongs to that checkpoint, before the later
capability-preservation repair.
