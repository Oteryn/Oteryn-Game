# Universal client: work plan

Status: plan for owner review (2026-11-11). Companion to `UNIVERSAL_CLIENT_PROPOSAL.md`; it does not lift the
2026-11-10 implementation pause and accepts no ADR.

## 1. Goal

One Rust client, the final player client, for Windows, Linux and macOS from one code base. The browser follows later
(owner decision D2). Every window and dialog is drawn by the game and looks the same on every system.

## 2. What we have

| Area | State | Cross-platform? |
|---|---|---|
| Window and input | `winit 0.30.13` | Yes. Windows, X11, Wayland, macOS and web. |
| GPU | `wgpu 30.0.1`, features `dx12` and `wgsl`. Tests already use `vulkan`. | Yes, once the Vulkan and Metal backends are turned on. |
| Game rendering | `crates/renderer`, sprites and map | Yes (pure wgpu) |
| Game state, protocol, network | `client-domain`, `client-runtime`, `session`, `protocol-oteryn`, Tokio | Yes (native) |
| UI logic | `client-ui`: 141 features of state, actions and text; tested | Yes. It has no widgets and no layout. |
| UI drawing | Win32 dialogs in `apps/client`: about 39,700 lines, 190 files, 65 features | No. Windows only. |
| Text | Win32 draws dialog text. The renderer has only a 5x7 ASCII bitmap, with no Polish characters. | No |
| Audio | `rodio 0.21` (cpal) | Yes. WASAPI, CoreAudio and ALSA/Pulse. |
| Credentials, single instance, shell, capture, updater, crash report | `windows-sys` | No |
| CI | Windows build and Linux tests | Linux build of the client: no. macOS: no. |

## 3. What we need, and what to use

Upstream first: take mature crates and write only the glue.

| Need | Choice | Why / alternative |
|---|---|---|
| Widget kit (layout, buttons, lists, tabs, scrolling, focus, modal dialogs, text field with caret, selection, IME and clipboard) | **`egui` core** (MIT/Apache), drawn through our own small painter on the existing `wgpu 30` renderer | egui is mature and renderer-agnostic: it outputs triangle meshes and a font atlas. It handles text input, IME, Unicode, scrolling and focus, and it runs in the browser too. This replaces about 22 person-days of the hand-written kit in the proposal (part 3a). Alternatives: our own kit (slow, high risk on text input); `iced` (its own runtime, harder to put inside the game loop); Slint (GPL or commercial licence, rejected). |
| egui to winit glue | `egui-winit`, if its winit version matches 0.30; otherwise about 300 lines of our own glue | Check in F0. Do not take `egui-wgpu` if it pins a different wgpu: our painter is small. |
| Fonts with Polish characters | Bundled OFL font: Noto Sans or Inter for the UI, plus one OFL pixel font for in-game names | egui uses TTF/OTF directly. Licence: OFL allows bundling. |
| Tibia-like look (frames, buttons, scrollbars) | egui `Style` plus 9-slice textures | The skin images must be our own or carry redistribution rights. Third-party client art is reference only. |
| Clipboard | `arboard` (or the egui-winit clipboard) | Windows, macOS, X11 and Wayland |
| Credentials | `keyring` | Windows Credential Manager, macOS Keychain, Linux Secret Service |
| Single instance | Lock file in the user data directory (`fs4` or our own) | Replaces the Win32 named mutex |
| Data and config paths | `directories` | Correct per-OS paths |
| Audio | `rodio` (have) | Linux CI needs `libasound2-dev` |
| Packaging | Windows: current installer. Linux: AppImage and `.tar.gz`. macOS: `.app` and `.dmg` via `cargo-bundle`, unsigned (D4). | Signing and notarisation only before a Mac release to players |
| Updater | Per-OS downloader behind a trait; the UI goes through egui | Windows keeps today's logic |
| Crash report | Panic hook plus a report written by the game; the dialog goes through egui | No OS dialog needed |

No game engine (Bevy, Fyrox and the like) is needed: the game already has its own renderer, loop and state. Switching
engines would mean rewriting working code, not porting it.

## 4. Tests

