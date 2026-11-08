# Native authored binding admission

Status: qualified by 326 quest unit tests and a full seven-case real-dispatch
PostgreSQL confirmation run on `c7f9d9ba7` (main `98d6ae8ce`).

Native boot registers regular JSON documents from `content/quests/authored-bindings/`.
The build script generates the file registry in `OUT_DIR`; lanes add their own
quest files without editing a shared list. `Bindings::parse_documents` combines
a complete bounded snapshot and rejects duplicate owners or unknown fields.

The native quest loader installs canonical recipe evidence after the retained
quest owners have lowered their definitions. `Bindings::attach` then requires an
exact canonical chosen recipe digest, complete recipe data, and no unresolved
data holds. Only the two implementation-only missing-binding codes are allowed.
Shard membership, schema, identity uniqueness, and population must match the
compiled Quest index. Catalogue extension preserves this evidence.

The default entrypoint remains `loader::load_embedded_quest_state`. The explicit
snapshot entrypoint is `loader::load_embedded_quest_state_with_bindings`; it uses
the same evidence and owners while replacing the compiled binding snapshot.
`Bindings::verify_embedded_recipe_evidence` lets authoring callers check the same
canonical evidence directly. Recipe hashes use sorted compact UTF-8 JSON.

Bare catalogues retain the low-level binding API used by synthetic engine tests.
They do not claim native boot admission. An empty native binding set preserves
legacy tracks, transitions, definition hashes, and NPC predecessor information.
No canonical readiness flag changes automatically, and no synthetic document is
registered in production by this feature. Whole-quest qualification remains the
owning lane's responsibility.

The real-dispatch PostgreSQL regression is
`authored_native_boot_recipe_admission_dispatch_postgres`. Its original RED ran
52.000s with a 51.942s PostgreSQL completion marker and reported: `native boot
accepted a stale authored recipe digest`. The implementation must refuse that
digest and deliver the correctly bound reward through the ordinary USE owner.

The seven-case confirmation passed in 386.448s. Native admission itself passed
with a 59.105s PostgreSQL completion marker, after an earlier independent pass
of 67.376s. The first broader run had one intermittent catalogue-drain timeout
in the retained sequential-kill fixture; the complete set then passed unchanged.
No timeout or test assertion was weakened, and no fix to that intermittent drain
failure is claimed.
