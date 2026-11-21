# Runtime capability fragments

One `<id, four digits>-<name in lower case, dashes for underscores>.json` file describes
each registered capability. This is runtime composition metadata; the accepted contracts
remain in `docs/contracts/protocol-oteryn/v1/registry/` (REGISTRY-SPLIT-1).

- `constant` is the owning Rust module's capability const path, relative to the protocol crate.
  Generated compile-time assertions check its value.
- `requires`, `commands` and `domains` match the contract fragment. `domains: null` is the
  capability 6 exception: its codec gates the extension of core domain 1, not the whole domain.
- `acceptance_requires` controls inclusion in acceptance-message closure validation. Preserve
  the existing asymmetric coverage: 18, 25, 36 and 37 have registry/offer requirements that
  acceptance messages do not currently enforce. Changing this is a separate wire behavior task.
- `offers` declares production fallback, Item view and bundle World membership. Requirements
  are identical in each offered set. Per-connection weather filtering remains in dispatch.
  `production` names the fallback without a non-empty active Item index; it is not a
  shipped-game availability flag. The shipped Synology topology enables the pinned World
  bundle and stages `content/spells.manifest.json`, including its Item keys. Its offer uses
  `bundle_world`, where field runes (91), coin conversion (171) and their prerequisites
  are enabled. The server library test `shipped_world_offers_field_runes_and_coin_conversion`
  activates that manifest and checks the shipped scope offer and dependency selection.
- `supersedes` records the existing MAP-WIRE-1 core-domain replacement (18 supersedes 2).

`build.rs` scans the directory, checks exact registry coverage, filenames, unique ids/names,
requirements, command/domain ownership and offer closure, and emits sorted Rust into `OUT_DIR`.
It never invents reserved or missing ids. The registry JSON aggregates remain unchanged.
No generated file is committed. Dispatch materializes its existing `OfferedCapability` arrays
from the generated protocol pairs in a `const fn`; there is no runtime parsing or allocation.

## Capability 23 transport prerequisite

At `9306b4eadb1209754d65ebd33c31c12291cce1fd`, keep all `PRESENTATION_V1` offers
disabled. The generator rejects enabling any offer set until the server transport is verified:

- `connection/serve/outbound.rs` emits movement magic effects in domain 19, and
  `connection/serve/refresh.rs::sounds` emits committed effects/sounds. These are partial paths.
- `connection/serve/snapshot.rs` does not install domain 19 or 20 snapshots.
- `spell_presentation_cooldowns.rs::CandidateCooldownPublisher` is candidate-only, uses
  `spell_presentation_candidate` payloads, and has no production connection caller.

The paths above are relative to `crates/gameplay/dispatch/src/gameplay_transport/`.
Before removing the generator gate and enabling offers, verify real committed combat events
and own-actor cooldowns through negotiated connection frames, initial snapshots and reconnect,
including capability rejection and revision/sequence continuity. Use the already accepted
`presentation` codecs: `WORLD_PRESENTATION_BATCH_V1`, `WORLD_PRESENTATION_SNAPSHOT_V1`,
`ACTOR_COOLDOWNS_DELTA_V1` and `ACTOR_COOLDOWNS_SNAPSHOT_V1`. No new wire message is needed.
The existing movement/sound paths do not justify advertising the complete capability.