| Level | What | Tools |
|---|---|---|
| Logic | Current `client-ui` and `client-runtime` lib tests, unchanged | `cargo test` |
| UI behaviour | Every ported window: open it, click, type Polish text, check the action sent | `egui_kittest` (egui's test harness; finds widgets by label through AccessKit) |
| Pixel snapshots | Key windows rendered off-screen and compared with a stored PNG within a tolerance | `egui_kittest` snapshots, wgpu off-screen; Linux CI uses software Vulkan (Mesa lavapipe) |
| Architecture | Rule: no `windows_sys::Win32::UI` outside `platform/windows`. A baseline list of remaining files may only shrink. | `oteryn-architecture-check` |
| Build matrix | Client builds for Windows x64, Linux x64 and macOS arm64 | GitHub runners `windows-latest`, `ubuntu-latest`, `macos-14`. macOS minutes cost 10x: build on every PR, run the full set only on merge. |
| End-to-end | Scripted session on each OS: login, enter the world, walk, chat, open options, cast a spell, log out (criterion S3) | Local server in Docker with PostgreSQL; the client in script mode |
| Performance | Windows frame time and memory before and after (criterion S4) | Measured baseline before F2, budget gate |
| Manual | Windows 10/11, Ubuntu 24.04 (X11 and Wayland), macOS 14 arm64; Polish keyboard, IME, HiDPI 150%/200% | Checklist per release candidate |

## 5. Templates

- **Port packet** (one per feature, for Codex), carrying:
  - the feature, owned files and the matching `client-ui` view;
  - the egui view to write;
  - the `egui_kittest` test to add (red before, green after);
  - the Win32 file to delete;
  - the baseline line to remove.
- **egui view skeleton**: one function `fn show(ui, &ViewModel) -> Vec<Action>`. The view holds no state and makes no
  network calls; all logic stays in `client-ui`.
- **Platform trait skeleton**: `trait Platform { credentials, single_instance, open_url, data_dir, capture }`, with
  implementations in `platform/windows`, `platform/linux` and `platform/macos`.

## 6. Order of work

Times are wall-clock time with Codex lanes, using the tempo measured on 2026-11-11: about 20 minutes for six fixes plus
one feature.

| Phase | Content | Exit test | Time |
|---|---|---|---|
| F0 spike | egui core and our painter in the game window; one Polish text field; the About window ported; check the egui-winit and wgpu versions | Polish text typed, copied and pasted; the window draws over the map; go / no-go on egui | 0.5 day |
| F1 foundation | `client-ui-egui` crate (painter, skin, fonts, input/IME/clipboard); platform trait and Windows implementation (3d); architecture rule with baseline; Vulkan and Metal on; client build behind `cfg` for Linux/macOS (step 1); CI build matrix | Game window renders on Linux and macOS; Windows unchanged | 1-2 days |
| F2 features | 65 features in 4 parallel lanes, one lane per area so lanes share no files. Lane 1: options and preferences (3 tasks). Lane 2: Wheel, Cyclopedia, Bestiary. Lane 3: chat, social, party, guild. Lane 4: Store, account, market, rest. | Each feature: kittest green, Win32 file deleted, baseline shorter | 2-4 days |
| F3 entry windows | Login, character list, device login, status, updater, crash report | Full start-to-game path without Win32 | 1 day |
| F4 OS bring-up | Linux X11/Wayland, macOS Metal, HiDPI, keyboard layouts; packages; E2E on 3 systems; performance gate | S1-S5 met | 2-5 days |
| F5 browser | Steps 2 and 4 of the proposal, later (D2) | Under the web plan's gates | later |

Native total: about **1-2 weeks wall-clock**. The pure writing is about 1-2 days of that. The rest is review, CI and
testing on real systems.

## 7. Needed from the owner

1. Lift the pause for this work.
2. Accept egui as the UI kit (it replaces the hand-written kit in part 3a).
3. Approve by name the workflow change that adds the Linux and macOS build matrix (macOS runners cost money).
4. Accept the OFL fonts and the rule that skin images are our own art.
5. Access to a Mac and a Linux machine (or testers) for F4; CI covers the build, not how the windows feel.
