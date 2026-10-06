# ARCH-I18N-A11Y-CREATIVE-0: player text, accessibility baseline and asset provenance

- Decision id: ARCH-I18N-A11Y-CREATIVE-0. Gaps: `UX-I18N-A11Y-01`, `CREATIVE-DIRECTION-01`.
- Origin: owner ruling 2026-10-06: localization, accessibility and creative direction are decided now.
- Status: proposed decision; it has no runtime authority. The owner rulings of 2026-10-06 on Q1,
  Q2, Q3 and the field of view are recorded in §4 and the body follows them; Q5 is open with a
  PROPOSED default. The §1 rulings and
  the §3 packets are accepted on merge. Every contract amendment in §2 is exact text marked
  pending: it is applied by the named packet, because the owning file is a candidate
  (`ALPHA-CLIENT-01`) or because the change lands with the packet that builds it. Creative taste
  is the owner's (§1.14); this decision fixes only structure.
- Owning texts amended: `ALPHA-CLIENT-01` §12; content tree contract §8; ARCH-ERROR-CODES-0 §1.9
  (localization item). Model: ARCH-ERROR-CODES-0.
- Related: ARCH-LIVE-READINESS-0 §2 amendment A3 replaces the release-display item of the same
  ERR-CODES §1.9; the two replacements touch different sentences and do not conflict.
  ARCH-ALPHA-OPS-0 §4 ships assets inside the build, which is the build that §1.17's manifest checks.

## Implementation brief

1. Four kinds of player text, four rules. UI text: Fluent keys in client catalogs. Outcome text
   (errors, dispositions, fixed notices): a typed code or enum on the wire, the sentence in the
   client. Content text (item, creature, NPC, quest, achievement): the server keeps sending
   English and later adds a stable text reference, only to clients that select a new capability.
   Player text (chat, names, books): never translated.
2. The client uses Project Fluent (`fluent-bundle`, `fluent-langneg`, `unic-langid`). Catalogs:
   `apps/client/locales/<bcp47>/*.ftl`. `en` is complete and is the fallback for every message.
   Alpha ships `en` and `pl` for UI and outcome text; content stays English (Q1 b).
3. The locale is a client-only `OS_USER` setting, defaulting from the OS preferred languages. The
   server never receives a locale.
4. Plurals use Fluent selectors (CLDR) and numbers `NUMBER()`. ICU4X is added only when a shipped
   string needs locale grouping or a date. Instants on the wire stay UTC.
5. The ERR-CLIENT-2 catalogue becomes `errors.ftl` (`error-<code>`, `error-block-<n>`); the
   behaviour of ARCH-ERROR-CODES-0 §1.6 is unchanged.
6. The server stops building English sentences for an outcome when that path is touched. An
   unknown enum value shows its family's generic text.
7. Content key = typed identity `#` field, e.g. `oteryn:item.tibia.i3288#name`. A translation
   stores the English source SHA-256 and is shown only while it matches; otherwise English.
8. The content corpus stays English in place. Translations are gettext PO overlays, added later.
   No content locale is needed for alpha.
9. NPC keywords stay canonical English tokens matched on the server. The client may show
   clickable keywords that send the canonical token.
10. Text rendering must cover Polish. Font: Noto Sans (OFL-1.1), a LICENSED asset. Shaping stays
    gated by UI programme P4-T (CANDIDATE: cosmic-text).
11. Alpha accessibility: UI scale; panel text size; full key remapping on `BindingMap`; no
    gameplay information by colour alone; reduced-motion and no-flash switches; a visual
    equivalent for each gameplay sound once audio exists. Settings never reach the server.
12. Out of scope for alpha: screen reader, controller, account-synced settings.
13. Creative direction is owner-owned at `docs/architecture/OTERYN_CREATIVE_DIRECTION.md`
    (pillars, tone, art, audio, UI, naming, per profile). Architecture fixes only its structure.
14. Every shipped asset and text corpus has a provenance record in `content/assets/catalog/`:
    ORIGINAL, LICENSED, OWNER_CLEARED_THIRD_PARTY or REFERENCE_ONLY. Every AI-produced asset has
    one too.
15. A shipped-build manifest and a CI validator fail any shipped input that is REFERENCE_ONLY or
    has no record. Only what a release build ships is scanned: the release-compiled embeds of the
    two production roots, the content trees the production boot path loads and the packaged
    client assets. Test, dev and fixture files are never scanned (§1.17).
16. Two UI layouts in one client, switched by the client setting `ui_layout`: A classic-faithful
    (built first), B modern Oteryn. Both are data files over theme tokens; final art is
    AI-produced. The switch is presentation only and never touches the field of view.
17. The field-of-view experiment is a second, independent client setting `fov_arm`: FOV-fixed
    standard, FOV-fixed large, FOV-responsive. Any arm runs under either layout. The client only
    requests; the server grants a per-session extent within the channel cap and the viewport
    budget, on a non-production test channel only (§1.21, packet MAP-VIEW-EXTENT-7).
18. Packets: I18N-CATALOG-0 (after ERR-CLIENT-2), then A11Y-BASELINE-1. ASSET-PROVENANCE-2 runs
    in parallel. CREATIVE-DOC-3 needs the owner. I18N-TEXT-RENDER-4 follows P4-T. UI-LAYOUT-6
    follows packet 0. MAP-VIEW-EXTENT-7 follows MAP-CUTOVER-1 and needs protocol review.
    I18N-CONTENT-KEYS-5 is deferred until a content locale is chosen.
19. Owner rulings 2026-10-06 (§4): Q1 b; Q2 a (OWNER_CLEARED_THIRD_PARTY, the owner's risk;
    distribution still waits for the provenance records and validator); Q3 both layouts, A
    first; both field-of-view arms built and switchable in the client for the A/B test, final
    policy after the evidence. Q5 (the larger fixed size) is open with a PROPOSED default.

## 0. Facts

