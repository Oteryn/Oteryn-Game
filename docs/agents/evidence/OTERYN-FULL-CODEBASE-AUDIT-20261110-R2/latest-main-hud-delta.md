# Successor audit: Windows manifest and HUD initialization diagnostics

**REVIEWED at `b028bef4ddcc6d48bdd6cf61479c94e42435ed88`**, compared with the previously reviewed successor `af33fd3baeac15d9991bd24c91bcacc8733492e2`. The main full-codebase ledger remains bound to `4f0b6c62a789489fca6c37dd36d5917d096f328e`. No checkout or production source was changed. This report is a separate successor record, not a claim that the whole repository was reaudited at the new SHA.

All **18 changed files / 4,071 tracked lines** are covered: twelve source/manifest files were read in full, and six localization documents received whole-document semantic validation plus review of their added translations. Exact Git bytes, SHA-256 and ranges were checked for every file. Detailed notes and hashes are in `latest-main-hud-delta.json`; exact input copies are in `latest-main-hud-input/`. No new confirmed finding arose from this delta. This is a static review, not independent verification of a successful Windows launch.

## Behavior and production callers

`apps/client/build.rs` replaces the former Common Controls dependency linker argument with `/MANIFESTINPUT` while preserving `/MANIFEST:EMBED` and the existing Windows MSVC condition. The input path is absolute and participates in build invalidation. The new `resources/windows.manifest` preserves the exact Common Controls 6 identity and adds the Windows 8, 8.1 and 10/11 supportedOS identifiers. This remains a package-wide link option, including the launcher, as the earlier dependency argument was.

The manifest addresses the relevant documented prerequisite: a layered child window requires the application to declare Windows 8 awareness. The unchanged ArcHud creates exactly a `WS_EX_LAYERED | WS_CHILD` window. The supplied identifiers match Microsoft's manifest table, and the input linker option is paired with its required embed option. These are source/configuration checks; no PE manifest resource or actual Windows behavior was inspected. Sources: [Using Windows](https://learn.microsoft.com/en-us/windows/win32/winmsg/using-windows), [Application manifests](https://learn.microsoft.com/en-us/windows/win32/sbscs/application-manifests), [MANIFESTINPUT](https://learn.microsoft.com/en-us/cpp/build/reference/manifestinput-specify-manifest-input?view=msvc-170).

`features/local-spell-action-bar/native.rs` is the actual shell HUD module. Its constructor now returns `Result<Hud, HudInitError>` and propagates pane, proficiency, arc, spell-button and shortcut failures. The changed constructors capture the error immediately after each failed native operation. Explicit cleanup follows capture, and constructors with partially built Rust owners let their destructors destroy the created subtree or buttons. Boxed callback state remains owned through teardown. The error distinguishes the failing part; the window-handle conversion deliberately uses code 0, and native cases report the captured thread-local last error. API-specific validity of every last-error code was not established by a native fault-injection test.

`windows_shell/application_handler.rs:32–38` handles this result on the real non-smoke path and retains `ShellError::HudInitialization`. `windows_shell.rs` formats the localized prefix followed by part/code and preserves the first fatal error through event-loop exit. The unchanged successor `main.rs:64–74` sends that formatted error to `crash_report::record_startup_failure`; the four files from the earlier af33 startup-diagnostics delta are byte-unchanged by b028. Settings attach now follows successful HUD creation. The existing smoke branch still exits before any HUD constructor, so a smoke pass would not validate this behavior.

The similarly named constructor in `cast_watch/native.rs` belongs to the separate `native_hud.rs` module and is unaffected. The legacy `windows_shell_methods_1.rs` text has no tracked module/include reference and is not the application handler compiled from `windows_shell.rs`. It is not evidence of a missed Result conversion in the actual caller.

## Coverage

| Area | Files | Review |
|---|---:|---|
| Build script and manifest | 2 | Full source/XML read; namespace, dependency identity and supportedOS checks |
| Native HUD, panes, arc, proficiency, spell row, shortcut strip | 6 | Full source read, constructor/error propagation, partial cleanup and unchanged callbacks |
| Portable error type, library export, shell and application handler | 4 | Full source read, actual module/caller and fatal-error propagation |
| de/en/es/pl/pt/sv locales | 6 | Full JSON parse with duplicate-key rejection and exact comparison to af33 |

Each locale contains 3,280 string entries. Every previous key/value is unchanged, exactly one new key (`client HUD initialization failed`) is present, and the new value is nonempty without format placeholders. The six new translations were reviewed. This does not claim a fresh manual rereading of every unchanged translation.

## Verification actually performed

The existing portable `hud_error::tests::error_names_the_failing_part_and_win32_code` was compiled directly from the exact b028 file using `/home/agent/.cargo/bin/rustc --edition=2024 --test` and executed: **1 passed, 0 failed**. This verifies the ArcWindow label/code 87 formatting assertion only. No replacement implementation or newly invented test was substituted.

Whole-document locale checks and XML checks passed. All 18 file hashes/ranges match b028 Git bytes, and the immutable checkout HEAD remains 4f. The test result and provenance are recorded in the JSON.

**Not run:** Cargo build/test/clippy/fmt; Windows MSVC linking; native HUD creation, input or cleanup execution; installer `/RELAUNCH`; smoke invocation; PR-reported checks or reviews. The PR description's execution claims are not adopted as our results. Actual installed startup success remains unknown.

## Existing findings

This change does not close any existing confirmed client audit finding. The item-action source validation and custom Use paths underlying CL-001/CL-002 are unchanged. Movement spell command text underlying CL-003 is unchanged; the locale semantic comparison preserves all old entries. The constructor/result changes leave the reviewed nonconstructor HUD action bodies unchanged. Other findings retain their separately documented scope and verification status. There is no new gameplay, wire or durable authority change in these 18 files.
