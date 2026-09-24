# Oteryn native client viewport A/B experiment plan

- Date: 2026-09-10
- Repository: `Oteryn/Oteryn-Game`
- Status: **OWNER-REQUESTED EXPERIMENT PLAN / DECISION DEFERRED PENDING EVIDENCE**
- Extends: `docs/architecture/OTERYN_NATIVE_CLIENT_UI_ARCHITECTURE_BASELINE_2026-09-10.md`
- Runtime/product activation authority: **NONE**
- Addon/mod/plugin implementation authority: **NONE**

## 1. Purpose

The native-client UI baseline intentionally keeps world zoom independent from UI scale and gives the world viewport the remaining workspace after UI layout. One player-facing question must remain unresolved until it is tested in the physical client:

> When the player increases client-window size or display resolution while keeping the same world zoom, should the gameplay field of view reveal more map tiles, or should gameplay field of view remain substantially fixed and the larger screen primarily increase presentation scale/layout space?

This document defines the required A/B qualification. It does **not** select either result in advance.

## 2. Addons, mods and community customization are deferred

Addon/mod/plugin architecture is explicitly **DEFERRED / FUTURE CONCEPT** for the current client/UI implementation wave.

Current work MUST NOT introduce, solely for hypothetical future addons:

- an addon runtime;
- a scripting VM or WASM/Lua choice;
- addon manifests or package formats;
- community plugin APIs;
- addon capability/permission infrastructure;
- third-party UI widget registration;
- hot-reload infrastructure for untrusted extensions;
- addon marketplaces, signing or distribution systems.

The current UI foundation should remain internally modular and use semantic presentation boundaries where already justified by the product, but it MUST NOT incur substantial complexity for an addon platform that is not presently required.

A future owner-authorized architecture task may reopen addon/customization support after the native gameplay client and representative HUD are physically proven.

## 3. Legacy OTClient evidence to preserve as reference, not authority

Read-only inspection of `blakinio/otclient@53646cfa1957ce75f18547424a9e9177c4ad8cc2` shows:

- `MapView` initializes to a visible dimension of `15 x 11` tiles;
- `data/setup.otml` configures `map.viewport: 8 6`;
- `Map::resetAwareRange()` turns that configuration into asymmetric map awareness around the center (`8 left / 9 right / 6 top / 7 bottom`, i.e. an 18 x 14 data extent);
- standard view mode uses `keepAspectRatio=true`, `zoom=11`, `visibleDimension=15 x 11`;
- an extended mode can use `keepAspectRatio=false`, allowing width to follow the map-panel aspect ratio while height remains driven by the zoom value;
- `UIMap::onGeometryChange()` recalculates map presentation on resize;
- `MapView` separates logical visible/draw dimensions from `g_map.getAwareRange()`;
- the protocol supports `ClientChangeMapAwareRange` / `GameServerChangeMapAwareRange` when the corresponding feature is enabled.

Therefore legacy OTClient already demonstrates an important architectural separation:

```text
window/map-panel geometry
        !=
render/visible dimension
        !=
server-provided aware range
```

Oteryn should preserve that separation while testing a better product policy.

## 4. Invariant for both variants

Both A and B MUST keep these concepts independent:

```text
WindowSize
UiScale
WorldViewportRect
WorldZoom
RenderedTileExtent
GameplayVisibility / LOS
ServerInterest / RelevanceExtent
```

Neither experiment may make client window size authoritative for gameplay visibility, LOS, combat legality, creature existence, targeting legality, or server state.

The server remains authoritative over the information delivered to the client.

## 5. Variant A — responsive field of view

### Product behavior

At a fixed world zoom / tile presentation scale, increasing the available `WorldViewportRect` reveals more map tiles.

Conceptually:

```text
larger window or higher resolution
        -> larger WorldViewportRect
        -> larger required RenderedTileExtent
        -> larger requested ServerInterestExtent, when supported
        -> server clamps/accepts according to policy and resource limits
        -> more authoritative map observations may become visible
```

UI scale remains independent: enlarging UI controls is not equivalent to zooming the world.

### Required behavior

- manual live window resize must update the viewport continuously and deterministically;
- 1080p, 1440p, ultrawide and 4K must be supported without one fixed tile-count assumption;
- the renderer derives required tile extent from actual viewport geometry and world zoom;
- overscan/prefetch may be used for smooth movement but must be bounded;
- missing server-authoritative data is never fabricated;
- the server may cap requested interest extent;
- changing resolution must not change gameplay legality by client-side inference.

### Primary risks

- monitor/resolution-dependent information advantage;
- increased server AOI/relevance work;
- increased map bandwidth and client decode/state cost;
- higher CPU/GPU cost from additional tiles, creatures, effects, text and lighting;
- ultrawide edge cases where horizontal awareness becomes disproportionately valuable.

## 6. Variant B — fixed gameplay field of view

### Product behavior

The gameplay-aware/visible world extent remains substantially constant across window sizes at a fixed world-zoom setting. Larger windows/resolutions improve UI workspace and/or presentation scale rather than revealing materially more gameplay information.

Conceptually:

```text
larger window or higher resolution
        -> larger WorldViewportRect
        -> same bounded GameplayFovExtent
        -> map presentation scales/fits within viewport
        -> server interest/relevance extent remains unchanged
```