- F1 PROVEN. Client UI text is hardcoded English: `apps/client/src/spell.rs` lines 29–38
  (`NotEnoughMana => "Not enough mana"`), `apps/client/src/cyclopedia.rs` lines 29 and 35–56. No
  catalog or locale code exists in `apps/` or `crates/`.
- F2 PROVEN. ARCH-ERROR-CODES-0 §1.6: a client catalogue keyed by code, block generic text for an
  unknown code, and no server English for coded errors. §2.3 ERR-CLIENT-2 adds `error_text` in
  `apps/client/src/`. §1.9 leaves localization undecided.
- F3 PROVEN. The server still builds English: `apps/game-server/src/spell/native_house_movement.rs`
  lines 222 and 566; `apps/game-server/src/spell/native_companions.rs` lines 793–801
  (`failure_messages`, `creature_name: "Knight familiar"`).
- F4 PROVEN. The wire sends English, sometimes with a key (`docs/contracts/protocol-oteryn/v1/`):
  `account_achievements_v1.proto` `key` + `name` (64 B) + `description` (256 B);
  `achievement_notices_v1.proto` `key` + `name`; `quest_log_v1.proto` `name` (128 B) + rendered
  journal `text` (1,024 B, from `QuestLogCatalogue`, F21); `chat_v1.proto` `ChatLineV1`
  `speaker_name` + player `text`.
- F5 PROVEN. `item_view_v1.proto`, `world_spatial_v1.proto` and `charm_bestiary_v1.proto` carry no
  name or description. How item and creature names reach the client is UNKNOWN (U1).
- F6 PROVEN. Achievement rows are ordered by grade, then name by Unicode code point, then key
  (`account_achievements_v1.proto` line 47). The order depends on the English name.
- F7 PROVEN. NPC replies are a fixed template filled with data; the server ASCII-case-folds and
  matches whole English keywords (`reviews/OTERYN_GAME_NPC0_NPC_RUNTIME_SERVICE_DECISION_2026-09-30.md`
  §3.3–§4). Replies output strings and template ids (`reviews/OTERYN_GAME_ARCH_NPC_PACKETS_2026-10-05.md` §1.2).
- F8 PROVEN. Content text is stored three ways: item JSON inlines `display_name`
  (`OTERYN_ITEM_AUTHORING_FORMAL_SCHEMA_V1.md` §2); NPC bundles hold references
  `{sha256, length, placeholders, links}` with full text in WorldProject/v2, and D9 admits Tibia
  NPC text 1:1 (`OTERYN_NPC_AUTHORING_SCHEMA_V1.md` §2, D9); quest samples keep references only
  and the validator rejects committed text (`OTERYN_QUEST_AUTHORING_FORMAT_V1.md` lines 58–60).
- F9 PROVEN. The client uses `wgpu` and `winit`; no font, shaping or audio crate is in the
  workspace. The shaping library is EVIDENCE_REQUIRED_BEFORE_SLICE (P4-T,
  `OTERYN_NATIVE_CLIENT_UI_IMPLEMENTATION_PROGRAMME_V1.md`). The text pipeline must handle
  Unicode and Polish (`OTERYN_NATIVE_CLIENT_UI_ARCHITECTURE_BASELINE_2026-09-10.md` §16).
- F10 PROVEN. Same baseline: §7 `physical_px = logical_ui_unit * platform_dpi_scale *
  user_ui_scale`, world zoom independent; §17 "A theme must not change gameplay-significant
  information availability"; §18 semantic `UiFontKey`.
- F11 PROVEN. `crates/input-actions/src/physical.rs` `KeyCode(u16)` is "independent of localized
  labels"; `BindingMap` refuses `ConflictingBinding` and `ReservedBinding`. No client settings file
  exists.
- F12 PROVEN. `ALPHA-CLIENT-01` (CANDIDATE) §12: "Exact keybind/controller/accessibility policy is
  deferred." Audio needs master, effect and music controls. `ACCOUNT` sync needs an accepted
  Platform contract; language and accessibility "MAY" live in `OS_USER`.
- F13 PROVEN. `UX-I18N-A11Y-01` is REQUIRED_FOR_ALPHA and must preserve: "translated strings never
  become protocol or domain identifiers; accessibility cannot depend on server-side trust or alter
  authoritative rules" (`GAMEPLAY_AND_PRODUCT_ARCHITECTURE_HORIZON.md` line 532).
- F14 PROVEN. `LICENSE-ASSETS.md`: fonts, audio, names, descriptions, quest and NPC text are all
  rights reserved. Third-party text "may be recorded or reproduced as reference data";
  redistribution of original third-party files and media is "assessed separately".
