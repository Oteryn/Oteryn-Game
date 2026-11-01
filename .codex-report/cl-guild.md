# cl-guild

- Files changed: session re-export; client-runtime guild transport; client-ui guild state/actions/rendering and registrations; native-client play adapter, mailbox, command loop, Windows panel; unsupported baseline.
- Tests added: client-ui refusal/action unit test and native play harness coverage with real guild request/reply codecs.
- Results: package tests, clippy, cargo formatting, feature formatting, and architecture-check tests pass (the repository-wide feature glob reports pre-existing formatting differences in capability 148).
- Not on the wire: guild description/title/rank-name editing, disbanding, and invitation target character IDs. They were intentionally not invented.
