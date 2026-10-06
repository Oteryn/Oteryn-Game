# ARCH-I18N-A11Y-CREATIVE-0: player text, accessibility baseline and asset provenance

- Decision id: ARCH-I18N-A11Y-CREATIVE-0. Gaps: `UX-I18N-A11Y-01`, `CREATIVE-DIRECTION-01`.
- Origin: owner ruling 2026-10-06: localization, accessibility and creative direction are decided now.
- Status: the §1 rulings and the §3 packets are accepted on merge. Every contract amendment in §2
  is exact text marked pending: it is applied by the named packet, because the owning file is a
  candidate (`ALPHA-CLIENT-01`) or because the change lands with the packet that builds it. The
  owner questions of §4 are pending; the recommended option is the working assumption until the
  owner answers. Creative taste is the owner's (§1.14); this decision fixes only structure.
- Owning texts amended: `ALPHA-CLIENT-01` §12; content tree contract §8; ARCH-ERROR-CODES-0 §1.9
  (localization item). Model: ARCH-ERROR-CODES-0.
- Related: ARCH-LIVE-READINESS-0 §2 amendment A3 replaces the release-display item of the same
  ERR-CODES §1.9; the two replacements touch different sentences and do not conflict.
  ARCH-ALPHA-OPS-0 §4 ships assets inside the build, which is the build that §1.17's manifest checks.

## Implementation brief

1. Four kinds of player text, four rules. UI text: Fluent keys in client catalogs. Outcome text
   (errors, dispositions, fixed notices): a typed code or enum on the wire, the sentence in the
   client. Content text (item, creature, NPC, quest, achievement): the server keeps sending
   English and later adds a stable text reference. Player text (chat, names, books): never
   translated.
2. The client uses Project Fluent (`fluent-bundle`, `fluent-langneg`, `unic-langid`). Catalogs:
   `apps/client/locales/<bcp47>/*.ftl`. `en` is complete and is the fallback for every message.
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
    ORIGINAL, LICENSED, OWNER_CLEARED_THIRD_PARTY or REFERENCE_ONLY.
15. A shipped-build manifest and a CI validator fail any input that is REFERENCE_ONLY or has no
    record.
16. Packets: I18N-CATALOG-0 (after ERR-CLIENT-2), then A11Y-BASELINE-1. ASSET-PROVENANCE-2 runs
    in parallel. CREATIVE-DOC-3 needs the owner. I18N-TEXT-RENDER-4 follows P4-T.
    I18N-CONTENT-KEYS-5 is deferred until a content locale is chosen.
17. Owner questions (§4): Q1 first locales; Q2 shipping Tibia-sourced text; Q3 the alpha UI look.

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
  journal `text` (1,024 B, from `apps/game-server/src/content/project_fs.rs` `read_journal`);
  `chat_v1.proto` `ChatLineV1` `speaker_name` + player `text`.
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

1.7 **Content on the wire.** Unchanged until the first content locale is accepted (Q1). That packet
   adds an optional `TextRef {key, source_sha256}` beside each content text field it touches; the
   English field stays filled. An older client ignores the unknown field (F19); a newer client on
   an older server shows English. No capability is needed, since no meaning changes. The `key`
   byte limit goes to `RESOURCE_LIMITS_REGISTRY.json` in that packet: CANDIDATE, set from the
   longest identity measured.

1.8 **Stale translations.** A translation records its English source SHA-256 and is shown only
   while that equals `source_sha256`; otherwise English. NPC references already carry the hash
   (F8); items and quests compute it at content build.

1.9 **Corpus translation is off the alpha path.** Overlays are gettext PO at
   `content/translations/<bcp47>/<family>.po` (new): `msgctxt` = key, `msgid` = English, `#.` =
   hash. PO is chosen for mature translator tooling (Weblate, Poedit; `polib` in tools). The
   content build compiles a client pack. Nothing is built before Q1 selects a content locale.

1.10 **NPC keywords** stay canonical English tokens matched on the server (F7). A client may show
   a reply's keywords as clickable items with localized labels that send the canonical token.
   Server-side localized aliases are out of scope.

1.11 **Ordering.** The server orders paged lists by canonical text or key (F6). The client may
   re-sort a fully loaded list with locale collation; paging never uses translated text.

1.12 **Rendering** covers Latin Extended-A (Polish), has a fallback font and follows the UI baseline §7 scaling
   (`OTERYN_NATIVE_CLIENT_UI_ARCHITECTURE_BASELINE_2026-09-10.md`).
   The UI font is Noto Sans (SIL OFL 1.1), recorded as LICENSED, unless Q3 or the creative
   document picks another OFL or owned font. The shaping library stays with P4-T, CANDIDATE
   cosmic-text (upstream rustybuzz and swash). Evidence: a `pl` sample, a fallback glyph, and the
   frame cost at 1080p and 4K. No fork.

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
   art and new text. Taste beyond Q1–Q3 stays open and does not block alpha.

1.16 **Provenance record.** Every shipped asset file and text corpus has one record in
   `content/assets/catalog/`: `asset_id` (typed, never a source id), `path`, `sha256`, `class`,
   `source`, `license_id` (SPDX where one exists), `attribution`, and `owner_record` (a locator,
   required for OWNER_CLEARED_THIRD_PARTY). Classes: ORIGINAL, LICENSED,
   OWNER_CLEARED_THIRD_PARTY, REFERENCE_ONLY (may be stored, never shipped). The 15.30 set is one
   OWNER_CLEARED_THIRD_PARTY package with a per-file manifest citing D154 / Q12a. Text corpora get
   one record each: item text, NPC dialogue (D9), quest journals, achievements.

