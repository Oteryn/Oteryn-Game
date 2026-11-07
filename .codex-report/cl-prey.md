# cl-prey report

## Files changed
- Added capability 140 Prey runtime transport, client UI state/actions/text, native app panel, and real-codec play coverage.
- Re-exported the existing Prey codec through `session`, wired the additive play command/session/mailbox seams, and removed `0140-prey` from the unsupported baseline.

## Tests
- Added client-ui request/result tests covering every request variant and refusal feedback.
- Added shared play-harness coverage proving real request decode and reply encode/decode through the fake Stepper.
- Validation results are recorded in the final response.

## Not on the wire
- Gold reroll price, wildcard action prices, bonus/grade names, and creature display names are not on the wire, so the panel does not invent them.
- Command-level refusals carry no detailed reason on the wire; the panel reports a generic server refusal. Reply status 8 carries the specific unavailable-loot refusal.
