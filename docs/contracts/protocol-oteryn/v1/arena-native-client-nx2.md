# W17 native arena consumer (NX-2)

Delivery base: `81504ec792c186a3adcdc95aef8610d0862334cf`, the owner-selected
consolidated PR #3002 head. The uncommitted work was preserved as a binary patch,
moved onto that head and reapplied without a commit, push or history rewrite.
The original-base validation is historical only, not this candidate's evidence.

## Scope and production path

The base already contains the Tibiadrome server lifecycle, persistence, practice
exclusions, scoring, purchases and physical-wave tests. NX-2 consumes that owner
instead of introducing a duplicate server. It uses the shared `DromeAction`,
`ArenaAction::Drome` and `DromeResult` directly; no private wire codec remains.

Existing capability **170**, command **436**, action **6**. No new capability,
command, protocol message or migration number is allocated. Reserved capability
335–339, command 570–579 and migration 0990 remain unused.

Ctrl+Shift+Z opens the existing native arena window. Drome status, enter, leave,
lower starting level, claim potion and all 18 purchase selectors are connected:

`native.rs -> drome_intent -> PlayLink::arena_request -> PlayCommand::Feature ->
run_session -> gated_command -> command 436 -> arena_match_wire -> drome_command
-> PostgreSQL -> arena_result -> apply_result -> panel_text -> native EDIT control`.

Server implementation evidence on the delivery base:
`crates/gameplay/dispatch/src/gameplay_transport/arena_match.rs:173–174`,
`tibiadrome.rs:50`, `crates/gameplay/durability/src/durability/tibiadrome.rs`,
and migration `0973_tibiadrome_lifecycle.sql`. These are inherited implementation,
not new migrations or claims of independent canonical-rule approval.

## Wire and presentation

Request: singular uint32 varints, field 1 = 6; field 2 = operation
(1 status, 2 enter, 3 leave, 4 lower start, 5 claim potion, 6 buy);
field 3 = selector 1–18 for buy only. The client never sends kills, prices,
participants, deadlines, scores or reward-item identities.

The shared response decoder enforces 128 bytes, version 2, disposition 1–8,
phase 0–2, unique known uint32-varint fields and valid/truncated/overflowing data.
Fields 3–12 carry rotation level, starting level, points, phase, wave, remaining
seconds, cooldown, rank, unclaimed potions and the next rotation boundary.

Pending typed request context selects Svargrond or Drome decoding. Payloads are
not universally disjoint: the Drome version value overlaps Svargrond WrongState.
The serialized production session correlates each result to its command; no
heuristic guesses its family from the bytes. Rejections and malformed responses
release pending and preserve prior state; accepted/replayed replies update only
the requested panel state. Tests cover both decoder paths and malformed replies.

Native buttons and combos disable while pending or unselected. The panel shows
server progress, rank, cooldown, potion entitlement and all eight gameplay
outcomes. It never predicts progress, purchases or potion grants. Combat timing
is a server snapshot, explicitly refreshed by the player. The rotation boundary
alone is displayed as a labelled local-clock estimate. Status phase is rendered
as idle/active without inventing a preparation phase that this producer does not
send. Svargrond controls, reward groups, bytes and the existing key chord remain.
The native control presentation extends the base's accepted Arena v1 native UI;
no exact proprietary-client layout parity is claimed.

## Evidence and unresolved canonical facts

