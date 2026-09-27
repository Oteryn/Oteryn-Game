# DUR-03 native one-item audit schema/codec evidence

Classification: **UNREGISTERED/OFFLINE candidate evidence**. This packet measures
one closed representation of the protected DUR-03 section 39.3 native bindings.
It does not register an event, admit Content or gameplay policy, authorize a
runtime write, or prove PostgreSQL durability, restart recovery, client
playability, Reference parity, or Canary/Crystal parity.

## Bound candidate

- Protected source base: `e422fc9d2962f63ccb6f0af9ee80ec5f449c2a31`
- Coordinator allocation: issue #162 comment `5859343636`
- Candidate schema: `docs/contracts/game-events/v1/native_one_item_transaction.proto`
- Executable measurement: `apps/game-server/examples/dur03_native_one_item_audit.rs`
- Normalized JSON: 16,643 LF bytes, SHA-256
  `1558b339151612ecaa3a334a2863ffa7c54fc77c249ce00ac902a324f2157656`
- Rust source: 70,285 LF bytes, SHA-256
  `5da852d958799925acea1dd2fbed7bc781851ac7d8e7e12be04c416b540cb7a5`
- Candidate schema: 3,473 LF bytes, SHA-256
  `165beae9c29c3405002adf4d6cd2e6cf1cbdff752451c806162643f45eece983`
- Execution target: Linux x86_64. Compiler/build identity remains
  `UNKNOWN` because the standalone example embeds no build-time attestation.

## Observed deterministic fixtures

Both definitions use the same generic codec and validation path. Names and
revisions are synthetic typed-definition fixtures, not admitted product rows.

| fixture | operation | payload bytes | envelope bytes | payload SHA-256 | envelope SHA-256 | retained dynamic capacity | raw clone bytes | transient construction/decode/canonical-re-encode dynamic capacity |
|---|---:|---:|---:|---|---|---:|---:|---:|
| definition_a | MINT | 329 | 416 | `355c447ed81ccb1a67601ef9ffdad51ee51c84176447869aee99097bcfe96210` | `f81957f4ca74958584be5d6f106daa66e298bd8f7e1b030f3b397e7df3cb9100` | 1,838 | 745 | 4,383 |
| definition_a | TRANSFER | 477 | 564 | `a54f63bf00f23b67ba2e92e7337a24f8ce018f75c4dad3173549d1f3bc8e5267` | `e7fe8ec7987e4dcb0de6888f6a27c025bc13b4ad283cfd3df91fca5491053298` | 2,551 | 1,041 | 6,199 |
| definition_b | MINT | 328 | 415 | `6429db7ec84e939a8f8e3a02a4953d95d7b64ecde00beded7af8889200de87c4` | `66acffe2f98474871b9ef2c43ab8c56d10ef11243f0ce8a8d0ee24b7fa41e6ca` | 1,833 | 743 | 4,370 |
| definition_b | TRANSFER | 475 | 562 | `e13b32de2d0779f44cf99ce6f455ea15d5286ff2ee40265c592f0773fe038419` | `81b6f3a989d7688e375ed4c78ab6295143096d6c49937a53c9c0a532f9fad99a` | 2,541 | 1,037 | 6,173 |

The JSON retains the complete literal payload/envelope vectors and records all
eight bounded resource dimensions: payload bytes, envelope bytes, raw clone
bytes, transient construction/decode/canonical-re-encode dynamic capacity,
hash input, decode input, retained dynamic capacity, and retry work. It also records cumulative
encode work, participant/effect counts, and retry outcomes. Each MINT and
TRANSFER is a separate one-member aggregate with `ordinal=1,count=1`.

## Executed validation

The exact executable candidate passed 14 focused tests under WSL with:

`cargo test --offline --locked -p oteryn-game-server --example dur03_native_one_item_audit --no-default-features`

The suite covers both-operation literal payload and full-envelope goldens,
canonical decode/hash/re-encode, unknown/duplicate/nonminimal wire rejection,
two generic typed definitions, explicit pre-MINT semantic absence, exact native
Ground/inventory positions, semantic mutation of actual decoded fields,
independently supplied current facts, exact EventId/TransactionId/membership
receipt binding, lost acknowledgement, ambiguous hold, proven-noncommit retry,
idempotent committed replay, distinct MINT/TRANSFER identities in one shared
receipt ledger, independent TRANSFER source-item binding, every measured
resource dimension at max and max+1, checked overflow, inherited ANL ceilings
and candidate schema drift.

`rustfmt --check` passed. The only build warning is the inherited missing-doc
warning in vendored Tokio and is outside all allocated paths.

Three independently executed renderings were byte-identical to the checked-in
JSON:

- normal fixture order;
- `--reverse-fixtures`;
- absolute binary with `PATH=/nonexistent`.

All three and the checked-in file hash to
`1558b339151612ecaa3a334a2863ffa7c54fc77c249ce00ac902a324f2157656`.

## Limits and dispositions

The accepted ANL ceilings apply conjunctively: 262,144 encoded envelope bytes,
196,608 payload bytes, 128 UTF-8 bytes per envelope string, and protobuf nesting
depth 32. The private 4,096-byte candidate wire cap is deliberately lower and
has no product authority. The candidate schema nesting depth is five.

The candidate binds expected typed definition/revision, item state, World and
Channel, map/content/native-room/typed-position/runtime generation, committed
death and distinct loot-output occurrence plus loot/content/ruleset/SIM
revisions. TRANSFER additionally binds Character, typed position,
GameSession/session generation, CharacterLease generation, and CommandRef.
These immutable expected bindings never manufacture current authority.

TRANSFER remains production-closed on legal CharacterInventory
position/capacity and the migration-0009 global CharacterRevision/XP-receipt
composition conflict. MINT remains unimplemented: B2 supplies no legal Content
row, loot probability/quantity, source eligibility, event/profile/resource
registration, SQL receipt, current runtime fence, or production permission.

R7 owns separate XP/reference evidence paths. This packet changes no R7,
Character, Content, registry, SQL, room, Movement, Server Seam, protocol, client,
Cargo, migration, workflow, or runtime path.