1.17 **Shipped-build manifest.** A generator lists the files and corpora a client or server build
   packages. A validator fails an entry that has no record, is REFERENCE_ONLY, differs from its
   record hash, or is LICENSED without attribution in the shipped notices file. Until packaging
   exists (F18), it checks what the client embeds or loads, plus every record, in the content path
   class of `game-gate`. This decision grants and narrows no rights; rights come only from owner
   records (Q2).

1.18 **Stale text.** ASSET-PROVENANCE-2 adds one line under "Non-claims" in the asset-version
   decision, pointing to the 2026-09-29 supersession (F16).

1.19 **Before freeze.** (1) The amendments below are exact text. (2) No concurrent transition is
   added. (3) Restart needs only the `OS_USER` settings file. (4) References are typed
   (`<identity>#<field>`, `asset_id`). (5) Older clients get generic fallback text and ignore the
   additive `TextRef`. (6) Content text and its `TextRef` leave in one message from one path.

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

- **I18N-CATALOG-0** (Fluent in the client; after ERR-CLIENT-2 merges). Owned: `apps/client/Cargo.toml`,
  `apps/client/src/` (new `text` module; call sites in `spell.rs`, `cyclopedia.rs`, `error_text`),
  `apps/client/locales/en/` (new), `Cargo.lock`. Builds §1.2–§1.5. Tests: missing, unused or
  mismatched id fails; unknown enum shows generic text; `pl-PL` falls back per message; error
  fallback tests unchanged.
- **A11Y-BASELINE-1** (after packet 0). Owned: `apps/client/src/`, `crates/input-actions/src/`.
  Builds §1.13 (a)–(e) and the colour-only audit list. Tests: remap round-trips through the
  settings file; conflict and reserved key refused; the UI baseline §7 formula; flash rate ≤ 3/s; legibility
  check fixes the CANDIDATE range.
- **ASSET-PROVENANCE-2** (parallel). Owned: `content/assets/catalog/`, `tools/asset-provenance/`
  (new), CI path routing, one line in `OTERYN_CLIENT_ASSET_VERSION_OWNER_DECISION_2026-09-27.md`.
  Builds §1.16–§1.18. Tests: REFERENCE_ONLY fixture fails; missing record fails; hash drift fails;
  LICENSED without attribution fails; the 15.30 package passes.
- **CREATIVE-DOC-3** (owner). Owned: `docs/architecture/OTERYN_CREATIVE_DIRECTION.md` (new) with
  the §1.14 sections; the owner fills or accepts it, and the Q3 answer goes in §5.
- **I18N-TEXT-RENDER-4** (after P4-T evidence and packet 2). Owned: `apps/client/`,
  `content/assets/` (font and record), `Cargo.lock`. Tests: `pl` golden render, fallback glyph,
  frame-cost measurement.
- **I18N-CONTENT-KEYS-5** (deferred until Q1 picks a content locale; protocol review). Owned:
  `content/translations/` (new), `tools/i18n/` (new: PO extract, validate, pack), touched `.proto`
  files and registry limits. Builds §1.6–§1.9. Tests: older-client fixture ignores `TextRef`;
  stale hash shows English; placeholder mismatch fails; ordering unchanged.

Order: 0, then 1 and 4; 2 in parallel; 3 waits for the owner; 5 last.

## 4. Owner questions

1. **Q1. Which locales ship at alpha?** Polish is the main audience; content is English Tibia text.
   a) `en` only, `pl` pipeline ready; b) `en` + `pl` for UI and outcome text, content in English;
   c) `en` + `pl` including content. **Recommendation: b** (one `.ftl` set; Reference content
   stays English).
2. **Q2. May the builds you distribute serve Tibia-sourced text?** NPC (D9), quest, item and
   achievement text is admitted as reference data; shipping it is "assessed separately" (F14,
   F17). a) yes, extend the 2026-09-29 clearance to these corpora as OWNER_CLEARED_THIRD_PARTY;
   b) closed alpha only, re-ruled before any public release; c) no, REFERENCE_ONLY, and alpha
   needs Oteryn-authored text. **Recommendation: b.**
3. **Q3. What look does the Oteryn-drawn UI have at alpha?** Reference sprites are fixed; chrome,
   panels and font need theme tokens. a) classic-faithful layout, drawn originally; b) modern
   Oteryn style; c) neutral placeholder until the creative document exists.
   **Recommendation: a** for the Reference profile.

## 5. Rejected options

- gettext for UI text: weak argument and plural handling, no selectors.
- Server-side locale or server-rendered translations: a locale-aware server, more session state,
  text on the authority path (F13).
- A separate content key registry: duplicates typed identity (§1.6).
- Keys-only content text behind a capability: breaks older clients, forces translation before
  alpha (§1.7).
- Localized NPC keywords on the server: needs the locale on the server.
- Translations without a source hash: stale text shows silently.
- Own shaping or plural code, or a fluent-rs fork: upstream is sufficient (playable-first policy).
- Provenance in spreadsheets or commit messages: CI cannot check it.
- Account-synced accessibility now: needs a Platform contract (F12).

## 6. Open unknowns

- U1. How item and creature names reach the client (appearances data in `content/assets/files/`,
  or a later export). Fixed by the first client packet that shows a name; §1.6 applies either way.
- U2. Whether any owner record clears shipping of Tibia-sourced text (feeds Q2).
- U3. Whether quest journal text is admitted like NPC text (D9): the quest format rejects committed
  text (F8), yet `read_journal` serves text. The quest content owner resolves it before packet 5.
- U4. UI scale range, default and panel text steps: CANDIDATE, measured in packet 1.
- U5. The packaging layout of a shipped build: no workflow exists, so §1.17 scope is interim.