| Rule | Source / class | NX-2 treatment |
| --- | --- | --- |
| Svargrond ranks, fees and timing | Crystal `201189f871d23597b82caf88fd611454a2ddefb8`, `data-global/lib/quests/svargrond_arena.lua:9,118–173` (`OTS_HYPOTHESIS_ONLY`) | Existing controls and behavior preserved. |
| Independent Svargrond reward groups | Same Crystal revision, `data-global/startup/tables/chest.lua:2307–2419` (`OTS_HYPOTHESIS_ONLY`) | Existing claims preserved. |
| Drome access, group count, timing, exclusions, rotation and rewards | `docs/reference/tibia-manual/combat.md:249–284` (retained `CIPSOFT_OFFICIAL` capture notes); `docs/reference/content-prep/arena-rules-content-1/admission-evidence/tibiadrome-reference-evidence.json` (`TIBIAWIKI_STRUCTURED`) | Display server values; no client gameplay rule or new server substitute. |
| Potion identities | Same Crystal revision, `data/scripts/actions/items/drome_cube.lua:3–22` (`OTS_HYPOTHESIS_ONLY`) | Request the server-earned potion; never choose or reroll it. |
| Drome operations and selector ordering | Delivery-base `crates/protocol-oteryn/src/arena/drome.rs:5–136`, `docs/contracts/protocol-oteryn/v1/arena-v1.md`, Drome section | Use the shared producer types and decoder. |

**W17 overall: STOPPED on missing donor facts**, not reserved paths. Exact missing
facts are the canonical base health, damage/attack/condition profiles and
wave-level scaling formulas for **Murmillion, Scissorion, Hoodinion, Mearidion and
Domestikion**, plus their authoritative native-room placement. The base explicitly
substitutes qualified Svargrond opponents/attacks for Drome profiles/modifiers and
uses ten-percent scaling (`tibiadrome.rs`, `oteryn-choice` comments). Neither the
available official notes nor a donor implementation proves those substitutions
faithful; NX-2 does not silently accept them as current Tibia rules. In particular,
current Tibia's named Drome monsters and nine documented modifiers must not be
represented as completed by the existing Svargrond-profile substitute.

Both pinned donor trees were complete/non-truncated and had no Tibiadrome-named
lifecycle or named Drome monster paths. Current public main trees were also
checked: Canary tree SHA `666a60f6fef803b043cbaf1c7b5fab219da9bfa9` and Crystal
`2908aa026ebf0706fa9e74edfc3f7cb7a4ac3395`, neither truncated; the same path
search found none. This is a tree-path check, not a claim to have read every file.
No canonical monster stats or unaccepted scaling rule was invented.

Automated native-panel/session play and live PostgreSQL data reduction are
qualified separately below. Actual Win32 window execution is unavailable here:
only the Linux Rust target is installed and there is no Wine/Windows runtime.
This is a reported validation limitation, not a reserved-path stop or a claimed
successful graphical play test. No tools or targets were installed.

## NEED-FETCH — actual network refusals, not implementation permission requests

All calls used normal Python urllib, no spoofing, retries or protection bypass.
Retained evidence was used for work independent of these unavailable facts.

- HTTP 402 `Payment Required`:
  `https://tibia.fandom.com/api.php?action=query&prop=revisions&titles=Tibiadrome&rvprop=ids%7Ctimestamp%7Ccontent&rvslots=main&rvstart=2026-11-05T23%3A59%3A59Z&rvdir=older&rvlimit=1&format=json&formatversion=2`
- HTTP 403 `Forbidden`:
  `https://www.tibiawiki.com.br/api.php?action=query&prop=revisions&titles=Tibiadrome%7CMurmillion%7CScissorion%7CHoodinion%7CMearidion%7CDomestikion&rvprop=ids%7Ctimestamp%7Ccontent&rvslots=main&rvstart=2026-11-05T23%3A59%3A59Z&rvdir=older&rvlimit=1&format=json&formatversion=2`
- HTTP 403 `Forbidden`:
  `https://tibiawiki.com/api.php?action=query&prop=revisions&titles=Tibiadrome%7CMurmillion%7CScissorion%7CHoodinion%7CMearidion%7CDomestikion&rvprop=ids%7Ctimestamp%7Ccontent&rvslots=main&rvstart=2026-11-05T23%3A59%3A59Z&rvdir=older&rvlimit=1&format=json&formatversion=2`
- Each returned HTTP 502 `Bad Gateway`:
  `https://api.tibiadata.com/v4/creature/murmillion`,
  `https://api.tibiadata.com/v4/creature/scissorion`,
  `https://api.tibiadata.com/v4/creature/hoodinion`,
  `https://api.tibiadata.com/v4/creature/mearidion`,
  `https://api.tibiadata.com/v4/creature/domestikion`.

