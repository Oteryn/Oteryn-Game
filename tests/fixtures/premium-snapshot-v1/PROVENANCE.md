# Premium snapshot v1 fixtures: provenance

These files are an unmodified copy of Platform's shared fixtures for `oteryn.premium_snapshot.v1`.
The producer owns them. Game uses them as producer truth in the cross-repository end-to-end test
(PREMIUM-DELIVERY-0 §3.1 and §12).

- Source repository: `Oteryn/Oteryn-Platform` (public)
- Source path: `docs/contracts/fixtures/premium-snapshot-v1/`
- Pinned commit: `71bbe6c5cffc29d430195fa286b3c6941af4abb8` (Platform `main`, 2026-10-04)
- Contract: `docs/contracts/OTERYN_V2_PREMIUM_TIME_SNAPSHOT_CONTRACT.md` at that commit (PREM-P,
  Platform PRs #1432 and #1433, Issue #1431)
- Copied by: task `PREM-E2E-1`, 2026-10-04

| File | SHA-256 |
|---|---|
| `manifest.json` | `37d314e9a387e1e349ff56f2107b2a70f72c1984542f15ba273f8cfc602481bb` |
| `request.schema.json` | `97a2d6cb3a6c7d4aead4e7c6296ff11ff2c74d2d13ec0e7fdf233fd04af1a097` |
| `request.valid.json` | `dadd057322450b5a823bc4562b81cb581c5fdd6b7c9d815dee279bd22b6168c5` |
| `snapshot.active-clipped.json` | `f8515cb5cfab31e2a16729aae654071dbf1b7c85371ade2c057711865539c73c` |
| `snapshot.active.json` | `a7bbdad602eea24e0ff1f77acec295b2ab28cf3d631b709ac31807a80e78c8e0` |
| `snapshot.expired.json` | `b764edb6ec61d5e2709828661ead5b08ca09ec6c00046290bf8c104ef9aa3cc2` |
| `snapshot.none.json` | `d668b4300e769980d8df53bdd5e964593fd8b98c7dd40e568b77ba8a8f4408ff` |
| `snapshot.revoked.json` | `46d07bf7bd5fc21d62d65f9f59c1eefb003805b8c49b82999c32af7607b0aae8` |
| `snapshot.schema.json` | `4537250d8e2919845906df137f6b6066c65c9d958a320709dbaa013dacc794c1` |

Do not edit these files here. To update them, copy the directory again from a newer Platform
commit, update this table and the pinned commit, and rerun
`cargo test --locked -p oteryn-game-server --test premium_platform_fixtures`. The test checks
these hashes, so a local edit fails it. A wire change on Platform's side comes with a new
`schema` id and a new fixture directory (contract §7).