- F15 PROVEN. The owner confirmed redistribution rights for the 15.30 client files in
  `content/assets/files/` (6,248 files; no fonts, audio or UI art):
  `OTERYN_CLIENT_ASSET_VERSION_OWNER_DECISION_2026-09-27.md` D154 / Q12a (#162 2026-09-29).
- F16 CONFLICT, textual. That file's "Non-claims" still say no proprietary asset is committed. The
  2026-09-29 supersession note governs; the text needs a pointer (§1.18).
- F17 DERIVED. ADR-0010 §9 (parity does not authorize copying text or art) and ADR-0005 line 379
  (no CipSoft asset without "confirmed rights and provenance"): authority comes from an owner
  record (F15), not from parity. No record yet covers shipping text (U2).
- F18 PROVEN. `content/assets/catalog/index.json` is the "Immutable asset identity/digest/licensing
  catalogue", `READY_UNPOPULATED`. Content tree contract §8: `source id != canonical Oteryn gameplay id`.
  No packaging or release workflow exists in `.github/workflows/`.
- F19 PROVEN. FND-02 §7: additive protobuf fields; unknown fields never activate behaviour. §9:
  capabilities only for features an older peer can safely skip.
- F20 PROVEN. The content-text decoders are strict and reject an unknown field as malformed:
  `crates/protocol-oteryn/src/account_achievements.rs` line 316 (row) and
  `crates/protocol-oteryn/src/quest_log.rs` line 276 (quest line; negative test
  `quest_log_tests.rs` line 265). A field added to those messages breaks a deployed client.
  Negotiation exists:
  the client lists `supported_capability_id` in `ClientBootstrap` and `ClientResume`
  (`docs/contracts/protocol-oteryn/v1/foundation.proto` lines 44 and 96), the server answers
  `selected_capability_id` (lines 79 and 109), and `SelectedCapabilities::select`
  (`apps/game-server/src/gameplay_transport/capabilities.rs` line 178) keeps only offered ids.
- F21 PROVEN. The server serves content text from these inputs: achievement name and description
  from `content/achievements/*.json`, embedded with `include_str!`
  (`apps/game-server/src/achievement_catalogue.rs` lines 35–36); NPC dialogue from the
  WorldProject families `content/npcs/` and `content/dialogues/`, loaded at run time by
  `load_data_only_npc_catalogue` (`apps/game-server/src/content/npc_catalogue.rs` line 48);
  quest names and journal text from `QuestLogCatalogue` (`apps/game-server/src/quest/log.rs`
  line 138), which no production source feeds yet (`gameplay_transport/quest_log.rs` lines
  19–20); item `display_name` inline in `content/items/` (F8).
- F22 PROVEN. `OTERYN_NATIVE_CLIENT_VIEWPORT_AB_EXPERIMENT_PLAN_2026-09-10.md` §5 Variant A is a
  responsive field of view, §6 Variant B a fixed one; §9 asks whether A gives an advantage, §11
  runs both behind a non-production switch. ARCH-MAP-VIEWPORT-BUDGET-V1
  (`reviews/OTERYN_GAME_ARCH_MAP_VIEWPORT_BUDGET_2026-10-06.md`) fixes an 18x14 viewport and
  `MAP01-VIEWPORT-SNAPSHOT-US` at 2,000 us p99.
- F23 PROVEN. The server view is fixed at 18x14 today. `apps/game-server/src/movement/interest.rs`
  line 77: `VisibilitySettings` is an immutable per-Channel size, `REFERENCE` 18x14, and `new`
  admits 15..=36 by 11..=28 (lines 24–27). The entity query of capability 6 uses `REFERENCE`
  (`gameplay_transport/world_spatial.rs` line 175). The domain 17 window is wire-fixed:
  `crates/protocol-oteryn/src/world_map.rs` lines 66–69 (`VIEW_WIDTH` 18, `VIEW_HEIGHT` 14,
  left 8, top 6), `MAPW-RL-02` = 2,016 tiles (line 40), and `world_map_v1.proto` line 7 makes a
  tile outside the window fail closed. Attack and creature-targeting legality also use
  `REFERENCE.can_see` (`gameplay_transport/attack.rs` line 224, `ai/targeting.rs` line 56).
  Capability 18 is not offered (`offer_gate` in `PROTOCOL_OTERYN_V1_REGISTRY.json`).
- F24 PROVEN. Not every embed is shipped. `crates/protocol-oteryn/src/damage_element.rs` lines
  49–54 and `apps/game-server/src/durability/schema.rs` lines 136–139 embed a `.proto`, a SQL
  migration and Rust source inside `#[cfg(test)]` modules; many server modules embed fixtures in
  test modules. `workspace-boundaries.toml` line 5 names the production roots: `oteryn-client`
  and `oteryn-game-server`. Release-compiled embeds of the server today include
  `content/achievements/`, `content/quests/missions/`, `content/combat/`, `content/movement/`,
  `rulesets/progression/` and even a file under `tools/content-schema/spell-authoring/samples/`
  (`spell/native.rs` lines 18–20), so directory names do not tell shipped from test inputs; the
  client embeds `content/movement/step_speed_v1.json` (`apps/client/src/input.rs` line 62). The
  NPC loader takes a pinned tree hash (`load_data_only_npc_catalogue`,
  `content/npc_catalogue.rs` line 48).

## 1. Rulings

1.1 **Text classes.** Each player-visible string is exactly one class: (a) UI, owned by the client;
   (b) outcome, a closed set chosen by the server; (c) content, owned by a content record;
   (d) player text or a player-chosen name. Class (d) is never keyed, translated or reordered.
   Translated text is never an identifier, command argument, server sort key or authority input
   (F13).

1.2 **Catalogs.** Classes (a) and (b) use Project Fluent (`fluent-bundle`, `fluent-langneg`,
   `unic-langid`). Catalogs: `apps/client/locales/<bcp47>/<area>.ftl` (new), embedded at build
   time. Ids are kebab-case and stable; a new meaning or argument set takes a new id, and an old
   id is never reused. `en` defines every id. CI fails on a missing `en` id, an unused id or an
   argument mismatch between locales.

1.3 **Locale.** An `OS_USER` setting. Its default is the Windows preferred UI languages, negotiated
   with `fluent-langneg` against the catalogs present. Fallback per message: selected locale, then
   `en`. A message missing everywhere shows its id in test builds and the generic `en` text in
   release builds. The server receives no locale; no wire, admission or Platform field is added.
   Alpha ships `en` and `pl` catalogs for classes (a) and (b) (Q1 b). A `pl` id missing at a
   release build fails that build; in development it falls back to `en` per message.

1.4 **Formatting.** Plurals use Fluent selectors (CLDR); plurals built in Rust are rejected.
   Numbers use `NUMBER()`. ICU4X (`icu_decimal`, `icu_datetime`) is CANDIDATE, added when a
   shipped string needs locale grouping or a date (trigger: first `pl` review finding). The wire
   carries UTC instants; the client shows local time.

1.5 **Outcome text (b)** extends ARCH-ERROR-CODES-0 §1.6. The server sends a typed code or enum
   and typed arguments, never a sentence. A server-built sentence (F3) is converted when its path
   is touched: a new value plus `.ftl` entries. An unknown value shows its family's generic
   message, so an older client stays readable. ERR-CLIENT-2's texts become `error-<code>` and
   `error-block-<n>` in `errors.ftl`; its tests are unchanged.

1.6 **Content key (c)** = `<typed content identity>#<field>`, e.g. `oteryn:item.tibia.i3288#name`.
   It is derived, never stored, so there is no second registry. Placeholders keep their
   content-format names (NPC schema §6); a translation must keep the same placeholder set. The
   English source stays where it is (F8) and is the `en` text.

1.7 **Content on the wire.** Unchanged until a content locale is accepted; Q1 b selects none for
   alpha. Packet 5 then adds an optional `TextRef {key, source_sha256}` beside each content text
   field it touches; the English field stays filled. The current decoders reject an unknown field
   (F20), so `TextRef` is capability-gated (FND-02 §9). Packet 5 registers a new capability
   (working name `CONTENT_TEXT_REF_V1`, no command type) in `PROTOCOL_OTERYN_V1_REGISTRY.json`.
   A client that decodes `TextRef` lists it in `supported_capability_id` at bootstrap and resume;
   the server selects it with `SelectedCapabilities::select` and keeps the selection for the
   GameSession. The server writes `TextRef` only into payloads for a session whose selection
   contains the capability; every other session gets the legacy encoding, byte for byte. A newer
   client on an older server, or one not selected, shows English. The client never decides
   whether it receives `TextRef` beyond advertising support. The tests are packet 5's. The `key`
   byte limit goes to `RESOURCE_LIMITS_REGISTRY.json` in that packet: CANDIDATE, set from the
   longest identity measured.

1.8 **Stale translations.** A translation records its English source SHA-256 and is shown only
   while that equals `source_sha256`; otherwise English. NPC references already carry the hash
   (F8); items and quests compute it at content build.

1.9 **Corpus translation is off the alpha path.** Overlays are gettext PO at
   `content/translations/<bcp47>/<family>.po` (new): `msgctxt` = key, `msgid` = English, `#.` =
   hash. PO is chosen for mature translator tooling (Weblate, Poedit; `polib` in tools). The
   content build compiles a client pack. Nothing is built before a content locale is chosen; Q1 b
   chose none for alpha.

1.10 **NPC keywords** stay canonical English tokens matched on the server (F7). A client may show
   a reply's keywords as clickable items with localized labels that send the canonical token.
   Server-side localized aliases are out of scope.

1.11 **Ordering.** The server orders paged lists by canonical text or key (F6). The client may
   re-sort a fully loaded list with locale collation; paging never uses translated text.

1.12 **Rendering** covers Latin Extended-A (Polish), has a fallback font and follows the UI
   baseline §7 scaling (`OTERYN_NATIVE_CLIENT_UI_ARCHITECTURE_BASELINE_2026-09-10.md`). The UI
   font is Noto Sans (SIL OFL 1.1), recorded as LICENSED, unless a layout's theme tokens (§1.20)
   or the creative document pick another OFL or owned font. The shaping library stays with P4-T,
   CANDIDATE cosmic-text (upstream rustybuzz and swash). Evidence: a `pl` sample, a fallback
   glyph, and the frame cost at 1080p and 4K. No fork.

1.13 **Alpha accessibility baseline.** All settings are `OS_USER`, client-only and
   presentation-only (F13).
   - (a) UI scale: `user_ui_scale` in steps. Range and default are CANDIDATE, set by a legibility
     check at 1080p/100% and 4K/150% DPI.
   - (b) Text size follows UI scale. Chat, log and message panels add a three-step text size.
   - (c) Remapping of every semantic action via `BindingMap`. Conflicts are refused, reserved keys
     are listed in the binding schema, and reset is available. Labels follow the current keyboard
     layout for the physical `KeyCode`.
   - (d) No gameplay-significant state by colour alone. Client-drawn UI pairs colour with a shape,
     icon or text. Where Reference art differs only by colour (skulls, party shields, message
     colours), the client adds a tooltip, inspect line, message prefix or tab. Theme colours pass
     a deuteranopia, protanopia and tritanopia simulation check.
   - (e) A reduced-motion switch (client camera shake, UI animation) and a no-flash switch. Client
     effects never flash more than 3 times per second (WCAG 2.3.1). Reference sprites unchanged.
   - (f) No audio exists (F9). When it lands, each gameplay cue gets a visual equivalent, spoken
     lines get captions, and the `ALPHA-CLIENT-01` controls apply.
   - (g) Out of scope for alpha: screen reader or TTS, controller, one-handed presets, dyslexia
     font, `ACCOUNT`-synced settings (a Platform proposal routed by the control plane).

1.14 **Creative direction document.** `docs/architecture/OTERYN_CREATIVE_DIRECTION.md` (new) is
   owned by the owner; agents propose, only an owner ruling accepts. Sections: 1 pillars; 2 tone
   and writing voice; 3 art per profile; 4 audio; 5 UI style and theme tokens; 6 naming and
   branding (`TRADEMARKS.md` applies); 7 accessibility commitments (§1.13); 8 profile scope
   (ADR-0010). An empty section means "not decided".

1.15 **Profiles.** Reference visuals are the owner-cleared 15.30 files (F15); the creative
   document does not restyle them. It governs Oteryn-original UI chrome, fonts, audio, Evolved
   art and new text, including the look of both layouts (§1.20). Taste beyond the §4 rulings
   stays open and does not block alpha.

1.16 **Provenance record.** Every shipped asset file and text corpus has one record in
   `content/assets/catalog/`: `asset_id` (typed, never a source id), `path`, `sha256`, `class`,
   `source`, `license_id` (SPDX where one exists), `attribution`, and `owner_record` (a locator,
   required for OWNER_CLEARED_THIRD_PARTY). Classes: ORIGINAL, LICENSED,
   OWNER_CLEARED_THIRD_PARTY, REFERENCE_ONLY (may be stored, never shipped). The 15.30 set is one
   OWNER_CLEARED_THIRD_PARTY package with a per-file manifest citing D154 / Q12a. Text corpora get
   one record each: item text, NPC dialogue (D9), quest journals, achievements; each is
   OWNER_CLEARED_THIRD_PARTY citing the Q2 ruling (§4). An AI-produced asset (§1.20) is ORIGINAL;
   its `source` names the producing task, the kind of AI tool and the reference inputs by
   `asset_id`, and review confirms it does not reproduce a rights-reserved file.

1.17 **Shipped-build manifest.** A generator lists the files and corpora a client or server build
   packages. A validator fails an entry that has no record, is REFERENCE_ONLY, differs from its
   record hash, or is LICENSED without attribution in the shipped notices file. The manifest
   enumerates shipped inputs, not records, so an unrecorded shipped input fails. Only shipped
   production inputs are in it. Until packaging exists (F18) the interim set S is the union of:
   - E, release embeds: every non-`.rs` path in the dep-info that the release compile writes for
     each bin target of the production roots (`workspace-boundaries.toml` `production_roots`:
     `oteryn-client`, `oteryn-game-server`): `cargo build --release --locked`, default features,
     `target/release/<bin>.d`; the packet may read the per-unit `.d` of
     `cargo check --release --locked` instead if a test shows the same set. The compiler decides
     membership, not a directory name or a regex (F24): `#[cfg(test)]` modules (and the
     `*_tests.rs` files they declare), `tests/`, `benches/`, `examples/`, dev-dependencies and
     `tools/dev-client` are never compiled into these targets, so their embeds are never in E.
   - L, run-time loads: the content trees the production boot path loads, each named once in
     `tools/asset-provenance/shipped_inputs.json` with the loader that reads it and the tree hash
     it pins (the WorldProject families, `content/npcs/`, `content/dialogues/`; F21). A test
     checks that every production loader's root is listed there. Test WorldProjects and fixture
     trees that no production loader reads are not in L.
   - P, packaged client assets: the asset roots the release client loads at run time
     (`content/assets/files/`, and `apps/client/locales/`, `themes/` and `layouts/` once they
     exist), listed in the same file.
   Each member of S is either an asset or text input, which maps to exactly one record or corpus
   record (a corpus record covers a family root and the hash of its tree), or a reviewed
   `NON_ASSET_INPUT` entry in `shipped_inputs.json` naming a path or root and the reason. That
   class is for files with no media and no player-visible text: numeric rule tables, protocol and
   resource registries, SQL migrations. It needs no provenance record and adds none to the
   catalogue. A member that is neither fails, so a new release embed always needs a decision; a
   `NON_ASSET_INPUT` entry inside a text corpus root or an asset root is refused. Once packaging
   exists, P becomes the release manifest of the packaged set and E stays. The check runs in the
   content path class of `game-gate`. Tests: a release-path embed of an unrecorded corpus file
   fails; an unclassified release embed fails; a `NON_ASSET_INPUT` entry under a corpus root is
   refused; an unrecorded file embedded only under `#[cfg(test)]` or by a `tests/` target is not
   in S and passes; on the current tree the `#[cfg(test)]` embeds of F24
   (`damage_element_v1.proto`, the durability migration and Rust source) are not in S; a new
   production content family without a corpus record fails; a REFERENCE_ONLY corpus in L fails; a
   fixture WorldProject no production loader reads is ignored. This decision grants and narrows
   no rights; rights come only from owner records (§4 Q2 ruling).

1.18 **Stale text.** ASSET-PROVENANCE-2 adds one line under "Non-claims" in the asset-version
   decision, pointing to the 2026-09-29 supersession (F16).

1.19 **Before freeze.** (1) The amendments below are exact text. (2) No concurrent transition is
   added. (3) Restart needs only the `OS_USER` settings file. (4) References are typed
   (`<identity>#<field>`, `asset_id`). (5) Older clients get generic fallback text and never
   receive `TextRef`, which is sent only under the capability of §1.7. (6) Content text and its
   `TextRef` leave in one message from one path. (7) The layout setting changes presentation only
   (§1.20). The field-of-view arm is a separate setting; the client requests, the server grants
   within the channel cap, and the two settings never read each other (§1.21).

1.20 **UI layouts (Q3 ruling).** One client carries two layouts, switched by an `OS_USER` setting
   `ui_layout`: A classic-faithful and B modern Oteryn. A is built first and is the default; B
   follows. Two structures make this one client, not a fork:
   - Theme tokens. Colours, fonts (`UiFontKey`, UI baseline §18), spacing and sprite sets are
     named tokens in `apps/client/themes/<theme_id>.json` (new); widgets reference tokens, never
     literal values or sprite paths.
   - Layout as data. Panel set, placement, docking, sizes and the theme in use are data in
     `apps/client/layouts/<layout_id>.json` (new). The client code has no branch per layout; a
     new layout is a new data file.
   The switch never changes gameplay information (UI baseline §17, F13): both layout files expose
   the same set of gameplay information, which a test checks. A layout file has no field-of-view
   field and `ui_layout` is never sent to the server, so switching layout leaves the granted
   extent unchanged under every `fov_arm` (§1.21). Final art for both layouts is produced with AI
   tools, by agents or with an external AI tool; every such asset gets a §1.16 record before it
   ships. The creative document §5 records the owner's taste for each layout.

1.21 **Field of view (owner ruling 2026-10-06, §4 item 4).** Both arms of
   `OTERYN_NATIVE_CLIENT_VIEWPORT_AB_EXPERIMENT_PLAN_2026-09-10.md` (F22) are built and can be
   switched in the client for the A/B test. To avoid a clash with layouts A and B, this decision
   calls the plan's Variant B **FOV-fixed** and its Variant A **FOV-responsive**. The client
   setting `fov_arm` (`OS_USER`) has three values:
   - `FIXED_STANDARD`: today's 18x14 extent (F23), the default;
   - `FIXED_LARGE`: the larger fixed extent, which is the channel cap (Q5, PROPOSED: the largest
     candidate that measures within budget), to judge whether a larger map gives too much
     advantage;
   - `RESPONSIVE`: an extent in whole tiles derived from the client's world viewport and world
     zoom (plan §5).
   Rules:
   - (a) Independence. `fov_arm` and `ui_layout` (§1.20) are separate settings with separate
     defaults. Neither reads the other, no packet or wire field carries the layout, and every
     pair of layout and arm is a valid configuration. A comparison between arms holds the layout
     and the world zoom equal (plan §7). The tester or harness sets the arm for each run and the
     evidence records it; the arm is never derived from the layout.
   - (b) The server decides. The client sends only a request: the arm and, for `RESPONSIVE`, its
     tile extent. The server grants a per-session extent. `FIXED_STANDARD` and an unknown arm get
     18x14; `FIXED_LARGE` gets the channel cap; `RESPONSIVE` gets the request clamped on each
     axis between 18x14 and the channel cap. No grant exceeds the channel cap; a larger request
     is clamped, not refused. A session without the extent capability gets the legacy 18x14 view
     whatever its arm. The client draws only what it is sent and never invents tiles (plan §5).
   - (c) Fairness. The cap is a per-Channel setting, the same for every session on that channel.
     A production World never offers the extent capability, so every production session keeps
     the 18x14 legacy view until the final policy is decided. Only a non-production test channel
     sets a cap above 18x14, and there the differences between arms are what the experiment
     measures (plan §9).
   - (d) Delivery, not legality. The grant sizes only that session's domain 17 window and its
     capability 6 entity query. Attack and targeting legality, line of sight and every other
     `can_see` use stay on `VisibilitySettings::REFERENCE` (F23, plan §4).
   - (e) Budget. A cap above 18x14 is admitted only when the domain 17 snapshot at that extent
     measures within `MAP01-VIEWPORT-SNAPSHOT-US` (2,000 us p99, ARCH-MAP-VIEWPORT-BUDGET-V1) by
     that decision's §1.1 method on the reference node class. If it does not fit, the cap stays
     18x14 and the packet returns a budget question (an encode optimisation or a separate
     test-channel row); the wire is never changed only to pass. Each arm also reports the
     network and server measurements of plan §8.
   - (f) The final policy stays undecided (`RESPONSIVE_FOV_POLICY` and `FIXED_FOV_POLICY` are
     UNDECIDED) until the plan's §12 evidence exists. Packet MAP-VIEW-EXTENT-7 builds the server
     side and the client request; UI-LAYOUT-6 has no field-of-view code.

## 2. Contract amendments

1. `docs/architecture/ALPHA-CLIENT-01_NATIVE_CLIENT_ARCHITECTURE_CONTRACT_CANDIDATE.md` §12.
   Replace "Exact keybind/controller/accessibility policy is deferred." with:

   > Keybind remapping and the alpha accessibility baseline are fixed by ARCH-I18N-A11Y-CREATIVE-0
   > §1.13: `OS_USER`-scoped, presentation-only settings for UI scale, panel text size, full
   > semantic-action remapping with conflict refusal, no colour-only gameplay information, and
   > reduced-motion and no-flash switches. Controller, screen-reader and account-synced settings
   > are deferred.

2. `docs/architecture/OTERYN_FULL_GAME_CONTENT_AND_RULESET_TREE_V1.md` §8. Append:

   > Every shipped asset file or text corpus has one provenance record in `content/assets/catalog/`
   > with class `ORIGINAL`, `LICENSED`, `OWNER_CLEARED_THIRD_PARTY` (with an owner-record locator)
   > or `REFERENCE_ONLY`. A `REFERENCE_ONLY` item is never packaged; a shipped-build validator
   > enforces this (ARCH-I18N-A11Y-CREATIVE-0 §1.16–§1.17). `content/translations/<bcp47>/` holds
   > translation overlays keyed by `<identity>#<field>`; a translation never becomes an identity.

3. `docs/architecture/reviews/OTERYN_GAME_ARCH_ERROR_CODES_2026-10-05.md` §1.9. Replace
   "Localization of client text beyond the existing language." with:

   > Localization is decided by ARCH-I18N-A11Y-CREATIVE-0; the catalogue of §1.6 is its `errors.ftl`.

   The release-display item of the same section is replaced by ARCH-LIVE-READINESS-0 §2
   amendment A3; the two edits replace different sentences and may land in either order.

## 3. Packets

- **I18N-CATALOG-0** (Fluent in the client; after ERR-CLIENT-2 merges). Owned:
  `apps/client/Cargo.toml`, `apps/client/src/` (new `text` module; call sites in `spell.rs`,
  `cyclopedia.rs`, `error_text`), `apps/client/locales/en/` and `apps/client/locales/pl/` (new),
  `Cargo.lock`. Builds §1.2–§1.5. Tests: missing, unused or mismatched id fails; a missing `pl` id
  fails a release build; unknown enum shows generic text; `pl-PL` falls back per message in
  development; error fallback tests unchanged.
- **A11Y-BASELINE-1** (after packet 0). Owned: `apps/client/src/`, `crates/input-actions/src/`.
  Builds §1.13 (a)–(e) and the colour-only audit list. Tests: remap round-trips through the
  settings file; conflict and reserved key refused; the UI baseline §7 formula; flash rate ≤ 3/s; legibility
  check fixes the CANDIDATE range.
- **ASSET-PROVENANCE-2** (parallel). Owned: `content/assets/catalog/`, `tools/asset-provenance/`
  (new, with `shipped_inputs.json`), CI path routing, one line in
  `OTERYN_CLIENT_ASSET_VERSION_OWNER_DECISION_2026-09-27.md`. Builds §1.16–§1.18, including the
  four text corpus records of the Q2 ruling and the interim shipped-input set of §1.17 (release
  embeds, production loads, packaged client assets). Tests: REFERENCE_ONLY fixture fails;
  missing record fails; hash drift fails; LICENSED without attribution fails; the §1.17 set tests
  (an unrecorded release embed fails, a test-only embed is not scanned, an unclassified embed
  fails, `NON_ASSET_INPUT` under a corpus root is refused, an unrecorded production content
  family fails); an AI-produced asset without a record fails; the 15.30 package passes.
- **CREATIVE-DOC-3** (owner). Owned: `docs/architecture/OTERYN_CREATIVE_DIRECTION.md` (new) with
  the §1.14 sections; the owner fills or accepts it, and the Q3 ruling and each layout's taste go
  in §5.
- **I18N-TEXT-RENDER-4** (after P4-T evidence and packet 2). Owned: `apps/client/`,
  `content/assets/` (font and record), `Cargo.lock`. Tests: `pl` golden render, fallback glyph,
  frame-cost measurement.
- **I18N-CONTENT-KEYS-5** (deferred until a content locale is chosen; Q1 b chose none for alpha;
  protocol review). Owned: `content/translations/` (new), `tools/i18n/` (new: PO extract,
  validate, pack), touched `.proto` files, `PROTOCOL_OTERYN_V1_REGISTRY.json` (capability and
  limits), the touched codecs in `crates/protocol-oteryn/src/`, their server encoders in
  `apps/game-server/src/` and the client decoder. Builds §1.6–§1.9. Tests: a session without the
  capability receives payloads byte-equal to the legacy encoding, which today's strict decoder
  accepts; a selected session receives `TextRef` and the new decoder accepts it; a client
  advertising the capability to a server that does not offer it is not selected and shows English;
  selection survives resume; stale hash shows English; placeholder mismatch fails; ordering
  unchanged.
- **UI-LAYOUT-6** (after packet 0; layout A first, then B). Owned: `apps/client/src/`,
  `apps/client/themes/` (new), `apps/client/layouts/` (new), AI-produced art and its records in
  `content/assets/`. Builds §1.20 only; it has no field-of-view code. Tests: both layout files
  load and validate; widgets reference only tokens; both layouts expose the same gameplay
  information set; the layout schema has no field-of-view field; switching layout sends nothing
  to the server.
- **MAP-VIEW-EXTENT-7** (§1.21; after MAP-CUTOVER-1 offers capability 18 on a testing World;
  independent of packet 6; protocol review; numbers leased by the control plane). Owned:
  `docs/contracts/protocol-oteryn/v1/world_map_extent_v1.proto` (new),
  `PROTOCOL_OTERYN_V1_REGISTRY.json` (capability, command type, domain 17 snapshot and delta
  type 2), `RESOURCE_LIMITS_REGISTRY.json`, `crates/protocol-oteryn/src/` (a new extent codec
  beside `world_map.rs`), `apps/game-server/src/map/view.rs`,
  `apps/game-server/src/gameplay_transport/` (`world_map.rs`, `world_spatial.rs`, capability
  selection, the command), the channel
  configuration that holds the cap, `apps/client/src/` (the `fov_arm` setting, the request, the
  decoder), their tests and one evidence file under `docs/agents/evidence/`. Builds:
  - a capability (working name `WORLD_MAP_VIEW_EXTENT_V1`, requires 18), offered only on testing
    and preproduction Worlds, as 18 is;
  - a command `ViewExtentRequestV1 {arm, width_tiles, height_tiles}`;
  - under that capability, domain 17 snapshot and delta type 2, whose header carries the granted
    `width` and `height`; the window runs from `(x - (w - 1) / 2, y - (h - 1) / 2)`, which is
    `VisibilitySettings`' west and north and today's 8 and 6 at 18x14; a tile outside the granted
    window fails closed. A session without the capability gets type 1 byte for byte (F23);
  - the §1.21 (b) grant, kept in the GameSession's map state and across resume; a changed grant
    sends a full snapshot; requests are coalesced to at most one applied change per CANDIDATE
    1,000 ms per session (plan §8 debounce);
  - the session's capability 6 entity query at `VisibilitySettings::new(w, h)` (15..=36 by
    11..=28, F23); legality unchanged (§1.21 d);
  - CANDIDATE limit rows from the cap: snapshot tiles `w * h * 8`, delta entries
    `(w + h - 1) * 8`, handles by measurement under the existing cut rule, the payload bounds
    that follow, and the request rate.
  Tests: a session without the capability receives byte-equal type 1 payloads; a request above
  the cap is granted the cap; an unknown arm and a request below 18x14 are granted 18x14; a
  production World does not offer the capability; switching `ui_layout` sends no request and
  leaves the grant unchanged under each arm; every pair of layout and arm configures; attack and
  targeting legality are unchanged at every grant; the new decoder fails a tile outside the
  granted window; the grant survives resume; a request flood is coalesced. Measurement: snapshot
  p99 at each proposed cap by the budget's §1.1 method (release, reference node class) and the
  plan §8 network numbers for each arm, in the evidence file; a cap that misses 2,000 us is not
  admitted (§1.21 e).