## Validation and review

All final checks below use the delivery base plus this patch. Sources and full
logs are retained under `/tmp/nx2-sources`, outside Git. No commit or push was made.

| Exact command / filter | Result |
| --- | --- |
| `cargo fmt --all --check` plus explicit `rustfmt --edition 2024 --check` for mounted native/play/PG files | PASS |
| `cargo clippy --locked -p oteryn-client-ui -p oteryn-client -p oteryn-gameplay-dispatch --all-targets --all-features -- -D warnings` | PASS, 3m 21s; existing vendored dependency warnings remain unchanged. |
| `env -u OTERYN_TEST_POSTGRES_ADMIN_URL cargo test --locked -p oteryn-client-ui -p oteryn-client --all-features --quiet -- --test-threads=1` | PASS: 314 client unit tests, 259 client-UI unit tests; 609 passing tests across all selected targets, zero failures/ignored. PostgreSQL is deliberately absent in this bulk run. |
| `cargo test --locked -p oteryn-gameplay-dispatch --all-features --lib nx2_arena_status_postgres_wire_receipt_and_generation_fence -- --nocapture` | PASS: own live PostgreSQL/native reducer case, `NX2_POSTGRES_EXECUTED`, **3.435607776 s**. Exercises real Drome status/refusal frames, native rendering state, Svargrond persisted replay, stale-generation rejection and unchanged receipt count. |
| `cargo test --locked -p oteryn-gameplay-dispatch --all-features --lib reaudit_arena_lifecycle_postgres_ -- --nocapture --test-threads=1` | PASS: physical wave/exempt death **38.339468224 s**; rotation/rank/rewards/replay **36.529191693 s**, 2 passed in 74.87s. Qualifies the inherited implementation, not its unproven canonical monster substitutions. |
| `oteryn-architecture-check workspace .` | PASS (`workspace-boundaries: PASS`); arena coverage supported, existing soft file-size warnings. |
| `git diff --check` | PASS |

Red evidence: the original panel-presence test failed without Tibiadrome; on the
new base, removing only the Drome reducer made
`tibiadrome_controls_cross_session_and_preserve_server_authority` fail with
`missing Drome reply` (exit 101). Restoring the reducer makes it pass in the final
client suite. `arena_pending_controls_close_on_disconnect_and_logout` also failed
red on the actual terminal handlers before `ArenaPanel::end` was wired into them;
it now passes. Shutdown disables controls and clears pending without inventing a
new score or discarding a previously decoded server snapshot. The native text is
updated only when changed, allowing the status viewport to remain scrollable.

The first consolidated-base bulk client run also exposed an inherited, repeatable
movement-control integration failure: movement SAY was queued as ordinary chat,
then its reply cleared the local-chat panel while movement waited on the public
chat panel. The isolated existing test
`movement_spell_controls_reach_session_and_decode_refusal_without_public_rooms`
failed in 3.00s. A minimal internal `MovementSpeech` queue variant preserves origin
and delivers the reply to the right panel; the same test passed in 0.09s. This
changes no server spell rule or wire ID. A later concurrent run hit the existing
`pc01_appearance_scenario` catalogue timing failure; the final complete client
runs are serial, with every assertion retained and no test skipped or loosened.
No claim of parallel-test reliability or graphical Windows execution is made.

Self-review covered every changed production/test file: wire bounds are delegated
to the shared codec; request context selects the decoder; capabilities and pending
are gated; native selector indices are checked; terminal paths release pending;
new functions have production callers; no runtime guard crosses an await; no
persistence/authority implementation, migration or pin is modified. The movement repair retains
normal chat's existing reducer and routes only movement-origin speech separately.
No full nextest, full PostgreSQL suite, generated pin/golden edit or external
publication was performed. The unresolved canonical-rule facts above remain the
reason W17 is not declared DONE.
