# cl-vip

- Wired capability 43 and commands 48–50 through session, client-runtime, client-ui, and the native client feature adapter.
- Added protocol request encoding/result decoding round-trip coverage, UI state/refusal coverage, and play-session coverage.
- The reply's authoritative JSON view, revision, and refusal feedback are displayed; all twelve wire operations are exposed by the panel intent API.
- Not on the wire: global character IDs and live online status (the VIP view deliberately carries list-local numbers only; presence is capability 258).
