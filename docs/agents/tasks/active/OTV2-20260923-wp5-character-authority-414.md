# OTV2-20260923-wp5-character-authority-414

- Status: ACTIVE
- Coordinator: #162
- Source programme: #319
- Branch: `agent/wp5-character-authority-414`
- Admission main: `ec8803bd1a37600acd0ff544811871e8fb2c40cf`
- Scope: first authoritative Character bootstrap/current read with atomic receipt, registered durable audit payload and outbox.
- Excluded: transfer, world transfer, rename, retirement, Platform writes, Foundation composition and Server Seam.

## Owned paths

The exact paths are those in Work application #162 comment `5793512204`, including the bounded three-file prost dependency lease recorded by #247 comment `5793517381`.

## Qualification

Focused Rust/protobuf checks run locally. The registered `character_authority_postgres` target and whole candidate require PostgreSQL 17.6 and canonical repository CI before integration. Independent exact-head review, Ready, Merge Queue and closeout remain coordinator-owned.
