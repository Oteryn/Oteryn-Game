# Oteryn universal client: one Rust client for Windows, Linux, macOS and the browser

Status: **Proposal (draft); no implementation authority.** The owner paused all implementation work on 2026-11-10.
Nothing in this file starts a stage. It extends, and does not replace, `ADR-0018-browser-client-web-ready-boundaries.md`
(Proposed) and `WEB_CLIENT_IMPLEMENTATION_PLAN.md` (planning candidate, Issue #519).

## 1. The idea

Oteryn keeps **one** client codebase in Rust and builds it for four targets:

| Target | Shell | GPU backend | Transport |
|---|---|---|---|
| Windows | native window (`winit`) | DX12, Vulkan fallback | TCP profile 1 (QUIC opt-in) |
| Linux | native window (`winit`) | Vulkan | TCP profile 1 (QUIC opt-in) |
| macOS | native window (`winit`) | Metal | TCP profile 1 (QUIC opt-in) |
| Browser | canvas, `wasm32-unknown-unknown` | WebGPU | browser profile (WebSocket or WebTransport, ADR-0018 §7) |

The game rules, protocol (`protocol-oteryn`), client state, UI and renderer are shared code. Only a thin platform layer
differs per target: window and input, file and credential storage, networking primitives, audio, process services.

A separate JavaScript/three.js client was considered and rejected: it would duplicate the protocol, client state and
UI and drift from the native client. The rejection reasons are in ADR-0018 §2.

## 2. Goals

- G1. One playable client on Windows, Linux, macOS and a current desktop browser, with the same gameplay and UI.
- G2. One implementation of protocol handling, client state and UI; a feature is written once and works on every target.
- G3. Windows stays the first-class target. Its performance and stability must not get worse at any step.
- G4. Every target talks to the same authoritative Game through `protocol-oteryn` and the Platform/Game Gateway
  admission path. No target-specific gameplay rules.
- G5. A new target or OS-specific bug is fixed in the platform layer, not in feature code.

## 3. Non-goals

- Mobile (Android, iOS) and consoles.
- A second client in another language or engine (three.js, Unity, Godot, Electron wrapper around a JS client).
- Changes to server authority, persistence, `session-generation` fencing or the multichannel world model.
- A production browser rollout. That stays behind the later stages of `WEB_CLIENT_IMPLEMENTATION_PLAN.md`
  (W7 to W10) and their own acceptance.

## 4. Requirements

### 4.1 Functional

- R1. All gameplay features available on Windows today are available on every target, with the same layout, hotkeys
  and options.
- R2. Text input works on every target, including Polish characters, IME composition, clipboard, selection and
  Tab focus order.
- R3. Login, character list and session entry use the existing Platform identity flow. Each target stores credentials
  in the OS store (Windows Credential Manager, Linux Secret Service, macOS Keychain) or, in the browser, the
  Platform's web session. No plain-text credentials anywhere.
- R4. Assets are the same pinned content on every target. Native targets read local files; the browser loads a hashed
  manifest and caches it (ADR-0018 §9).
- R5. Updater, crash reporting and the "About" window either exist on each target or have a target-specific
  replacement (for example, the browser has no updater; the page version is the client version).

### 4.2 Non-functional

- N1. Performance: the Windows frame time and memory budgets stay as they are measured today. Every target has its own
  budget, recorded before the target counts as supported.
- N2. Security: no new trust in the client. The browser target follows ADR-0018 §13 (CSP, no `eval`, same-origin
  assets, fail-closed version fencing).
- N3. CI: every supported target compiles in CI; Windows keeps all current gates. A target without CI coverage is
  not supported.
- N4. Dependencies follow `PLAYABLE_FIRST_ENGINEERING_POLICY.md`: upstream crates (`wgpu`, `winit`, `keyring`-class
  crates or direct OS APIs), no forks without evidence.
- N5. No test is weakened, skipped or deleted to make a target compile.

## 5. Current state (measured on `origin/main` `43f48d1f3`, 2026-11-10)

| Item | State |
|---|---|
| `crates/client-ui` | Platform-neutral **view-model layer** (state, actions, protocol, text), not a widget toolkit: 732 files, about 45,600 lines, 141 features. Depends only on `client-domain`, `client-runtime`, `input-actions`, `renderer`. It has no layout, widgets, focus or text editing (audit, 2026-11-11). |
| Text rendering | The only renderer text is `crates/renderer/src/name_text.rs`, a 5x7 ASCII bitmap: no Polish characters. |
| `apps/client` | 1,023 files, about 138,100 lines. |
| Win32 UI code in `apps/client` | 190 files, about 39,700 lines, in 65 features plus about 9 shell/system files. 0 of these are test files. |
| Win32 features by `client-ui` coverage | 30 complete, 30 partial, 2 skeleton, 3 none (audit, 2026-11-11). Another 31 files (3,328 lines) use Win32 through `use super::*`. |
| Largest Win32 areas | options/preferences (40 files), Wheel of Destiny (10), Cyclopedia character (9), chat (8), spell action bar (5), Store (5), attack (5), account achievements (4). |
| Non-feature Win32 files | `about.rs`, `cast_watch/native_hud.rs`, `crash_report.rs`, `device_login/confirmation.rs`, `play/status_window.rs`, `updater/window.rs`, `win_mutex.rs`, `windows_shell/application_handler.rs`, `windows_shell/capture.rs`. |
| Renderer | `wgpu = "=30.0.1"` with features `std`, `dx12`, `wgsl` only. |
| Window | `winit = "=0.30.13"`; `windows-sys` for UI, credentials, shell and threading. |
| Runtime | Tokio multi-thread with `net` (not available in the browser). |

The line counts come from a pattern search (`windows_sys::Win32::UI`, `CreateWindowExW`, `SendMessageW`,
`GetDlgItem`), not a full read.

## 6. The four steps

| Step | What | Result | Covered by |
|---|---|---|---|
| 1 | Turn on more `wgpu` backends (Vulkan, Metal) and build the client for Linux and macOS behind `cfg`. | The game window opens and renders on Linux and macOS; Win32-only parts are switched off there. | new, this proposal |
| 2 | `wasm32` compile audit and platform seams: storage, time, threads, networking, audio behind traits (W1, W2). | Shared crates compile for `wasm32`; the native client uses the same seams. | `WEB_CLIENT_IMPLEMENTATION_PLAN.md` W1-W2 |
| 3 | Move all UI off Win32 into `client-ui`, and put system services behind platform interfaces. | No Win32 UI left; every window and dialog is the same on every target. | new, this proposal (§7) |
| 4 | Browser shell, WebGPU qualification, asset manifest, browser transport (W3-W6, then W7-W8). | The client runs in a browser against a non-production server. | `WEB_CLIENT_IMPLEMENTATION_PLAN.md` W3-W8 |

Steps 1 and 3 give Windows, Linux and macOS. Steps 2 and 4 add the browser. Step 3 is the largest step and is needed
by every non-Windows target.

## 7. Step 3 in detail

### 7.1 Work

| Part | Content | Estimate |
|---|---|---|
| 3a. Widget kit and text in `client-ui` | Layout, buttons, checkboxes, combo box, scrolling and multi-column/tree lists, focus and Tab order, rendered context menu and modal dialog, text field (caret, selection, IME, clipboard), and a Unicode font renderer so Polish characters display. An upstream text crate is preferred over writing one (dependency custody review). Win32 provided all of this. | about 22 person-days |
| 3b. Feature switch-over | 65 features: 30 complete, 30 partial, 2 skeleton (minimap, options/preferences), 3 none (account services, character selection, native login). Sized S 32, M 24, L 9. Options/preferences (40 files, 6,808 lines) is about three L tasks on its own. Four groups of near-duplicate Win32 windows (in 0195, 0153, 0230, 0148) are merged, not ported. | about 109 person-days (+ about 10 for options) |
| 3c. Non-game windows | About, status window, device-login confirmation, crash report, updater window, cast-watch HUD; mostly Yes/No prompts. Only the status window has a `client-ui` equivalent. | about 4 person-days |
| 3d. System services behind interfaces | Credentials, single-instance lock, shell integration and capture behind a platform trait with a Windows implementation; Linux/macOS implementations come with step 1. | a few days |

Total: **about 135-145 person-days.** 3a comes first and is mostly serial (about 4-5 weeks); 3b then splits well
across 3-4 parallel lanes. With lanes, review and CI that is roughly **3-4 months** of calendar time; for one developer,
6-7 months. The earlier figure of 5-8 weeks was too low: it assumed `client-ui` already drew its own widgets.
Per-feature table: `reviews/CLIENT-UI-WIN32-AUDIT-2026-11-11.md`.

### 7.2 Uncertainty

- The audit read about 17 features in full and the headers of 12 more; the rest are classified from automated
  signals. The 3a figure is an estimate, not a measurement.
- Text input in a game-drawn UI (IME, Polish characters, clipboard) usually takes longer than planned.
- Linux and macOS testing (Vulkan/Metal drivers, HiDPI, keyboard layouts) adds about 1-2 weeks outside step 3.

## 8. Success criteria

- S1. `apps/client` has no `windows_sys::Win32::UI` use outside the Windows platform module.
- S2. The client builds in CI for `x86_64-pc-windows-msvc`, `x86_64-unknown-linux-gnu` and `aarch64-apple-darwin`;
  shared crates build for `wasm32-unknown-unknown`.
- S3. A scripted session (login, enter the world, walk, open options, chat, use a spell, log out) passes on Windows,
  Linux and macOS.
- S4. Windows frame-time and memory budgets are not worse than the baseline recorded before step 3.
- S5. Every feature test that ran before still runs and passes; UI tests run against `client-ui` on every target.

## 9. Risks

| Risk | Effect | Mitigation |
|---|---|---|
| Game-drawn text input is weaker than native; today's font has no Polish characters | Players notice at once (chat, login) | Do 3a first; test Polish layout and IME before any feature switch |
| Step 3 conflicts with ongoing client feature work | Merge conflicts in hot client files | One feature per PR, package per feature area, rebase-free merges of `origin/main` |
| Windows regression | First-class target gets worse | Baseline measurement before 3b, budget gate in CI |
| macOS needs signing and notarisation | Unsigned builds are blocked by Gatekeeper | Owner decision on an Apple developer account before macOS release |
| Browser transport and admission are not decided | Browser target blocked after step 4 shell | Kept in W6/W7 of the web plan; native targets do not wait for it |

## 10. Order and dependencies

1. Owner decision on this proposal (taken, §11) and on ADR-0018.
2. Audit of `client-ui` coverage (done 2026-11-11, `reviews/CLIENT-UI-WIN32-AUDIT-2026-11-11.md`).
3. Step 3 (3a, then 3b and 3c in parallel lanes, 3d) and step 1 in parallel.
4. Step 2, then step 4 under the web plan's gates.

## 11. Owner decisions (2026-11-11)

- D1. Goal accepted: one Rust client for Windows, Linux, macOS and the browser.
- D2. Order: native Linux and macOS first (steps 1 and 3), browser later (steps 2 and 4).
- D3. The one-day audit of the 61 `client-ui` views runs before any code work.
- D4. macOS: built and tested in CI without signing, and given to testers unsigned. The Apple Developer account
  (needed for signing and notarisation) is bought only before a Mac release to players. Windows and Linux are the first
  player releases.

The decisions accept the direction of this proposal. Implementation still needs the owner to lift the 2026-11-10 pause,
and ADR-0018 stays Proposed until accepted on its own.
