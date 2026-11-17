# WORLD-SRC continuation ownership and integration

Historical1fa04bbb continuation; the later requested srvpkg3 rebase is recorded
in `srvpkg3-rebase-validation.md`.

Base: `1fa04bbbddb5d96e21fecca0bc456f3c5e1109b7`, fetched from the requested
GitHub repository. The received c8b3890e patch was preserved before reapplication
(SHA-256 `2228d16b14155816479eaf15f5d85511b38aae9407e3086a5dcd2e3cfcb07fcb`).
No commit or push is part of this continuation.

## Open package exclusion

The following branches were fetched and their changed paths computed against each
branch's merge base with the requested main. Their union is 402 protected paths.

| Package suffix | Fetched SHA | Merge base |
| --- | --- | --- |
| srvpkg3 | `faf8ae8700d935bca612261730aca5466526389a` | `1fa04bbbddb5d96e21fecca0bc456f3c5e1109b7` |
| clpkg6 | `ab31f9dbd89a1ef78dd9639431acf595edf212db` | `1fa04bbbddb5d96e21fecca0bc456f3c5e1109b7` |
| srxpkg | `07cfb2ea1d93f089e617f9c0670a41dd8b354bc7` | `1fa04bbbddb5d96e21fecca0bc456f3c5e1109b7` |
| platwire1 | `c5f5ff8eaaedf05161287e8dbf4c833e34e1bf8b` | `e91bce557728e52f5d61c38f8fad7dba3c1ee1d3` |

A later read-only ref check found srvpkg3/clpkg6 advanced; their fetched heads
above were used to widen the exclusion set. The final srvpkg3 refresh added the
NPC dispatch and conversation owners; the experimentally passing Ruprecht integration
was removed from the production diff to preserve this boundary. Its source findings
remain recorded; it is STOPPED pending integration in the owning branch. The srxpkg/platwire1 refs were no
longer advertised, so their successfully fetched original heads remain protected.
The explicit requested base stays 1fa04bbb even though remote main later advanced.

All main client 0057 handlers, mailbox/session-loop arms and capability admission
are retained. Existing main commands 72/73 and capability 57 provide map discovery
and donation. There is no duplicated handler or new command/capability allocation.
Migration 0963 was absent on main and all four package branches; it remains the
only new migration. No already merged migration is edited.

## Preserved main behavior

Main's SP02 fungus, Yasir and Grimvale map projection and spawn admission remain
in the normal boot path. Compiler-admitted Fury/Nightmare geometry is composed
first; the existing daily projection receives that palette and retains the layer
activation, epoch and return metadata. With no selected layer the existing
streaming projection remains available. Epoch zero receives no daily grants.

Main's donation carry and donations-during-boost behavior remains in the canonical
main-area conversion. Existing receipt and bank invariants are retained alongside
the new progress-preserving conversion test.

The v7 optional wire `layers` envelope is retained without extending public Rust
`Manifest` struct literals: `decode_manifest` returns the manifest and layers, and
`write_with_layers` writes them explicitly. This preserves unrelated main callers
and protected spell-tile fixtures. Empty layers leave existing manifest bytes
unchanged. The strict envelope rejects duplicate/unknown keys, including nested
records, and enforces the existing size bound.

The existing quest callback owner invokes the daily gate leaf only when no quest
callback matches. Ambiguous callback admission remains refused. The admitted
quest callback catalogue contains no overlapping daily gate action/unique IDs.
This avoids editing the protected dispatch root and movement owner.

Final commands, source stops, generated hashes and exact ownership checks are
recorded in `continuation-validation.md`. Historical `validation.md` describes the
received patch on its old base, not verification of this continuation.