Order: 0, then 1, 4 and 6 (A, then B); 2 in parallel; 3 waits for the owner; 7 after
MAP-CUTOVER-1, in parallel with 6; 5 last.

## 4. Owner questions

1. **Q1. Which locales ship at alpha?** Polish is the main audience; content is English Tibia text.
   a) `en` only, `pl` pipeline ready; b) `en` + `pl` for UI and outcome text, content in English;
   c) `en` + `pl` including content. **Recommendation: b** (one `.ftl` set; Reference content
   stays English).
   **Owner ruling 2026-10-06: b.** Alpha ships `en` and `pl` UI and outcome catalogs (§1.3);
   content stays English and no content locale is selected, so §1.7 and packet 5 stay deferred.
2. **Q2. May the builds you distribute serve Tibia-sourced text?** NPC (D9), quest, item and
   achievement text is admitted as reference data; shipping it is "assessed separately" (F14,
   F17). a) yes, extend the 2026-09-29 clearance to these corpora as OWNER_CLEARED_THIRD_PARTY;
   b) closed alpha only, re-ruled before any public release; c) no, REFERENCE_ONLY, and alpha
   needs Oteryn-authored text. **Recommendation: b.**
   **Owner ruling 2026-10-06: a.** The 2026-09-29 clearance is extended to the NPC dialogue (D9),
   quest, item and achievement text corpora as OWNER_CLEARED_THIRD_PARTY. This was not the
   architect's recommendation (b); it is the owner's ruling and the owner's risk. It permits:
   recording those four corpora as OWNER_CLEARED_THIRD_PARTY with this ruling as `owner_record`,
   and then serving them from client and server builds the owner distributes, with no closed-alpha
   limit. It does not by itself let anything ship. Distribution stays gated until
   ASSET-PROVENANCE-2 lands: one record per corpus with its family root, tree hash and this
   locator (the quest corpus also waits for U3), and the §1.17 validator passing on the build.
   Until then the corpora count as unrecorded, and no build is distributed on the strength of
   this ruling alone. The ruling covers only those four corpora as admitted today;
   other third-party text (books, documents, any later import), fonts, audio and UI art need their
   own owner record, and `TRADEMARKS.md` still governs names and branding.
