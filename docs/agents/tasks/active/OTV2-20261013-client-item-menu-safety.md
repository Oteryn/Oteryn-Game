# OTV2-20261013 — existing item menu safety before B2

```yaml
task_id: OTV2-20261013-client-item-menu-safety
issue: 1927
pr: 1942
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
branch: feat/client-completion-20261009
owner: root
```

Owner continuation, sole root writer, draft AUTHORING. Owned runtime:
apps/client/src/game_ui.rs and play.rs. This repairs the already enabled command9
context path before B2; it does not activate later B3 drag/preset/loot features
or bypass architect6146789689's live-gameplay-first sequence. No server, protocol,
dependency, account, deployment, switch, trust or personal-preference writes.

## Findings and repair

Accepted ITEM-CLIENT-1 (architecture packet2026-10-03 §2.7) permits an admitted
open-corpse entry to MainBackpack. Existing context menus incorrectly offered
the same move on backpack contents and paper-doll entries. Those affordances
are removed; accepted open-container entries retain the real typed command.
Missing drag acceptance remains missing, not replaced by this menu repair.

Upstream explicit UiBuilder::id binds widgets to connection generation,
container and item handle, not array indices; the ordinary child-salt API alone
does not stabilize automatic widget IDs when entries reorder. Scoped popup
cleanup closes removed/replaced/closed-container, reconnect, disabled/hidden,
lost-focus and undersized-view intent. A valid reordered source stays itself.
Busy enqueue refusal reports truthful existing feedback and leaves the popup
retryable. PlayLink also refuses a closed command receiver, consistently with
the existing step path. The generation getter projects existing admitted
metadata; no new wire IDs/capabilities/contracts are invented.

## Acceptance and limits

Three actual-egui regressions drive secondary press/release and menu clicks.
A test-local Stepper runs the real play_channel/run_session/send_action chain;
it records exact typed commands without a substituted send closure, sleeps,
network, environment changes or personal data. PL/EN tests cover backpack/icon
refusal, one original-handle move, busy feedback/retry and real receiver closure;
replacement/removal/container close/change, modal/focus and connection changes
send nothing. Received reordering never retargets the neighbour. State remains
unchanged on enqueue and scripted Unavailable feedback; no optimistic item count.
Equipment menus are removed in the production paper-doll path; its inaccessible
cap12 fixture type is not re-exported just for tests and no direct protocol edge
is added. Cleanup fixtures call shared production helpers, not the complete
GameUi.show shell; native equipment-capability acceptance remains unproved.

This qualifies client routing, not wire/live inventory gameplay, authoritative
Moved/domain9/11 deltas, original drag behavior or completed B3. Existing Session
command9/MainBackpack execution is reused unchanged. Locked all-target Linux
client228/203/5+Session82, strict client+Session Clippy, workspace fmt,
governance validation/55tests and diff checks pass. Fixture-only unwrap/borrow
lint findings are fixed without exceptions or weakened gates. Native
source00198640 additionally passed exact native MSVC227/209/5+Session82,
strict checks/release and DX1220pages/78captures/0invalid. Its binary/source
are reverified2026-10-14. New source6f217f16 also executes all three item-menu
tests on Windows, alongside the new step-projection regression; MSVC228/209/5
and Session82, strict checks/release and DX1220pages/78captures pass. Exact
digests, paths and retained attempts are in the2026-10-14 step-projection packet.
These are native component/page proofs, not wire/live inventory or original drag.

## External boundaries

Producer2041 is merged/main376; dedicated QA/item, deployed node and authorized
ON/OFF/rollback/harness/retirement prerequisites remain unsupplied. They are
explicitly visible in updated6149042654. No account/operator action is assumed.
Boundedfe1 CI snapshot:23success/4running/1skip/1superseded governance cancellation,
zero failures. Cancelled38434583341 is replaced by same-head38434726442 through
the workflow's existing edited-event concurrency; no gate weakening or rerun.
Not all-green, frozen, reviewed, MQ-ready, deployed or fully1:1.