Implementation may preserve aspect ratio, fit/crop within safe constraints, or use another measured presentation policy. The experiment must not accidentally turn non-uniform stretching into the final product behavior.

### Required behavior

- no material server-awareness increase from monitor resolution alone;
- consistent combat/map awareness between common display profiles;
- resize remains smooth and does not corrupt camera centering or input mapping;
- tile readability must remain acceptable on high-DPI and large displays;
- world zoom remains an explicit player-facing control independent of UI scale.

### Primary risks

- large/4K displays may feel wasteful because additional pixels do not reveal additional world space;
- sprites may become excessively large or the viewport may require padding/cropping;
- ultrawide support may look artificial;
- players may perceive resize as having little gameplay value.

## 7. A/B qualification matrix

Both variants MUST be run through the same deterministic scene and, when available, the same representative physical world slice.

Required display/window profiles:

```text
1280 x 720 windowed
1920 x 1080 windowed/fullscreen
2560 x 1440
3440 x 1440 ultrawide
3840 x 2160
manual continuous resize across representative intermediate sizes
```

Required world conditions:

- stationary player in a dense environment;
- movement in all cardinal and diagonal directions;
- multi-floor/occlusion-relevant geometry where supported;
- representative creature count;
- names/health bars/text enabled;
- lighting enabled;
- representative VFX load;
- active HUD, chat, battle list, minimap, action bars and containers;
- repeated viewport resize during movement;
- DPI/monitor scale transition where platform support permits.

The same world zoom must be used when directly comparing A with B.

## 8. Measurements

Capture at minimum:

### Player/presentation

- visible tile extent X/Y;
- effective tile pixel size;
- player-character centering stability;
- map readability;
- target/name/health-bar readability;
- amount of unused/cropped/stretched viewport space;
- subjective navigation comfort in blind comparison;
- perceived benefit of larger resolution/window;
- ultrawide behavior.

### Client performance

- CPU frame time p50/p95/p99;
- GPU frame time p50/p95/p99;
- rendered tiles per frame;
- visible creatures/effects/text labels;
- draw/instance counts;
- map extraction/culling time;
- resize/reconfigure spikes;
- client map/projection memory;
- transient allocation and cache pressure where measurable.

### Network/server relevance

For Variant A where server interest expands, measure:

- requested and accepted relevance extent;
- bytes/s and packets/s attributable to map/world updates;
- initial map/snapshot payload size;
- per-step strip/delta update size;
- server AOI/relevance CPU work;
- number of entities/tiles included in projection;
- latency impact during movement and resize-triggered extent changes;
- rate limiting/debounce behavior for repeated manual resize.

## 9. Fairness evaluation

The experiment must explicitly answer:

1. Does Variant A create a meaningful PvE navigation/avoidance advantage at 4K or ultrawide?
2. Does it create a meaningful PvP targeting/awareness advantage?
3. Can server-side gameplay visibility remain common while rendering more non-sensitive terrain, or would that produce confusing partially populated regions?
4. Is any advantage comparable to an acceptable world-zoom setting, or is it materially tied to hardware/resolution?
5. Does Variant B materially degrade the value/readability of larger screens?

No claim that either policy is fair is allowed before this evidence exists.

## 10. Decision rule

The final policy is **NOT DECIDED** by this document.

Prefer Variant A only if the physical evidence shows that:

- responsive FOV gives a clear usability/presentation benefit;
- client/server/network cost remains bounded;
- the server can safely support the relevance contract;
- gameplay fairness is acceptable under the intended product rules.

Prefer Variant B only if the evidence shows that:

- fixed gameplay FOV materially improves fairness, resource predictability or gameplay readability;
- the larger-display experience remains visually acceptable and does not waste screen space enough to harm the product.

If neither result is acceptable, a later bounded follow-up MAY test a hybrid capped-responsive policy, for example a responsive field of view up to a common server-approved maximum. That hybrid is deliberately not selected now.

## 11. Implementation sequencing

```text
UI/world viewport foundation
  -> deterministic resize + zoom test harness
  -> Variant A implementation behind non-production experiment switch
  -> Variant B implementation behind non-production experiment switch
  -> identical qualification scenes and measurements
  -> player/producer review of captured evidence
  -> explicit architecture decision
  -> remove or quarantine the losing experimental path
```

The first implementation MUST NOT silently make one experimental variant permanent merely because it is easier to code.

## 12. Acceptance evidence for the future decision

A later decision PR must include:

- exact client/server revision(s);
- exact experiment configuration;
- screenshots or recorded visual evidence at required resolutions;
- visible tile extents for A and B;
- client CPU/GPU measurements;
- network/server relevance measurements where A expands server interest;
- fairness/product review outcome;
- explicit selected policy and rejected policy;
- bounded limits/caps if the selected policy needs them;
- regression-test plan for resize, DPI, zoom and map-input mapping.

Until that evidence exists:

```text
RESPONSIVE_FOV_POLICY = UNDECIDED
FIXED_FOV_POLICY = UNDECIDED
ADDON_PLATFORM = DEFERRED_FUTURE_CONCEPT
```