3. **Q3. What look does the Oteryn-drawn UI have at alpha?** Reference sprites are fixed; chrome,
   panels and font need theme tokens. a) classic-faithful layout, drawn originally; b) modern
   Oteryn style; c) neutral placeholder until the creative document exists.
   **Recommendation: a** for the Reference profile.
   **Owner ruling 2026-10-06: both a and b, switchable in the client settings.** Layout A is
   classic-faithful, layout B modern Oteryn; A is built first, then B. Final art for both is
   produced with AI tools, by agents or with an external AI tool, and every AI-produced asset gets
   an ASSET-PROVENANCE-2 record. New requirements: theme tokens (colours, fonts, spacing, sprite
   sets referenced by token, not hard-coded) and layouts described as data (one client, a data
   file per layout, no code fork). The switch is client presentation only and never changes
   gameplay information. Body: §1.20, packet UI-LAYOUT-6.
4. **Field of view (game window size).** **Owner ruling 2026-10-06.** Both arms of the viewport
   plan are built and switchable in the client for the A/B test: FOV-fixed (plan Variant B) with
   a standard and a larger fixed size, to judge whether a larger map gives too much advantage,
   and FOV-responsive (plan Variant A). The arm is its own setting, independent of `ui_layout`;
   this replaces the round-1 text that tied the fixed view to layout A and the experiment to
   layout B. The server grants each session's extent within a per-channel cap, so the client
   cannot widen its view past the cap; caps above 18x14 exist only on a non-production test
   channel; their cost is measured against ARCH-MAP-VIEWPORT-BUDGET-V1. The final policy stays
   undecided until the plan's evidence exists. Body: §1.21, packet MAP-VIEW-EXTENT-7.
