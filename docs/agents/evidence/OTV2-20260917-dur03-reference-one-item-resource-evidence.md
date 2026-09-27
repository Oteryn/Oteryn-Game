# DUR-03 offline one-item candidate schema and resource measurement

Classification: `NONPRODUCTION_OFFLINE_CANDIDATE_SCHEMA_MEASUREMENT`.
Task: `OTV2-20260927-dur03-one-item-audit-offline-measurement-513`.
Allocation: [#162 comment 5855106063](https://github.com/Oteryn/Oteryn-Game/issues/162#issuecomment-5855106063).
Repository/base: `Oteryn/Oteryn-Game@a822326c9cf4607100e58bbc3673748f3fa299bb`.
Bound META: `Oteryn/Oteryn@1bfb5ff98c8aa156e73669a14e083a1d464c29fb`.

The sections through "Production gaps and lifecycle" below preserve the historical
revision-1 evidence, not current worker publication authority. The separately
allocated signed revision-2 successor is documented in the final section and
the JSON's `signed_offline_candidate_v2` key. Its local writer cannot publish.

## Authority and interpretation

`PROVEN`: the protected DUR-03 §39.1 supplies the closed semantic MINT/TRANSFER
aggregate basis. This allocation adds only an unregistered candidate IDL and
private prost carriers in the existing standalone example. Neither the foundation
IDL, event registry, resource registry, Cargo files nor server runtime is modified.
The candidate has no registered event type. The offline envelope deliberately
leaves event_type_id at zero, so canonical ANL admission remains invalid.
`offline_item_unaccepted` is a fixture marker, not an accepted retention profile.
Production admission and raw-ID export always reject, even if a caller asserts
export permission. The Character bootstrap retention profile is not inherited.

The fixed Gold Coin x1 mapping key/revision (1,1), UUIDs, scope and source/cause
are synthetic evidence. They do not bind real Content, select production quantity
ceilings or establish natural Rat drop probability (`UNKNOWN/NOT_ASSERTED`).
Private before/after states express one LIVE item. The MINT variant explicitly
means absence before and LIVE Ground after; it never encodes a zero-quantity live
item as absence. TRANSFER requires equal item identity/type/quantity/lifecycle in
before/after, Ground before and direct-root CharacterInventory after.
Corpse occurrence is provenance association, not a second location authority.

Each operation has its own TransactionId, EventId, complete membership (1,1), and
atomic fixture boundary. MINT has no invented player command. TRANSFER uses the
fixture CommandRef (GameSessionId, command_id=2). Applicable session, connection
generation, RuntimeOrderRef, revisions and membership stay in the foundation
envelope; typed item/corpse/source/Character state stays in the payload.
No OperationId is fabricated. Current facts are independent fixture inputs;
immutable event references are expected bindings, never production authority.

## Actual retained measurements

The retained run is Linux x86_64. These are actual executable target sizes with
padding; vector/string capacity is charged separately from inline carriers.
They exclude allocator metadata, transient decode allocations, peak RSS, SQL
storage and production capacity guarantees.

| Measurement | MINT | TRANSFER |
|---|---:|---:|
| Candidate payload encoded bytes | 155 | 223 |
| Full foundation envelope including payload/hash | 346 | 462 |
| Offline one-event aggregate count / bytes | 1 / 346 | 1 / 462 |
| Frozen carrier inline bytes | 576 | 576 |
| Payload vector capacity bytes | 155 | 223 |
| Envelope wire vector capacity bytes | 346 | 462 |
| All retained event vectors/strings capacity bytes | 643 | 859 |
| Frozen inline + retained dynamic bytes | 1219 | 1435 |
| Prototype participant records / retained / encoded | 1 / 44 / 44 | 1 / 44 / 44 |
| Prototype custody effects / retained / encoded | 1 / 72 / 58 | 2 / 144 / 94 |
| Prototype participant + effect encoded bytes | 102 | 138 |
| Fixed prototype Plan inline bytes | 344 | 344 |
| Reconciliation record inline bytes, including event carrier | 936 | 936 |
| Planning / application work units | 2 / 2 | 3 / 3 |

The final fixed Model is 1968 inline bytes. Its two event records retain 1502
additional dynamic bytes, giving 3470 inline-plus-dynamic bytes without double
counting the embedded carriers. The reconciliation carrier total is 1872 inline
bytes. This differs from the earlier prototype-only 360-byte record because it
now contains the 576-byte FrozenEvent carrier. The prior participant/effect
measurements and synthetic probe remain separate. Work units remain abstract
record visits, excluding codec, allocation, lookup implementation and wall time.

Payload SHA-256 values:

- MINT: `d92805b6ed5196cb0f41844f9270b96f1730cda8b5b0659c818a0419fa1b2947`.
- TRANSFER: `139865089c9450ade078899260f69f620ca17e3969844906f288eee2729665ef`.

The JSON is the program's complete normalized output, including exact payload and
envelope hex. It measures the retained frozen record, without regenerating the
event from mutable domain state. Its output is 11865 bytes including the final LF;
SHA-256 `d1ea0f574542fc677d6d5f60170743b4e758594ad5e1fc69e6ba8aad412f3bdd`.
Two independent executions and reversed presentation order are byte-equal.
Execution still respects MINT before TRANSFER; only fixture presentation reverses.

## Independent literal wire oracle

The four literal goldens embedded in focused tests were hand-derived with the
following field/tag/wire/varint/length table. The encoder under test did not
generate them. Independent PowerShell/.NET SHA-256 over those literal payload
octets supplied the hash literals in the full-envelope goldens. No cross-language
producer, protoc conformance or fuzz qualification is claimed.

For every tag, key = (tag << 3) | wire. Wire 0 is unsigned varint; wire 2 is
length-delimited. Identity bytes for fixture tag T are
`000000000001700080000000000000TT` (TT in hex).
Varints: 1=`01`, 2=`02`, 7=`07`, 100=`64`, 150=`96 01`,
155=`9b 01`, 218=`da 01`, 223=`df 01`.
Strings are literal ASCII/UTF-8. Scalar zero direct_root_slot is omitted by proto3.
The fixed fixture serialization lists fields in ascending tag order; ANL does
not require canonical protobuf ordering for general semantic equality.

| Payload message | Fields: tag/wire/key, value or nested byte length | Message bytes |
|---|---|---:|
| CandidateItemState | 1/2/0a ID03[16]; 2/2/12 World01[16]; 3/0/18 key1; 4/0/20 revision1; 5/0/28 quantity1; 6/0/30 LIVE1 | 44 |
| CandidateGround | 1/2/0a World01[16]; 2/2/12 Channel06[16]; 3/2/1a corpse0c[16]; 4/0/20 revision1; 5/0/28 x100; 6/0/30 y100; 7/0/38 z7 | 62 |
| CandidateInventory | 1/2/0a Character02[16]; 2/0/10 session_generation1; tag3 zero omitted | 20 |
| CandidateProvenance | 1/0/08 corpse-loot1; 2/2/12 occurrence0c[16]; 3/2/1a cause0a for MINT /0b for TRANSFER[16] | 38 |
| OneItemMint | 1/2/0a state[44]; 2/2/12 Ground[62]; 3/2/1a provenance[38] | 150 |
| OneItemTransfer | 1/2/0a before[44]; 2/2/12 after[44]; 3/2/1a Ground[62]; 4/2/22 Inventory[20]; 5/2/2a provenance[38] | 218 |
| Candidate root MINT | 1/0/08 revision1; 2/2/12 Mint[length96 01] | 155 |
| Candidate root TRANSFER | 1/0/08 revision1; 3/2/1a Transfer[lengthda 01] | 223 |

Each 16-byte identity field contributes 18 bytes. Each small scalar pair contributes
2. Each small nested wrapper contributes 2, except the root wrappers for 150/218
which contribute 3. Thus MINT = 2 + 3 + (46+64+40) = 155; TRANSFER =
2 + 3 + (46+46+64+22+40) = 223.

| Envelope field | Tag / wire / key | Literal value / nested length |
|---|---|---|
| envelope_revision | 1 / 0 / 08 | 1 |
| event_id | 2 / 2 / 12 | ID08 MINT / ID09 TRANSFER[16] |
| event_type_id | 3 / 0 / 18 | zero omitted; unregistered and invalid for production |
| event_schema_revision | 4 / 0 / 20 | 1 candidate-local |
| durability_class | 5 / 0 / 28 | 2 DURABLE_AUDIT |
| privacy_class | 6 / 0 / 30 | 3 RESTRICTED_PLAYER_LINKED |
| retention_profile_id | 7 / 2 / 3a | offline_item_unaccepted[23] |
| occurred_at_unix_ms | 8 / 0 / 40 | 1 synthetic timestamp |
| world_id | 9 / 2 / 4a | World01[16] |
| channel_id | 10 / 2 / 52 | Channel06[16] |
| instance_id / node_id | 11 / 2 / 5a; 12 / 2 / 62 | absent |
| game_session_id | 13 / 2 / 6a | absent MINT; Session07[16] TRANSFER |
| connection_generation | 14 / 0 / 70 | absent MINT; 1 TRANSFER |
| runtime_order | 15 / 2 / 7a | [4]: 08 01 10 01 MINT /08 01 10 02 TRANSFER |
| command_id | 16 / 0 / 80 01 | absent MINT; 2 TRANSFER, after tag15 |
| operation_id | 17 / 2 / 8a 01 | absent |
| transaction_event | 18 / 2 / 92 01 | [22]: 0a 10 ID04/05; 10 01; 18 01 |
| correlation_id | 19 / 2 / 9a 01 | absent |
| causation | 20 / 2 / a2 01 | absent MINT; [22] TRANSFER = 12 14 + Command[20] |
| analytics_actor | 21 / 2 / aa 01 | absent |
| protocol_major | 22 / 0 / b0 01 | absent; gameplay protocol not selected here |
| ruleset_revision | 23 / 2 / ba 01 | fixture1[8] |
| content_revision | 24 / 2 / c2 01 | fixture1[8] |
| server_build_id | 25 / 2 / ca 01 | offline[7] |
| payload | 26 / 2 / d2 01 | length9b 01[155] MINT /df 01[223] TRANSFER |
| payload_sha256 | 27 / 2 / da 01 | [32] literal independently computed digest above |

Command[20] = 0a 10 Session07 + 10 02. Causation[22] = 12 14 Command[20].
MINT envelope overhead is 191 bytes, TRANSFER overhead 239 bytes. The latter adds
18 session bytes +2 generation +3 command +25 causation =48.
Thus 155+191=346 and 223+239=462. No framing or array wrapper is added to the
one-event aggregate; it is exactly this single full envelope.

## Bounds, immutability and adversarial evidence

`PROVEN`: accepted shared ANL limits are conjunctive: payload 196608, envelope
262144, each envelope string 128 UTF-8 bytes and message depth 32. Focused tests
match these constants to the untouched resource registry. The raw envelope length
is checked before decode/retention; borrowed preflight bounds nested lengths,
string/identity/hash widths, validates varints/wire keys and recursively visits
only known schema messages without allocating. It counts the payload as an
envelope child. Prost 0.14.4 retains its upstream recursion ceiling of 100;
the private preflight enforces 32 and rejects all protobuf groups before prost.
No dependency fork or generic serialization framework is introduced.

Known singular duplicates, simultaneous oneof variants, unsupported revisions or
closed shapes reject. Safe additive unknown numeric fields are skipped; unknown
opaque byte fields reject because their identity/privacy meaning is unresolved.
Exact payload bytes remain immutable even when decoding discards safe unknown
fields. Reordered outer-envelope wire may be semantically identical; changed
exact payload bytes/hash under the same EventId remains a conflict.

Encoding checks the finite shape before copying private fields or calling
prost encoded_len/encode. The schema-derived conservative payload bound is
64*13+13*16=1040 bytes. The offline retained reservation includes two such bounds,
1024 envelope overhead, at most 174 identity/string side-buffer bytes and the
inline carrier. Checked add/multiply and try_reserve_exact guard allocation.
Those conservative fixture bounds are neither measurements nor production maxima.
The encoder's exact encoded_len is checked inside that pre-reserved shape.
Borrowed decode enforces the shared limits before prost or wire.to_vec.

Exact generic max/max+1 comparisons cover all four shared limits; real finite
fixtures cover coupled byte/semantic limits. The known-child preflight accepts
depth32 and rejects depth33. A 128-byte string passes the borrowed length check,
129 rejects; closed fixture/profile validation remains independent. Oversized
raw envelope/payload rejection is exercised. No IDL filler fields fabricate a
max-sized production event.

`PROVEN`: 30 focused test functions include the original 16 prototype tests plus
14 candidate tests. They cover every candidate/foundation private field tag,
literal MINT/TRANSFER payload+envelope goldens and round-trip, reordered fields,
safe additive unknowns and exact-payload conflict, every truncated envelope
prefix, bad varint/wire/key/length/group/duplicate/oneof/revision, missing/nil and
mismatched world/channel/corpse/Character/session/definition/generation facts,
MINT missing state/source and zero/live lifecycle, transfer quantity/type/identity
conservation, membership gap/count/order/duplicates, identity-content conflicts,
privacy downgrade/missing profile/secret and raw-ID export rejection.

They also cover lost responses, noncommit reconciliation and known-abort retry,
exact EventId/semantic envelope/membership/payload/hash/outer-wire preservation,
corrupted frozen carriers rejecting before retry/reconciliation mutation,
stale current fixture facts rejecting after known abort, retry exhaustion,
read-only replay and partial-transfer rejection before custody publication.
Original prototype dimensions still accept exact injected usage and reject
max+1 before publication, including overflow and synthetic audit count/bytes.
The FrozenEvent is installed before possible ambiguity and is never regenerated
for retry/reconciliation; replay receives only an immutable model borrow.

## Validation environment and reproduction

`PROVEN`: authoring checks ran from the exact protected base plus this bounded
five-path delta in /home/mole/dur03-offline-a822-513, WSL Ubuntu Linux x86_64.
Observed tools: rustc 1.94.0 (4a4ef493e 2026-03-02),
host x86_64-unknown-linux-gnu, LLVM 21.1.8;
cargo 1.94.0 (85eff7c80 2026-01-15), prost 0.14.4, sha2 0.10.9.
The toolchain file selects 1.94.0. The executable still reports
build_toolchain_identity=UNKNOWN: it has no build-time compiler attestation and
does not infer build identity from a compiler found at runtime.

The validation tar was created with git -c core.autocrlf=false archive at the
exact base. Initial native archive conversion produced CRLF scratch files and
a failed full rustfmt check; the corrected LF scratch passed without changing
unallocated files in the source worktree. The existing Cargo target cache at
/home/mole/dur03-513-p2.9unFjx/repo/target supplied incremental dependencies;
Cargo rebuilt this example and consuming source from the new exact LF scratch.

```sh
cargo fmt --all -- --check
cargo test --offline --locked -p oteryn-game-server --example dur03_reference_one_item_resource_prototype --no-default-features
cargo build --offline --locked -p oteryn-game-server --example dur03_reference_one_item_resource_prototype --no-default-features
cargo clippy --offline --locked -p oteryn-game-server --example dur03_reference_one_item_resource_prototype --no-default-features --no-deps -- -D warnings
target/debug/examples/dur03_reference_one_item_resource_prototype > run1.json
target/debug/examples/dur03_reference_one_item_resource_prototype > run2.json
target/debug/examples/dur03_reference_one_item_resource_prototype --reverse-fixtures > reversed.json
cmp run1.json run2.json
cmp run1.json reversed.json
cmp run1.json docs/agents/evidence/OTV2-20260917-dur03-reference-one-item-resource-evidence.json
sha256sum run1.json
python3 tools/agents/validate_governance.py
python3 tools/repository/validate_repository_policy.py
cargo run --offline --locked -p oteryn-architecture-check -- workspace .
git diff --check
```

Checks PASS: 30 tests, locked focused build, strict example Clippy, full-workspace
rustfmt, governance (22 required policy documents /9 lanes), repository policy
(23 files /43 workflows plus exact-candidate routing regressions), workspace
boundaries, five-path diff and normalized retained output equality.
Existing vendored Tokio missing_docs warning at sync/oneshot.rs:502 remains;
no new source warning or suppression was added.

Native Windows x86_64-pc-windows-msvc tools are also 1.94.0. Native governance
passes. The focused server example cannot build on that target because inherited
server source uses std::os::fd, UnixListener/UnixStream and Linux-only rustix;
the attempt produced 67 inherited errors before this example compiled.
Native repository policy encountered the CRLF LICENSE/pinned-MPL text mismatch.
These failures are recorded as host/source limitations, not successful checks;
the LF Linux validation route passes. No server/platform or license repair is
allocated. Windows client/SIM, PostgreSQL, connected gameplay E2E and hosted CI
remain control-plane qualification, not worker evidence.

## Production gaps and lifecycle

`UNKNOWN/EVIDENCE_GAP`: actual production mandatory audit event contribution
count and aggregate bytes remain null in every operation. The measured offline
count1/346 or1/462 is a candidate design measurement, not acceptance of emitted
production membership. The original synthetic probe count1/16 bytes is retained
separately and is not an event. DUR03-RL-07 retains the production gap.

No registry admission, item retention/purpose/access/export/expiry profile,
production numerical acceptance, PostgreSQL audit atomicity/isolation/restart,
runtime session fencing, real Content binding, connected Combat/pickup,
natural loot probability or Reference parity is established. No SQL migration,
foundation API, protocol, authority or runtime activation is added.
Production audit expiry cannot reopen mint eligibility or permit identity reuse.

This evidence describes the completed authoring delta before remote freeze.
The standalone worker may publish one API-native candidate on its allocated
branch and return its exact head in external handoff evidence. A commit cannot
embed its own SHA. Hosted exact-head checks, independent review, PR creation,
ready state, Merge Queue and protected integration belong to the active control
plane and remain pending. Material repair after freeze requires control-plane
disposition; this worker must not move the frozen head.

## Signed offline revision-2 successor (current authoring evidence)

Allocation: [#162 comment 5856736427](https://github.com/Oteryn/Oteryn-Game/issues/162#issuecomment-5856736427),
with [custody refinement 5856742846](https://github.com/Oteryn/Oteryn-Game/issues/162#issuecomment-5856742846).
Protected source base: `56e5c8a39bf2d899cbbc24735d2d7440d4ecaec1`.
Exactly five owned paths; root is sole API publisher/control plane and this worker
authors locally only. Task: `OTV2-20260927-dur03-signed-offline-candidate-resource-evidence-513`.
No PR/frozen head is asserted by this local report.

`PROVEN`: v1 IDL, literal goldens, 155/223 payload bytes, 346/462 envelope bytes,
1040-byte historical fixture charge and all historical JSON fields are preserved.
Removing only `signed_offline_candidate_v2` restores the original JSON value.
The historical LF report was 11865 bytes with SHA-256
`d1ea0f574542fc677d6d5f60170743b4e758594ad5e1fc69e6ba8aad412f3bdd`.
The new namespace is `oteryn.events.candidate.v2`, interpretation revision 2;
foundation envelope revision 1/schema revision 2/type 0. Crossrevision rejects,
including old positive uint32 geometry that would otherwise reinterpret 100 as
sint32 50. There is no automatic migration or Tibia z-to-floor adapter.

`PROVEN`: signed x/y use `sint32`; floor also uses `sint32` with a preallocation
checked-i16 range. Independently supplied synthetic World facts bind exact WorldId
and `content_revision=synthetic_world_r2`, half-open x/y bounds `[-200,200)`, and
sorted unique floor set `[-1,0,7]`. A malformed profile rejects: empty/unsorted/
duplicate floors, empty/reversed intervals, or bounds beyond i32 endpoints.
Separate positive tests supply the full synthetic signed domain in i64 intervals
`[i32::MIN,i32::MAX+1)` and floors `[i16::MIN,0,i16::MAX]`. Those are synthetic
representation tests, not product World activation or numerical gameplay policy.
The typed root is closed `SyntheticDirectRoot=1` (0/unknown reject), **not** the
old revision-1 slot 0, Tibia slots 1..11, or a u16/u32 product slot ceiling.

The v2 fixture has its own two-record custody/retry model. It retains signed
Ground `[100,-100,7]`, compares the complete retained MINT.after/Ground and
synthetic World revision with TRANSFER.before/source, and then moves
the same fixed synthetic live item (WorldId/item identity/key1/revision1/qty1)
to the synthetic direct root. A World-admitted but different source coordinate
rejects without publication. This is not an audit sidecar on the old +100-y model.
Different v2 EventIds/TransactionIds separate it from historical v1 goldens.

### Independent finite byte oracle

`DERIVED/PROVEN_BY_WIDTH_WITNESS`: an evidence-backed conservative canonical
serialized representation superset is sufficient for bounded byte safety;
it need not be a tight reachable admitted maximum. Private key/revision/quantity
u32 extremes are only width witnesses, not accepted Content semantics. Optional
Envelope fields may be mutually incompatible; negative timestamp and maximal
scalar values may be semantically invalid. Width witnesses are explicitly
labelled `semantically_admitted=false` and never published by the fixture model.

| Component (including its own field keys) | Width upper, bytes |
|---|---:|
| ItemState: two IDs + three u32 fields + lifecycle=1 | 36+18+2 = 56 |
| Ground: three IDs + u64 corpse revision + two zigzag i32 + zigzag i16 | 54+11+12+4 = 81 |
| Inventory: CharacterId + u64 generation + closed enum=1 | 18+11+2 = 31 |
| Provenance: source kind=1 + two IDs | 2+36 = 38 |
| MINT nested body / revision-2 payload | 58+83+40 = 181 / 186 |
| TRANSFER nested body / revision-2 payload | 58+58+83+33+40 = 272 / 277 |
| All-field Envelope non-payload overhead | 1004 |
| MINT full Envelope = complete one-event aggregate | 1004+2+2+186 = 1194 |
| TRANSFER full Envelope = complete one-event aggregate | 1004+2+2+277 = 1285 |

The independent envelope non-payload per-tag charges are:
`1:2, 2:18, 3:0(omitted), 4:2, 5:2, 6:2, 7:131, 8:11,
9:18, 10:18, 11:18, 12:18, 13:18, 14:11, 15:24, 16:12,
17:19, 18:25, 19:19, 20:34, 21:164, 22:7, 23:132, 24:132,
25:132, 27:35 = 1004`. Payload tag 26 has a two-byte key and two-byte length
prefix for both v2 bounds. Header constants are pinned candidate constants,
not full-width u32 gameplay values. UUIDs are exactly 16 bytes, SHA-256 exactly
32, string byte maximum 128, RuntimeOrder two u64, and complete membership
one TransactionId plus fixed ordinal=count=1. CommandRef is the largest allowed
causation alternative; actor identity domain is also charged to 128 bytes.
An independent read-only oracle hand-encoded the valid signed payload goldens
(157/227 bytes) and independently verified these component/aggregate formulas.
The code compares literal hex, not a second call to the same encoder.

### Admission and allocation boundary

`PROVEN`: the v2 borrowed preflight closes **both** the entire Envelope and
payload grammar before prost decoding or input-sized owned copies. Unknown,
duplicate, out-of-order and wrong-wire-type fields, multiple oneof members,
overlong/nonminimal/truncated/overflowing key/length/scalar varints, u32 overwidth,
floor zigzag >65535, wrong ID/hash widths, non-ASCII/empty/>128-byte text,
missing required members and unsupported enum/revisions reject. Explicit
event_type_id tag3=0 also rejects: the omitted zero header cannot add uncharged
raw bytes. Canonical default omission and field ordering are candidate-local
restrictions, not a claim about general ANL additive compatibility. V1's existing
reordered/additive-unknown compatibility tests remain unchanged and passing.
The root schema has finite nesting (Envelope→Payload→operation→leaf), no recursive
containers; checked arithmetic guards all sums, lengths, reservations and retry
increments. The global ANL 196608/262144 caps are not promoted to game limits.

Actual valid fixture results on Linux x86_64:

| Measurement | MINT | TRANSFER |
|---|---:|---:|
| Payload / Envelope / complete one-member aggregate bytes | 157 / 358 / 358 | 227 / 476 / 476 |
| Frozen carrier inline bytes | 576 | 576 |
| Owned semantic payload capacity | 157 | 227 |
| Owned full wire capacity (contains another payload copy) | 358 | 476 |
| All retained vector/string capacities | 667 | 887 |
| Frozen carrier inline + dynamic capacity | 1243 | 1463 |
| Conservative dynamic capacity reservation | 2228 | 2410 |

All dynamic side buffers are accounted, including optional UUIDs, membership,
each causation alternative, actor ID/domain, retention/build/ruleset/content
strings. A string's actual capacity is charged, not just byte length.
Conservative reservation adds payload separately to full wire, 208 bytes of
ID/hash side vectors and five 128-byte strings (848 bytes total side buffers).
Model inline bytes are 1200 including both spare record slots; actual dynamic
1554; actual total 2754; injected conservative two-record reservation 5838.
These are retained carrier/capacity facts only: no allocator metadata, transient
decode/proposed-event copies, peak RSS, SQL, persistence or production capacity
claim. Proposed frozen-carrier capacities are checked too: byte equality alone
cannot admit an enlarged backing allocation. Private prost buffers are created only after facts and the complete
finite injected reservation pass; publication is after exact validation/budget.

Retry/reconcile work counts three **logical** units (inspect/compare/disposition)
per retry transition; the injected per-record budget is six. Derived shape units
count one participant plus custody effects, or effects plus publication receipt
(MINT 2, TRANSFER 3). They are not measured CPU instructions or latency and exclude
parsing, hashing, allocation, text scans and World-profile iteration. The actual
profile has three floors; no production retry horizon or CPU ceiling is chosen.
Known noncommit retains the exact original bytes and permits only the same
candidate retry. Lost commit responses hold immutable EventId/payload/hash/full
Envelope until reconciliation; unknown observations cannot remint or resurrect
Ground. Conflicting payload under the same transaction rejects. Budget, wrong
commit observation and checked-overflow failures leave custody/records unchanged.

### Dispositions and qualification boundary

The JSON gives explicit DUR03-RL-01..08 dispositions for this successor:
one item; one MINT establish/two TRANSFER effects; no account/transform/container
surface; physically retained two records; complete one-member audit aggregate
with finite candidate byte upper; immutable in-memory retry only. Production
RL-07 remains `EVIDENCE_GAP`, as do restart/SQL/current live fencing, real Content
stack/quantity/direct-root legality, event/profile/resource registration, playable
Combat/pickup and Reference parity. Candidate representation improvements do not
turn those unknowns into owner-selected numeric limits.

Authoring qualification uses offline locked no-default-features example tests,
build, strict Clippy, format, governance, repository policy and architecture
boundaries on a clean LF Linux execution copy. Generation runs twice and in
reverse presentation order and compares exact LF bytes. Current commands and
review state are in the new task record. The inherited vendored Tokio missing-doc
warning is not a new candidate warning. Exact remote freeze, candidate-specific
CI/independent whole-diff review and DRAFT PR remain parent control-plane steps;
no ready/review-trigger/enqueue/merge is authorized to this writer.
