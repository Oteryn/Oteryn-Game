# Closed Item packet replay after the main reconciliation

The Item CI sweep found one context family behind 31 failing commands: two
World index files changed serialization, six Terrain shards gained the accepted
MAP-KIND-CLASS-1 kinds, and ITEM-SEM-USE-1 extended the Rust Item semantics file.
The closed Item producers commit whole input-file hashes, so rebuilding their
historical packets directly against the newer tree changes their bytes or fails
their original source pins. These changes do not require rewriting Wiki facts,
Item promotions, or their immutable qualification evidence.

`retained-inputs.zip` contains exactly nine original Git blobs. `context.json`
records the original SHA-256 and Git blob identity, the separately checked
current hashes, and the complete historical/current World input inventories.
The context and archive are checksum-bound by the explicit replay helper.

The helper verifies the actual current World inventory and Rust source witness
before constructing a disposable historical byte view. It independently proves
that the view preserves every World-to-Item pointer, including its record
identity and file. Current Item definitions, import/source identity bindings,
World declarations and authoring owners remain live inputs. Existing source,
name/conflict and exact sealed 411/700 owner-closure checks are unchanged. The
current Rust modifier enum also matches all 37 historical entries. The retained
Rust file is a compiler input witness; it is not built or granted current runtime
authority. The normal current-runtime Rust gates remain separate.

Only historical Item qualification commands use the explicit wrapper. Schema,
raw-source, migration, World, runtime and integration checks retain their current
input selection. Writable Item schema/catalog/template files in the view are
detached copies. The product World data, original promotion packets, producer
implementations and their pins remain unchanged. A future input change fails
closed and needs a newly qualified context rather than silent history rewriting.

The boundary tests cover the actual positive view, writable-output isolation,
current World substitution, current runtime-source substitution, and corrupted
archive/context rejection. Execution receipts and the complete failure/repair
sweep are recorded alongside this context after qualification.