5. **Q5. What is the larger fixed size (`FIXED_LARGE`)?** Today's view is 18x14; the server
   admits 15..=36 by 11..=28 (F23). The 18x14 snapshot measured 1.683 ms p99 against the
   2,000 us gate, so a linear estimate puts 22x16 (1.4 times the tiles) near 2.35 ms: the
   measurement decides whether it fits. The same estimate puts 20x16 near 2.14 ms; about 299
   tiles (for example 20x15) fit. a) 22x16 (+4 columns, +2 rows); b) 26x20; c) the largest of
   20x15, 20x16, 22x16 and 26x20 that measures within 2,000 us p99. **Recommendation: c**,
   because the estimate puts a and b over the budget; with §1.21 (e): if no candidate fits, the
   cap stays 18x14 and the packet returns a budget question. **PROPOSED default until the owner
   answers: c** (reversible; test channel only).

## 5. Rejected options

- gettext for UI text: weak argument and plural handling, no selectors.
- Server-side locale or server-rendered translations: a locale-aware server, more session state,
  text on the authority path (F13).
- A separate content key registry: duplicates typed identity (§1.6).
- Keys-only content text, even behind a capability: a client without a translation pack has no
  text, and it forces translation before alpha (§1.7).
- `TextRef` as an ungated additive field: deployed strict decoders reject it (F20, §1.7).
- A code fork or build per UI layout: two clients to keep in step (§1.20).
- A client-decided field of view: the client would decide how much of the map it receives;
  it only requests and the server grants within the cap (§1.21 b).
