# Native client UI seam

CLIENT-STRUCT-1 implements only step 1 of `CLIENT_STRUCTURE_TARGET_1.md` §5.
There is no gameplay behavior or public UI API yet.

`build.rs` shares the architecture check's fragment scanner. It scans
`src/features/*/registrations/*.json`, validates before generation, and writes
private JSON descriptors as Rust literals plus `client-mechanic-coverage.json`
to `OUT_DIR`. Generated Rust is never checked in. Executable protocol adapters,
panel factories and gameplay migrations belong to subsequent steps.

Feature names use the server runtime capability's zero-padded number and name,
with `_V<number>` removed and underscores converted to hyphens. For example,
`0057-cyclopedia-map`. `local-*` is reserved for non-server UI features. Shared
visual primitives belong to `src/components`.

Every registration file contains one object matching
`tools/architecture-check/client-fragment.schema.json`. The six kinds are
`handlers`, `panels`, `keybindings`, `settings-pages`, `cyclopedia-pages` and
`assets`; filenames are discovered rather than listed. Supported handlers own
one command each. Temporary unsupported handler fragments may enumerate a
mechanic's pending commands together. Unsupported registrations need a reason;
absent concern files need a `not_applicable` reason for each concern. They do
not require empty Rust placeholders.

Each feature keeps its sorted registration golden in
`fixtures/registrations.json`. A supported entry points to an existing
`relative-file.rs::function`. Assets have a feature-relative path and lowercase
SHA-256 digest. Panel/page parents name another registration or `root`.
Default bindings use a context, a case-insensitive `+`-separated chord and
optional integer precedence (0–255, default 0). Equal normalized chords in the
same context must have distinct explicit precedence. Different contexts are
separate scopes. Settings descriptors retain their entire `settings_envelope`,
including unknown keys and player rebinding choices; player conflicts are not
build errors.

The architecture command lists every server mechanic's status. All 62 initial
mechanics are explicitly unsupported. Removing coverage or adding an
unsupported mechanic beyond `client-unsupported-baseline.json` and
`client-unsupported-baseline.d/<mechanic>.json` (one file per newer mechanic,
content `{"mechanic": "<mechanic>"}`) fails. As a
mechanic gains support, the guard requires removing its unsupported baseline
entry in the same change, so it cannot regress. The baseline is a migration ceiling, not a
second server registry.

The size baseline records exact current physical line and byte ceilings for
existing client files above warning thresholds. Shrink or remove entries as
files split; never increase them. The purity baseline preserves only the exact
existing foundation cancellation import. Neither baseline authorizes new
exceptions.

Run `cargo test --locked -p oteryn-architecture-check --test client_layering`
and `--test client_structure`; `cargo run --locked -p oteryn-architecture-check
-- workspace .` includes both guards. The focused `client-structure` command
accepts the same repository root and reports coverage and the largest files.
