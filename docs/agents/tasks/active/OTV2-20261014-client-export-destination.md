# OTV2-20261014 — Options export destination ownership

```yaml
task_id: OTV2-20261014-client-export-destination
issue: 1927
pr: 1942
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
branch: feat/client-completion-20261009
owner: root
```

Continuation authorized by the owner; sole root writer, parent client-game-ui,
draft AUTHORING. Owned runtime: apps/client/src/options_export.rs only. Native
component qualification reuses isolated builds/fixtures, not owner preferences,
clipboard,accounts,deployments,trust or later gameplay capabilities. No workers,
paid resources,dependencies,new serializer/import format or HUD redesign.

## Bounded finding and repair

DERIVED pending Windows regression: existing active files already canonicalize
through std::fs and are protected. When preferences do not yet exist, resolution
canonicalizes the parent but retains the selected basename's case. Rust's lexical
Path equality misses settings.json versus SETTINGS.JSON on Windows, allowing
the ZIP exporter to create an archive at the future active-preference path.
This is not a proven overwrite of an existing settings file or ZIP interoperability.

Reuse existing filesystem resolution,validated serialization,ZIP writer and
atomic destination replacement. Keep exact equality everywhere; on Windows,
also conservatively refuse ASCII-case-equivalent basenames in the same resolved
parent. The protected basename is the client's own ASCII settings.json. This
does not weaken limits or introduce a second persistence representation.

Windows-only real-filesystem test covers existing and absent active files,
preserves original bytes/absence and permits ordinary sibling ZIP save/load.
The temporary fixture,never the owner's LOCALAPPDATA,owns every affected path.
An isolated counterfactual changes only the comparison back to lexical equality;
retain its failed test log,restore exact tracked source,then run full MSVC tests,
strict Clippy,release and20-page DX12 audit. Counterfactual failure is diagnostic,
not a qualifying attempt or final artifact; exact source/artifact fences remain.

The seven Linux archive regressions pass before this added Windows-only case.
Full Linux230/211/5,fmt,strict all-target client Clippy and governance60 pass.
Physical picker/OS clipboard/IME and original format parity remain UNKNOWN.
Dedicated B2 QA/node/ON-OFF/harness/retirement setup remains unsupplied.

## Retained prior input proof

98319fd7 is normally published with cfa72fca evidence; Linux230/211/5,strict
checks/governance60 and MSVC229/217/5+Session82/strict/release pass. Its native
helper13632 exits0,DX12 at125%DPI20pages/78rehashed captures/0invalid.
Seven production widget/mailbox history tests pass,including the initially red
Enter→history regression and12Unicode clipboard-order cases. This does not
access the OS clipboard or synthesize server chat. Exact hashes/private paths
and recovery are in UI-FIDELITY/local-input record. Changed export code must
qualify independently; no inherited current-head CI or final review/MQ claim.
