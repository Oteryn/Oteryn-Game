# cl-imbue report

- Wired capability 68 / command 85 through session, client runtime, client UI, and native client play layers.
- Added panel state and controls for query, apply, and clear, including server state and refusal feedback.
- Added client UI unit tests and a shared play-harness test using the real request and reply codecs.
- Removed `0068-imbue` from the unsupported-client baseline.
- Not on the wire: success chance and protection charms; neither is displayed or offered.
- Validation results are recorded in the final response.
