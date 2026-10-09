# OTV2-20261009 — complete native game client

Status: implementation in progress; private local source, not frozen or merged.
Owner scope: complete client and necessary engine/protocol functionality together.
No production deployment or database reset. Single writer: root.

First implementation batch consumes authoritative session projections for inventory,
open container, visible actors and chat; sends bounded chat intents through Session;
fits the map between panels with a shared rendering/picking transform; persists
inventory/battle/chat visibility with backwards-compatible defaults; separates text
and modal input from movement. Vitals are shown only when supplied by the session.

Owned paths: apps/client/src/{game_ui,layout,play,settings,settings_ui,windows_shell,
main,lib}.rs and crates/renderer/src/windows.rs. Existing private login/settings
implementation remains intact. The Windows source copy was compared against the
cloud checkout; differences in the six replaced existing files are only this batch.
Original Windows source backed up before replacement.

Validation: Linux client library tests 81 passed; strict library/test Clippy passed;
Windows GNU cross-check passed. Native Windows tests passed (81 library + 2 application); release build passed.
Client launched on owner PC via the existing local launcher. Login window
verified; portal unavailable because Docker Desktop engine was stopped.
Docker launch inherited a missing ProgramData variable from the remote shell.
Restored the canonical CommonApplicationData path for the child process only.
Docker now running; existing Game/Platform/gateway/database containers resumed
automatically. No factory reset, DB reset, or test-password rotation.
Native Windows strict Clippy also passed.
No native runtime claim before build and UI inspection.

Remaining completion scope: item names/icons and actions, full actor rendering/HUD,
minimap, skills/quests/social/cyclopedia panels, action bars and hotkey profiles,
chat filters/notifications/accessibility, audio, captures, Linux GUI composition.
Add required owning engine projections and commands, including missing mechanics;
current server capability availability is a sequencing constraint, not a scope cut.
Draft reference audit PR #1941 contains documentation only, not this product source.

Portal and public directory HTTP 200. Directory initially empty: the Game
container restarted only its sleep entrypoint, not the local server process.
Reviewed existing restart-game-ui.sh: scoped local node authorization/assignment
replacement, no database reset, account mutation or factory reset. Restarting
the existing local node in Ubuntu-24.04 with preserved character data.

Local restart was blocked by expired one-day qualification PKI (2026-10-09
07:53 UTC). Renewed 13 local certificates for 30 days, preserving existing keys,
subjects, SAN/EKU and local issuer boundaries; validated chains before replacing
23 matching public-certificate copies. Backups retained in local runtime tree.
Reloaded PostgreSQL and nginx certificates; restarted only the local gateway.
No DB/schema/account write, password rotation or production trust change.
