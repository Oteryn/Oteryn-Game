# OTV2-20261013 — supported hotkey/profile behavior batch

```yaml
task_id: OTV2-20261013-client-hotkey-authoring
issue: 1927
pr: 1942
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
branch: feat/client-completion-20261009
owner: root
```

## Scope and boundary

Owner continuation of OTV2-20261009-client-game-ui, one root writer, draft1942
AUTHORING. Owned client files: preferences_hotkey_pages.rs, hotkeys.rs,
game_ui.rs and settings.rs. No server/protocol, identity, deployments, accounts,
reference asset publication or new capabilities. No paid/final review claimed.

One supported-profile batch: text UTF-8 byte/control validation, bounded custom
action/profile creation, Unicode duplicate and auto-switch matching, and legacy
unavailable Spell/Object keys. Existing accepted consumers are retained; their
unavailable pickers/bindings remain disabled and legacy payloads remain loadable.

## Acceptance

Actual egui OK/Apply clicks accept64 emoji/128 two-byte letters but refuse the
next byte-budget step, controls/blank text and an already full256-action profile.
Refusal keeps entered text, profiles, selection and action-bar mirrors. Add,
Copy and Rename use the backend's existing trim/to_lowercase rule, not invented
Unicode normalization. Own case-only rename is allowed; bounded creation cannot
overflow32 profiles. Auto-switch uses the same retained-name matching rule.

Legacy Spell/Object data round-trips unchanged but neither resolver nor native
router claims its physical key or queues a command. Supported Text still uses
the real composer/typed chat path. Local locked client all-target tests, strict
Clippy and fmt plus a fresh exact-source Windows build/page audit are required.
Physical clipboard, original modal-Apply semantics and gameplay are not proved
by these UI transaction/component tests.

Linux locked all-target client228/199/5 tests, strict Clippy and workspace fmt
pass; governance60/diff pass. Exact sourcefabf441a1a45a747bad9f82c1a0b994b517d8e95
passes MSVC227/204/5+Session82, strict all-target Clippy/fmt/release and DX12
20pages/78independently checked hashes/0invalid at125%DPI. Binary/manifest hashes
and retained diagnostic failures are in UI-FIDELITY.md. Current loginPID38236,
window8065072, expected source title and path/hash are verified; old37764 responds.
No automatic login, account, deployment, restart, install or trust-store mutation.
Last same-head CI read:7successful/6running/0failed, not final CI/review proof.

## Dependencies and preserved evidence

Published MAP/PACE checkpoint668d5a8d is docs-only over runtime4ff71216. Its native
proof does not qualify this changed client. E2E execution needs are visible at
https://github.com/Oteryn/Oteryn-Game/issues/1622#issuecomment-6149042654 and in
OTV2-20261013-step-pace-client-1.md. Work continues on this unblocked batch while
server2041/ON-OFF authorization and dedicated QA setup remain external needs.
Latest bounded read: no supplied setup/approval;2041 remains unmerged but now at
e4ac000cabc760f693847d8d9b80a0ae17d87dd6, superseding the earlier4f4fbadc head.
