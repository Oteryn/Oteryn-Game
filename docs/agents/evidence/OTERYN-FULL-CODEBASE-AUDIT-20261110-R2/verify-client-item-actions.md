# Independent verification — client item actions

Input: `Oteryn/Oteryn-Game@4f0b6c62a789489fca6c37dd36d5917d096f328e`.
Reviewer: lead, independently of the client-support discovery lane.
Disposition: two distinct P2 defects, **CONFIRMED by static production caller/state proof**, deferred to a separate implementation task. This audit makes no source changes. A native client/Session integration test was not executed.

## CL-001 — current open-container sources fail runtime revalidation

The real action-bar caller selects a visible item from both the inventory and open container views (`apps/client/src/features/local-spell-action-bar/item_actions.rs:23-105`). For a self or current creature target it constructs `Action::UseOn(source)` with a current `Selection`. The custom-hotkey caller has the same supported visible-item source (`apps/client/src/features/local-reference-preferences/custom_dispatch.rs:199-258`). The minimum-count selection can choose a nested stack even when a root stack also exists.

The production command dispatch reaches `world_context_run`, then the native `Stepper` adapter, then `oteryn_client_runtime::world_interaction::send` (`apps/client/src/features/0011-world-interaction/protocol.rs:13-36,65-82`; `apps/client/src/play/session_adapter.rs:311-318`). This is not an unused helper or a test-only route.

Runtime `state::current` checks the UseOn source through `item_current` (`crates/client-runtime/src/features/0011-world-interaction/state.rs:9-15`). That helper checks direct inventory entries and world map/spatial items, but never open container views (`:97-115`). A source present only in an open nested carried container therefore fails. The runtime returns `Target changed` before writing the command (`protocol.rs:73-84`).

Counterguards checked: domain 14 updates `Session::container_views` separately (`crates/session/src/container_tree.rs:32-75`); it does not flatten nested items into domain 9. The server's inventory projection covers direct backpack entries and equipment (`crates/gameplay/dispatch/src/gameplay_transport/item_view.rs:91-130,895-929`). The server's current-authority item-use path explicitly allows carried nested inventory sources after its guards (`crates/gameplay/dispatch/src/gameplay_transport/item_use.rs:548-557`). Fresh liveness processing and a valid current actor selection do not change this omission. Neither malformed input nor stale/closed containers are required.

Impact: valid potion/rune/use-with action-bar or hotkey requests from an open nested bag fail locally. A direct backpack source is a workaround. Unauthorized server mutation, item loss and universal item-use failure are not established. The cursor-refresh adapter uses the same helper, but the confirmed core finding is the actual UseOn send path; no additional finding is counted for this sibling call.

Existing app assertions in `features/local-reference-preferences/tests/custom_interfaces.rs:155-229` exercise queue construction for open-container actions. They do not execute the runtime revalidation that rejects the command.

## CL-002 — plain Use custom hotkeys lack the required world selection

For `ObjectTarget::Use`, the production custom-hotkey builder initializes `selection = None`, chooses `Target::Item(handle)` and `Action::Use`, and sends that command unchanged (`apps/client/src/features/local-reference-preferences/custom_dispatch.rs:199-258`). The same actual dispatch chain above reaches the runtime.

Outside the special UseOn branch, runtime `state::current` requires a world selection and returns false for `None` (`state.rs:17-19`). Thus even a current direct backpack item is rejected by `protocol.rs:79-84`, before the legitimate Use/Item encoder at `:112-118`. Selected capability 4 and a fresh handle cannot satisfy the earlier selection guard.

Impact: the plain custom Use item hotkey fails locally. Other inventory UI routes are not established as broken. The app test above asserts the queued Use command, not successful runtime execution.

This is distinct from CL-001: adding open-container lookup alone cannot fix a plain Use command with no selection, while supplying a selection alone cannot make an omitted nested source current. They can be repaired in one future item-action work package, but require separate positive/negative regressions.

## Verification limits and proposed implementation follow-up

The relevant actual caller, data stores, rejecting guards, server acceptance conditions and existing test assertions were independently inspected at the exact input SHA. No native UI, TLS, server, SQL or real Session test ran. Runtime/deployment prevalence remains unknown. A later implementation should exercise both routes through the actual runtime send boundary, retaining rejection of stale/closed/mismatched handles rather than bypassing freshness validation.