- The field-of-view arm tied to the layout (fixed in A, experiment in B): comparing arms would
  also change the layout, and switching layout would change the field of view (§1.21 a).
- Scanning every `include_str!` under the source trees: it catches test-only embeds and fixtures
  and would fill the catalogue with non-assets; the release dep-info decides (§1.17).
- Localized NPC keywords on the server: needs the locale on the server.
- Translations without a source hash: stale text shows silently.
- Own shaping or plural code, or a fluent-rs fork: upstream is sufficient (playable-first policy).
- Provenance in spreadsheets or commit messages: CI cannot check it.
- Account-synced accessibility now: needs a Platform contract (F12).

## 6. Open unknowns

- U1. How item and creature names reach the client (appearances data in `content/assets/files/`,
  or a later export). Fixed by the first client packet that shows a name; §1.6 applies either way.
- U2. Resolved by the Q2 ruling of 2026-10-06 (§4); distribution still waits for the records.
- U3. Whether quest journal text is admitted like NPC text (D9): the quest format rejects committed
  text (F8), yet the quest log serves journal text (F21). The quest content owner resolves it
  before the quest corpus record of ASSET-PROVENANCE-2 and before packet 5.
- U4. UI scale range, default and panel text steps: CANDIDATE, measured in packet 1.
- U5. The packaging layout of a shipped build: no workflow exists, so §1.17 scope is interim.
- U6. The final field-of-view policy: open until the viewport plan's evidence exists (§1.21).
