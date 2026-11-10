# Independent verification — CL-003

Input: `Oteryn/Oteryn-Game@4f0b6c62a789489fca6c37dd36d5917d096f328e`.
Disposition: **P2 CONFIRMED by independent static production trace**. Deferred to an implementation task. No source fix or native client/server test was performed.

## Actual trigger and effect

With the supported Polish locale selected, the native Levitate Up, Levitate Down and Magic Rope controls send translated display text in place of canonical spell words. Those messages do not match the canonical spells, so the server routes them to ordinary chat handling and does not invoke movement-spell execution. A normal chat policy may additionally refuse the speech; the confirmed effect is that the requested movement spell is not recognized.

`crates/client-ui/src/features/local-spell-action-bar/protocol.rs:11-21` applies `tr!` to the three wire incantations. The locale generator emits a macro calling `localization::tr`, and that function resolves the current language (`src/components/localization/build.rs:9-51`; `src/components/localization/mod.rs:47-76`). This is not merely a translated label. The actual Windows shell loads the saved language (`apps/client/src/windows_shell.rs:173-178`); the settings path also sets it (`features/local-reference-preferences/settings_native_state.rs:253`).

The three Polish catalogue entries in `crates/client-ui/src/components/localization/locales/pl.json` are:

| Canonical command text | Returned Polish value |
|---|---|
| `exani hur "up` | `„Wydaje mi się” up` |
| `exani hur "down` | `Wygląda na to, że „down”` |
| `exani tera` | `Wędrówka tera` |

The actual native controls reach the movement enqueue function (`apps/client/src/features/0259-chat-public-channels/native.rs:165-177`). That function inserts the returned string unchanged into `ChatIntent::Say` (`features/local-movement-spells/input.rs:9-24`). `play/session_loop.rs:374-381` and `play/session_adapter.rs:393-398` reach `Session::chat` (`crates/session/src/session_commands.rs:293-304`), which encodes the supplied text; it does not translate it back to canonical spell words.

The server's admitted speech path calls `self.spells.spoken(text)` and routes an unmatched message to normal chat (`crates/gameplay/dispatch/src/gameplay_transport/composed_admission/spells.rs:251-262`). `SpellBook::spoken` uses canonical ASCII-case-insensitive word prefixes and its parameter grammar (`crates/gameplay/content/src/content/spell_data/mod.rs:341-392`). Each Polish value above lacks the required canonical prefix.

The exact canonical content records were checked in `content/abilities/definitions/player-spells.json`: `/bundles/129/bundle/spell` is `candidate:spell/levitate`, words `exani hur`, text parameter, native `vertical_move` with choices `up`/`down`; `/bundles/136/bundle/spell` is `candidate:spell/magic_rope`, words `exani tera`, no parameter, native `vertical_move` mode `rope_up`. Inspecting these two JSON records does not count as a full review of that catalogue.

## Counterguards and scope

The selected-speech and pending-operation guards gate the request but do not restore canonical text. Current terrain, vocation, mana and movement permissions apply only after recognition and cannot repair a mismatching prefix. English controls and manually entering canonical words are outside the demonstrated failure. Other locales contain changed strings too, but this independently confirmed case is deliberately limited to the three Polish controls; no aggregate affected-user or deployed-incident claim is made.

Existing inline tests assert the English command strings under the default locale. They do not exercise the actual Polish catalogue through the native send path. A later repair should keep canonical spell commands separate from localized UI labels and cover every supported locale at the real enqueue/encoding boundary.
