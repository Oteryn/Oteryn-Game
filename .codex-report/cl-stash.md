# cl-stash report

- Wired capability 0062 / command 79 through session, client-runtime, client-ui, and the native play loop.
- Added the `0062-stash` panel with browse/refresh, previous/next paging, stow-by-item-handle, and withdraw-selected-row actions.
- The panel renders authoritative row keys, revisions, quantities, migration flags, page state, and server refusal/receipt feedback.
- Added client UI reducer/codec tests and a shared play-harness test using real stash request/reply codecs.
- Removed `0062-stash` from the unsupported client baseline and registered its handler, keybinding, and panel.
- Tests: cargo fmt check passed; targeted clippy passed; client runtime/UI/app tests passed; architecture-check passed.
- Not on the wire: display names, icons, categories, search/filter/sort controls, and a client inventory picker for stowing. The native control accepts the existing session item handle instead.
