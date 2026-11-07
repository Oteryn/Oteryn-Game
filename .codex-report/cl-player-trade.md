# cl-player-trade

- Added capability-60 runtime command transport, session-layer wire re-export, and authoritative state/result decoding.
- Added client UI state, every wire intent (offer, accept, cancel, look-own, look-counter), refusal feedback, plain-text state rendering, Ctrl+T opening, and a minimal Windows panel.
- Added the native play adapter, mailbox/session command routing, fake-peer play test, codec round-trip coverage, client registration fragments, and removed the unsupported baseline entry.
- Not on the wire: partner discovery and a selectable backpack-item catalogue. `Offer` therefore accepts the exact entity reference and live item handle supplied by world interaction; no client-only identity was invented.
- Tests: codec round trips, UI request/refusal behavior, and an end-to-end play-session fake peer that decodes real requests and returns state decoded from real encoded wire bytes.
