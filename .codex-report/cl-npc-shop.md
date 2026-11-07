# cl-npc-shop

## Files changed
- Added capability-81 runtime command/reply wiring through `oteryn-session` and the existing NPC shop codec.
- Added NPC shop UI state, both Look and Sell All intents, result/refusal rendering, registrations, and unit tests.
- Added native-client feature protocol/input/view adapters, Windows list/action controls, session-loop/mailbox command wiring, and a fake-peer play test.
- Re-exported the NPC shop codec and shared enums from `crates/session`; removed `0081-npc-shop` from the unsupported baseline.

## Tests
- UI unit tests cover action selection, pending suppression, reply state, and server refusal feedback.
- Play test decodes real requests and encodes/decodes real replies for Look and Sell All.
- Validation results are recorded in the final task response.

## Not on the wire
- Capability 81 carries no catalogue/open-window projection, item names, or offer discovery; the panel accepts the selected offer identity supplied by the NPC trade UI and does not invent them.
- The wire carries no dedicated panel-open key binding or occurrence identity, so neither was added.
