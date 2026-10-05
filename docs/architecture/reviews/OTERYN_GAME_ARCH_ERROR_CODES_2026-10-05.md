# ARCH-ERROR-CODES-0: one error code space, one registry and one diagnostic line

- Decision id: ARCH-ERROR-CODES-0.
- Status: the §1 rulings, the `FOUNDATION_ERROR_VOCABULARY.md` "Code space" amendment and the §2
  packets are accepted on merge. The owner ruled on the four items of §1.8 on 2026-10-05:
  1a, 2a, 3a and 4b. All four packets may be allocated now. The owner ruled on the four
  debugging items of §1.10 on 2026-10-05 (1a, 2a, 3a, 4a), which add packet ERR-DIAG-4. The
  owner then ruled 1b: the connection `trace` goes on the wire behind an optional capability
  (§1.10 item 6, FND-02 §18 amendment), which adds packet ERR-TRACE-5. That part needs
  protocol review.
- Origin: owner request (2026-10-05): design the error codes and the whole system around them,
  so that the owner and the agents can identify a problem quickly.
- Owning contracts: `docs/contracts/FOUNDATION_ERROR_VOCABULARY.md` (categories, progression,
  code space; amended here) and `docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json` `error_codes`
  (wire codes, FND-02 §18; unchanged here). `FND-02_PROTOCOL_OTERYN_V1_CONTRACT.md` §18 gains
  the connection trace amendment (§1.10 item 6), pending on acceptance of this decision.

## Implementation brief

1. Every failure that crosses a boundary has one stable code: a number and a SCREAMING name,
   written `E1104 ADMISSION_GRANT_EXPIRED`. A boundary is the wire, a log line at `warn` or
   above, a process exit, an HTTP call to or from Platform, or a tool or CI failure.
2. One u32 space, split into blocks by owner (§1.2). Wire codes stay in the protocol registry
   (1000–1999). Every other code goes in one new file,
   `docs/contracts/OTERYN_GAME_ERROR_CODE_REGISTRY.json`. A number is never reused; a retired
   code stays in the file with `status: RETIRED`.
3. Each registry entry has `code`, `name`, `category`, `progression`, `owner`, `contract`, `hint`,
   `status` and, for codes that reach a player, `public_class`. `hint` is one line saying what to
   check first.
4. `python tools/errors/explain.py E3004` prints the entry from either registry. `--scan <file>`
   counts the codes found in a log. The owner pastes a code; an agent runs one command.
5. Rust: a zero-dependency crate `crates/error-codes` (`oteryn-error-codes`) holds `ErrorCode`,
   `Category` and `Progression`, and one `macro_rules!` that declares a boundary enum's code
   kinds, its `ALL` list and its `code()` from a single list. Each boundary error enum maps every
   variant to a kind with no wildcard arm, and its crate gets a test that every kind in `ALL` is
   registered with the same name and category, and with the same progression, stated or derived
   (§1.4). The pattern is the
   existing `error_codes_match_the_registry` (`crates/protocol-oteryn/src/lib.rs`).
6. Log line (node-boot D6, extended):
   `oteryn-game-server ts=<unix-ms> level=error module=node event=<name> code=E2003 name=CONFIG_INVALID cat=INVALID_INPUT
   trace=<uuidv7> [wire=E1050] [world=… channel=… session_gen=…]
   detail="<redacted, escaped>"`. `code` is the root cause; `wire` appears only when the wire
   carried a different, public code (§1.1).
   `trace` is an ANL-01 CorrelationId: one per boot and one per accepted connection, and it is
   inherited by everything that connection causes. No secrets, tickets, grants or payloads,
   and no player-linked identifier such as a CharacterId (ANL-01 §18, §1.5).
7. A player sees the public text and the short code, for example "Session expired (E1104)".
   An unknown code shows the generic text for its block and the number.
8. Tools and CI print `E8xxx NAME: message`. Under GitHub Actions they also print an
   `::error file=…,line=…::` annotation.
9. Order: ERR-REGISTRY-0 first. ERR-NODE-1, ERR-CLIENT-2 and ERR-TOOLS-3 then run in parallel,
   all allocated now (owner, §1.8 item 4b). ERR-DIAG-4 and ERR-TOOLS-HOOK-6 follow ERR-NODE-1,
   and ERR-TRACE-5 follows ERR-NODE-1 and ERR-CLIENT-2. Each packet stands alone and leaves
   `main` consistent.
11. Debugging (§1.10): every Rust binary writes one `E4001 PANIC` line from a panic hook, and
    names its build (`version+sha`) in its first line and its panic line. The client copies a
    one-line bug report on F12. `oteryn-game-ops diagnose` turns a report or a trace into the
    matching log lines. `OTERYN_LOG` sets per-module levels for `info` and `debug` lines. A
    client that supports `CONNECTION_TRACE_V1` receives its connection `trace` and puts it in
    the report.
10. Not changed: wire code numbering, FND-02 dispositions, N8 codes 1100–1116, and the per-message
    outcome enums (`ItemMoveOutcome`, `ChatDisposition`, `SpellCastDisposition`,
    `StepDisposition`). Those enums are already typed by their message, so `ItemMoveOutcome=3`
    is unambiguous. Logs print their variant name.

## 0. What exists and what is missing

### 0.1 Survey of `main` (a372a4471)

- Wire: `FoundationProtocolError` is `#[repr(u32)]` with codes 1001–1050, mirrored in the
  protocol registry and enforced by a test. `ProtocolError` (type 14) and a REJECTED
  `CommandResult` carry `error_code`. N8 (ARCH-LOGIN-FIRST, §1.2) assigns 1100–1116 to
  admission and reserves 1117–1199.
- Rust: about 174 `enum *Error`, 16 `*Refusal` and 4 `*Reason`, all with hand-written `Display`
  and none with a code. Ad-hoc stable identifiers exist in four places, in four unrelated
  shapes: `NotDelivered::class()` strings, `oteryn-game-ops` exit codes 2–7, the Postgres
  SQLSTATEs `OTN01`–`OTN03`, `OTI01`–`OTI05` and `OTC01`, and `oteryn-diagnostics`
  `DiagnosticCode` (`D00000007`, used only by `test-support`).
- Logs: `eprintln!` key=value lines through `fn event` (`apps/game-server/src/node/serve.rs`,
  `bin/oteryn-game-ops.rs`). `event=boot_failed reason="{Display}"` is free text. No line
  carries a code or a trace id.
- Contracts: the vocabulary requires "a stable machine category and contract-owned code" and
  correlation fields. FND-04A §11, FND-04C, ANL-01 §19 (`ANL_*`) and GAME-INTERACTION-01 §13
  (`GI_*`) define symbolic codes that exist only in documents. GAME-INTERACTION-01 defers its
  numeric registration.
- Platform: failed Node→Platform HTTP calls return empty bodies; the status code is the only
  signal.
- Tools and CI: validators print free-form f-strings, and `game-gate` reports pass or fail only.
- Client: hardcoded English text per enum (`apps/client/src/spell.rs`), and some server-built
  English text (`parameter_cast.rs`).

### 0.2 Why this blocks diagnosis now

- A boot failure, a refused admission and a failed Platform call all end as a free-text line
  or a silent close. Telling them apart needs the source code.
- A single player's problem cannot be followed across lines, because nothing links them.
- A CI failure needs the full job log read by a person.

## 1. Rulings

### 1.1 What gets a code

A failure gets a registered code when it crosses a boundary (brief item 1). Internal `Result`
plumbing inside one module does not. An error enum that reaches a boundary maps every variant;
a variant that wraps another coded error returns the inner code, so the root cause is kept in
diagnostics, not the wrapper.

The wire is different. Only 1000–1999 codes are ever sent to a client. When a failure with a
non-wire root cause (2xxx–9xxx) is reported on the wire, the message carries the public wire
code that the contract owning that message assigns, and the root code stays internal. The
diagnostic line records both: `code=` holds the root code and `wire=` the code sent. The owning
contract, not this decision, chooses the wire code for each such path; this decision only
forbids sending a non-wire code. The same rule applies to a process exit and to a Platform
response: each keeps its own accepted class (node `BootError` exit statuses 10–21, ops exit
statuses 2–7, the HTTP status), and the diagnostic line written before the exit or after the
call carries the registered root code. An OS exit status is never the registered code.

### 1.2 The code space

| Block | Owner | Registry | Contents |
|---|---|---|---|
| 0 | — | — | invalid, never assigned |
| 1000–1099 | protocol-oteryn (FND-02) | protocol | existing foundation wire codes |
| 1100–1199 | admission (FND-04A, N8) | protocol | admission refusals on the wire |
| 1200–1999 | protocol-oteryn | protocol | future wire rejections, through FND-02 registration |
| 2000–2999 | game-server node lifecycle | Game | config, content, map and world load, boot, shutdown |
| 3000–3999 | durability | Game | Postgres, fences, SQLSTATE `OTN*`/`OTI*`/`OTC*`, stored-state integrity |
| 4000–4999 | runtime internals | Game | fail-closed invariant breaks that are not sent on the wire |
| 5000–5999 | Platform integration | Game | registration, runtime status (`NotDelivered` classes), grant verification |
| 6000–6999 | ops tooling | Game | `oteryn-game-ops` failures; exit codes stay 2–7 as the coarse class |
| 7000–7999 | native and browser client | Game | connect, asset, render and local storage failures |
| 8000–8999 | tools and CI | Game | governance, repository policy, content validators, qualification |
| 9000–9999 | reserved | — | assigned only by a later amendment |

A block that runs out is extended by registering a further block of 1000 above 9999. Blocks are
never split or renumbered. ANL and GI symbolic codes take numbers only when they first cross a
boundary in code, in the block of the component that emits them.

### 1.3 The registry

- `docs/contracts/OTERYN_GAME_ERROR_CODE_REGISTRY.json` holds `schema_version`, `blocks` (the
  §1.2 table) and `codes`.
- Each entry has `code` (u32), `name` (SCREAMING_SNAKE, unique across both registries),
  `category` (a vocabulary category), `progression` (`RETRYABLE`, `TERMINAL` or
  `SECURITY_TERMINAL`), `owner` (crate and module path), `contract` (owning document and
  section, or `null`), `hint` (one line, at most 160 bytes, no secrets), `status` (`ACTIVE` or
  `RETIRED`) and `public_class` (only for codes a player can see; otherwise absent).
- The validator (ERR-REGISTRY-0) checks: the schema; that each number lies in its block; that
  numbers and names are unique across both registries; that the categories come from the
  vocabulary; and that no entry was removed or renumbered against `origin/main` (append-only).
- The protocol registry keeps its own shape and owner. The tool reads both files, so they act as
  one space without moving the wire codes.
- Protocol-registry codes have the fields that registry defines: `code`, `name`, `category` and
  `default_disposition`, plus `progression` and `public_class` on the N8 admission entries.
  The registry shape is unchanged. For an entry with no `progression` member (1001–1050), the
  progression and retry requirement are derived from `default_disposition` by the fixed table
  in the vocabulary "Code space" section:

  | `default_disposition` | Progression | Retry requires |
  |---|---|---|
  | `TRANSPORT_FATAL`, `SESSION_FATAL` | `TERMINAL` | a new connection and session |
  | `OPERATION_TERMINAL` | `TERMINAL` (the operation) | a new command in the same session |
  | `RESYNC_REQUIRED` | `RETRYABLE` | the resync, then the same session |

  This keeps the public progression contract for every wire code without moving FND-02's
  fields. An explicit `progression` (N8 entries) is authoritative. The validator refuses a
  disposition outside the table, and `explain.py` prints the derived values marked `derived`.
- Allocation: the author takes the next free number in the block. Two concurrent PRs that take
  the same number fail the uniqueness check in the merge queue, and the later one renumbers
  before merge. A number is fixed once it reaches `main`.

### 1.4 Rust

- `oteryn-error-codes` holds `ErrorCode { number: u32, name: &'static str }` with `Display` as
  `E{number:04} {name}`, plus `Category` and `Progression`, which mirror the vocabulary. The
  crate has no dependencies. Its only I/O is the §1.10 panic hook, which writes one line to
  stderr.
- **One source for the code set.** `oteryn-error-codes` exports one `macro_rules!`. Its single
  input lists, for a boundary enum, each kind with its number and name. It generates a
  fieldless `Kind` enum, `Kind::ALL` and `Kind::code()`, so a kind cannot be added without
  entering `ALL`. The boundary enum's `code()` is `self.kind().code()`, and `kind()` is an
  exhaustive match with no wildcard arm. A new variant therefore fails to compile until it is
  mapped to a kind, and every kind is in `ALL`. A variant that wraps another coded error maps to
  the inner error's code (§1.1) and is tested through the inner enum's `ALL`.
- Each crate that owns a boundary enum adds a test: every code in `Kind::ALL` is in the registry
  with the same name, category and progression, stated or derived (§1.3). For 1001–1050 the
  existing `error_codes_match_the_registry` test, which compares `default_disposition`, also
  stays. The crate's dev-dependency on `serde_json` is the only cost.
- `FoundationProtocolError` keeps `#[repr(u32)]`. It gains `code()` returning the same number,
  so logs and the tool treat it like every other code.
- `oteryn-diagnostics` is not changed by this decision (§1.9).

### 1.5 The diagnostic line

- Format: brief item 6. Field order is fixed:
  1. `ts`, `level` and `module`, always (§1.10 item 5);
  2. `event`, always;
  3. `build`, only on the `process_start` and `panic` lines (§1.10 items 1 and 2);
  4. `code`, `name`, `cat`, `trace` and `wire` (only when present);
  5. the scope fields `world`, `channel` and `session_gen`, each only when known;
  6. `detail`, last.

  Lines without a failure omit `code`, `name`, `cat` and `wire`. A parser rejects a line whose
  fields are out of this order.
- No player-linked identifier (AccountId, CharacterId, GameSessionId, AnalyticsActorId and the
  others of ANL-01 §18) appears in a diagnostic line, in `detail` included. A connection's
  failures are correlated through its `trace`. Linking a trace to a character is left to stores
  that ANL-01 permits to hold that link.
- Every value except `detail` is a registered token, a number, a UUID or a typed id, and never
  contains a space, a quote or a control character. No other free-text field is allowed.
- `trace` is a UUIDv7 CorrelationId (ANL-01). The node mints one at boot and one per accepted
  connection, using the existing `uuid_v7()` in `node/serve.rs`. Work caused by a connection,
  including its admission, durability writes and Platform calls, logs that connection's
  `trace`. The trace is never taken from client input and never authorizes anything.
- `detail` is the redacted diagnostic: the `Display` of the error with no secrets, in double
  quotes. Inside the quotes, `\` is written `\\`, `"` is `\"`, CR is `\r`, LF is `\n`, and
  every other control character (U+0000–U+001F, U+007F) is `\u{XX}` in hex. The escaped value is at
  most 512 bytes. A longer value is cut at the last UTF-8 character boundary that leaves room for
  `...`, which is then appended, and an escape sequence is never split.
- `ts` is the UTC wall-clock time of the line in Unix milliseconds. It orders nothing and is
  used only to find lines (§1.10). The line therefore stays
  single and a crafted message cannot add fields or events. A grant, ticket, token, password or payload never appears in it, as the
  vocabulary and node-boot D6 already require.
- `boot_failed` becomes `event=boot_failed code=E2xxx …`; the free-text `reason` goes into
  `detail`.

### 1.6 Player-facing presentation

- The client keeps one catalogue keyed by code number, holding the public text and the
  `public_class`. A player sees the text and the short code, for example "Session expired
  (E1104)".
- A code the client has no text for (a newer code), and a code without a `public_class` (for
  example 1001–1050), shows the generic text of its block and the number, so an older client
  stays readable against a newer server. The player never sees a name or a hint.
- The number shown is the number the client already decoded from the frame, so it tells the
  player nothing their own client does not already hold. Which failures a player can tell apart
  is decided on the server: only 1000–1999 codes reach the wire, and a non-wire root code stays
  internal (§1.1). A distinction that must not reach a player is removed by sending a less
  specific public code, never by hiding a number in the client. Whether release builds show the
  number stays open (§1.9).
- The server stops building English sentences for errors that have a code. It sends the code,
  and the client renders the text. Existing server-built text is replaced only when that code
  path is touched.

### 1.7 Tools and CI

- `tools/errors/explain.py`: `explain.py E3004` (or `3004`, or the name) prints the entry.
  `explain.py --scan FILE` lists each code found with its count, name and hint.
- Validators print `E8xxx NAME: message` on the failing line. When `GITHUB_ACTIONS=true` they
  also print an `::error` annotation, so the failure shows on the PR without opening the log.
- `game-gate` stays pass or fail. Annotations are additive, and no check is weakened.

### 1.8 Flagged for owner acceptance

1. **Display format.** Recommended: `E1104` with the name beside it in logs and tools.
   Alternatives: a domain prefix (`ADM-1104`), or names only.
2. **Whether a player sees the code.** Recommended: always, next to the text. Alternative: only
   in debug builds.
3. **Platform HTTP failure bodies.** Recommended: after ERR-NODE-1, propose to Platform an
   additive JSON body `{"code": …, "name": …}` on failures, as a cross-repository item under
   Platform's contract. Until then Game maps the status code to its own 5xxx code.
   Alternative: defer until a Platform failure blocks a diagnosis.
4. **Priority.** Recommended: ERR-REGISTRY-0 and ERR-NODE-1 now; ERR-CLIENT-2 and ERR-TOOLS-3
   after the playable-path packets already in flight. Alternative: all four now.

**Owner rulings (2026-10-05, given directly to the architect):**

1. **a)** `E1104`, with the name beside it in logs and tools.
2. **a)** A player always sees the code next to the text, because that serves testing best.
   Whether release builds keep showing it is a security question deferred to a later decision
   (§1.9). Until then, no registered code may let a player tell apart states the public text
   does not already reveal. The `public_class` mapping stays the gate, and a code with no
   `public_class` never reaches the player.
3. **a)** After ERR-NODE-1 merges, the control plane routes the additive
   `{"code": …, "name": …}` failure-body proposal to Platform under Platform's contract. The
   architect has no Platform write authority. Until Platform accepts it, Game maps the status
   code to its own 5xxx code.
4. **b)** All four packets now.

### 1.9 Deliberately not decided

- A metrics, tracing or log-shipping backend (OpenTelemetry or otherwise), alerting and
  retention. These remain under gap register §26.
- Crash packages, minidumps, backtraces and any upload. These remain under gap register §15 and
  `CLIENT_CRASH_DIAGNOSTICS_PRIVACY_OWNER_BASELINE.md`. §1.10 decides only the local panic line.
- Changing log levels without a restart, and any remote control channel for it.
- Localization of client text beyond the existing language.
- Whether `oteryn-diagnostics` (a client-era event model with a u64 CorrelationId and its own
  categories) converges on `oteryn-error-codes`. Its convergence is decided when the client
  track next touches it.
- Codes for the per-message outcome enums. They stay scoped to their message (brief item 10).
- Whether release (non-test) builds show the code to the player (owner, §1.8 item 2). It is
  decided after a security review of the `public_class` codes, before the first public release.
  The client catalogue keeps the code display in one place so that the later choice is a single
  switch.

### 1.10 Debugging

**Owner rulings (2026-10-05, given directly to the architect):** 1a a panic hook in the server,
the client and the Rust tools now; 2a build identification; 3a a client bug-report hotkey plus
an ops `diagnose` command; 4a per-module log levels with ERR-NODE-1. Then 1b: the connection
`trace` goes on the wire now, behind an optional capability (item 6).

1. **Panic line.**
   - Each Rust binary (`oteryn-game-server`, `oteryn-game-ops`, the client and the Rust tools)
     installs one panic hook at the start of `main`. It writes one §1.5 line,
     `level=error module=panic event=panic build=… code=E4001 name=PANIC cat=INTERNAL_UNAVAILABLE`,
     with the boot `trace`, and `detail` holding the thread name and the `file:line:col` location.
   - The panic payload goes into `detail` only when it is a `&'static str`. A formatted payload
     (a `String`) may carry runtime values, including player-linked identifiers, so it is
     replaced by `payload=formatted`. The location and the build identify the source line.
   - The hook replaces the default hook and does not chain to it, because the default hook
     prints the unredacted payload. `RUST_BACKTRACE` output is therefore not written (§1.9).
   - The hook only writes. It never catches, unwinds or continues: the panic proceeds under
     the existing profile, and FND-03 §24 containment is unchanged.
   - E4001 is the first 4xxx code. Its progression is `TERMINAL`, and it has no `public_class`.
   - The hook is one function in `oteryn-error-codes`, so every binary installs the same one.
     ERR-NODE-1 installs it in every `apps/game-server` binary, ERR-CLIENT-2 in the client, and
     ERR-TOOLS-HOOK-6 in the Rust tool binaries of the workspace (`architecture-check`,
     `synthetic-asset-compiler`, `synthetic-client-harness` and `world-bundle-compiler`).
     Crates outside the workspace members (`tools/world-project-v2-scale-measurement`) and
     `experiments/` are out of scope.
2. **Build identity.**
   - One build script, `crates/error-codes/build.rs`, derives the source id for the whole
     workspace. Each binary passes its own `CARGO_PKG_VERSION`, so `build` is
     `<version>+<source>`.
   - `<source>` is:
     - the first 12 hex digits of `OTERYN_BUILD_SHA` when that variable is set. It must be 40
       hex digits, or the build fails. A workflow that publishes binaries sets it from
       `GITHUB_SHA` on an exact checkout;
     - else `git rev-parse --short=12 HEAD` followed by `.local`;
     - else `unknown`.
   - Only a bare SHA claims the exact source. `.local` says the build may include uncommitted
     changes. Cargo cannot re-run a build script on every work-tree edit, so a `.dirty` check
     would go stale; `.local` makes no claim it cannot keep.
   - The script re-runs when `OTERYN_BUILD_SHA`, `HEAD` or the current branch ref changes, so
     the SHA part is never stale.
   - The first diagnostic line of every process is `event=process_start build=<OTERYN_BUILD>`,
     and the panic line and the bug report carry `build`. `--version` prints it.
3. **Bug report.**
   - F12 in the client copies one line to the clipboard:
     `oteryn-report build=… ts=… code=E1104 trace=… world=… channel=…`. It names the last code
     the client showed, the time it was shown, and the scope known then.
   - `trace` is the connection trace (item 6) received on the connection that delivered that
     code. Before any code is shown, it is the current connection's trace. It is omitted when
     no trace was received.
   - Before any code has been shown, F12 copies the line without `code`.
   - The report carries no account, character, session or other player-linked identifier
     (ANL-01 §18) and no free text.
   - Nothing is uploaded: the player pastes the line. The crash-diagnostics baseline is not
     touched.
   - Where the clipboard is unavailable, the client writes the same line to stderr.
4. **`oteryn-game-ops diagnose`.**
   - `diagnose --log <file> --trace <uuid>` prints every line of that trace in order.
   - `diagnose --log <file> --report "<oteryn-report line>"` prints all lines of the report's
     `trace` when it has one. Without a `trace`, it finds the lines with the report's code,
     world and channel within ±5 s of its `ts`, prints each match's `trace`, and then prints all
     lines of those traces.
   - Each printed code is followed by its name and category, and by its hint where the entry
     has one. Codes 1000–1999 are read from the protocol registry, all others from the Game
     registry.
   - The command only reads the named file. It needs no database and no Platform access.
5. **Log levels.**
   - Each §1.5 line has a level (`error`, `warn`, `info` or `debug`) and a module: the
     top-level module of its call site, for example `node`, `durability` or
     `gameplay_transport`.
   - `OTERYN_LOG` is read once at process start, for example `OTERYN_LOG=info,durability=debug`.
     The default is `info`.
   - Levels filter only `info` and `debug` lines. A `warn` or `error` line, and every line that
     carries a code, is always written.
   - A malformed `OTERYN_LOG` fails the start with a registered 2xxx code (6xxx for the ops
     tool), so a typo cannot silently hide lines.
   - Existing free-form `eprintln!` calls move to the leveled writer when their module is next
     touched.
6. **Connection trace on the wire** (owner, 2026-10-05, 1b). Amends FND-02 §18.
   - A new optional capability `CONNECTION_TRACE_V1`. The #1622 control plane leases its number
     (the next free id is 19). Its registry entry has no command types and no state domains,
     `offered: false`, and an offer gate naming ERR-TRACE-5.
   - A new field `bytes connection_trace` in three messages:
     - `ServerAccepted` field 11 (reserved becomes 12 to 20);
     - `ServerResumeAccepted` field 7 (reserved becomes 8 to 16);
     - `ProtocolError` field 6 (reserved becomes 7 to 15).
   - The value is exactly 16 bytes, a UUIDv7, never all-zero. It is the same `trace` that the
     server's diagnostic lines carry for that connection.
   - When the server sets it:
     - in `ServerAccepted` and `ServerResumeAccepted`, only when the capability is selected;
     - in a `ProtocolError` after acceptance, only when the capability is selected;
     - in a `ProtocolError` before acceptance (an admission refusal such as E1104), only when
       the decoded `ClientBootstrap` or `ClientResume` listed the capability as supported and
       the server offers it. This is a narrow exception to "active only if selected" (FND-02
       §9): the refusal ends the attempt before any selection exists, and the field carries
       nothing the client can act on.
     - Never before a bootstrap or resume has been decoded, so a malformed first frame gets
       no trace.
   - A client decodes the field only on a connection where it listed the capability. Elsewhere,
     and for any value that is not 16 bytes, nil, or not version 7, the frame is
     `MALFORMED_FRAME`, as with any other unknown or invalid field.
   - The trace is diagnostic only. It grants nothing, is never accepted from a client, and is
     never used as an identity, a key or a fence. It is a per-connection random value with no
     player-linked identifier inside it (ANL-01 §18). The client keeps it in memory only, for
     the report.
   - Older peers: an older client does not list the capability, so the server never sets the
     field; an older server does not offer it, so the field never appears. Strict decoders on
     both sides therefore see no change.

## 2. Packets

The CP checks path ownership against open PRs before allocation.

### 2.1 ERR-REGISTRY-0 (registry, validator, explain tool)

- Owned paths: `docs/contracts/OTERYN_GAME_ERROR_CODE_REGISTRY.json`, `tools/errors/**`, the
  wiring of the validator into `tools/repository/validate_repository_policy.py`.
- Scope: the registry with `blocks` and seed codes for what already has a stable identity: the
  `NotDelivered` classes (5xxx), the ops failures (6xxx, keeping exit statuses 2–7), the SQLSTATEs
  `OTN01`–`OTN03`, `OTI01`–`OTI05` and `OTC01` (3xxx), and the `BootError` variants (2xxx). Also
  the validator (§1.3) with its append-only check, `explain.py`, and unit tests under
  `tools/errors/tests/`.
- Validation: the new tests; `validate_repository_policy.py`; and a test that every protocol
  registry code resolves through `explain.py`, with each 1001–1050 code showing its derived
  progression from the §1.3 table, and that a protocol entry with a disposition outside the
  table is refused.

### 2.2 ERR-NODE-1 (Rust codes and the node diagnostic line)

- Depends on ERR-REGISTRY-0.
- Owned paths: `crates/error-codes/**` (new), the workspace `Cargo.toml` member line,
  `crates/protocol-oteryn/src/lib.rs` (only `code()`), `apps/game-server/src/node/serve.rs`,
  `apps/game-server/src/main.rs`, `apps/game-server/src/bin/oteryn-game-ops.rs`,
  `apps/game-server/src/native_admission_source/runtime_status.rs`, and the durability error
  mapping module that reads SQLSTATE.
- Also owned: the `main` of `oteryn-game-migrate` and `oteryn-game-import-proficiencies` (the
  hook install and `process_start` only). The build script is `crates/error-codes/build.rs`.
- Scope: §1.4 and §1.5 for boot, registration, runtime status, ops and durability, with `ts`.
  Also §1.10 items 1, 2 and 5 for `oteryn-game-server` and `oteryn-game-ops`, and the E4001
  entry in the registry. Admission
  logging adopts `code=` and `trace=` where N8-1 already maps codes. If N8-1 has not merged,
  admission is left to N8-1.
- Validation: the registry-match tests in each touched crate, the existing node-boot tests, a
  test that a `boot_failed` line parses into the §1.5 field order, and adversarial `detail` tests
  (quote, backslash, CR, LF, other control characters, a forged ` code=` and multi-byte text at
  the 512-byte cut) that each yield one line that parses back to the original fields. Also
  a test that a connection-scoped failure line has no `character=` field and no CharacterId
  text in `detail`, and a test that `boot_failed` exits with its existing `BootError` status.
  §1.10 tests:
  - a forced panic in a test binary writes exactly one E4001 line;
  - a `&'static str` payload appears in `detail`, and a formatted payload is withheld;
  - the first line of a process is `process_start` with `build`;
  - `OTERYN_BUILD_SHA` set gives a bare SHA, unset gives `.local`, and a value that is not 40
    hex digits fails the build;
  - every line parses in the §1.5 field order, with `level` and `module`;
  - `OTERYN_LOG=warn` drops `info` lines but keeps every coded line;
  - a malformed `OTERYN_LOG` fails the start with its code.

### 2.3 ERR-CLIENT-2 (client catalogue)

- Depends on ERR-REGISTRY-0. §1.8 items 1 and 2 are ruled (1a, 2a).
- Owned paths: `apps/client/src/` (a new `error_text` module and its call sites for wire codes,
  plus the panic hook and the bug report) and `apps/client/Cargo.toml` (the
  `oteryn-error-codes` dependency, plus a clipboard dependency only if the existing platform
  crates cannot write the clipboard).
- Scope: §1.6 for codes 1000–1199. Unknown codes fall back to the block's generic text. The code
  is shown in every build, rendered by one function, so that the deferred release-build choice
  (§1.9) changes one place. A test checks that only codes with a `public_class` are shown with
  their own text, and that a code without one shows the block's generic text and the number
  but no name.
- Also §1.10 items 1 to 3 for the client. Tests:
  - the report line holds exactly the §1.10 fields, with no player-linked identifier;
  - the report has no `code` before any code is shown;
  - a panic writes one E4001 line with `build`.

### 2.4 ERR-TOOLS-3 (validator output)

- Depends on ERR-REGISTRY-0.
- Owned paths: `tools/agents/validate_governance.py`, `tools/repository/validate_*.py`, and the
  8xxx entries in the registry.
- Scope: §1.7 for the two governance validators first; other validators adopt it when touched.

### 2.5 ERR-DIAG-4 (`oteryn-game-ops diagnose`)

- Depends on ERR-NODE-1.
- Owned paths: `apps/game-server/src/bin/oteryn-game-ops.rs` (the `diagnose` subcommand) and a
  new `apps/game-server/src/ops_diagnose.rs` module.
- Scope: §1.10 item 4. Both registries are embedded at build time:
  `docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json` for 1000–1999 and
  `docs/contracts/OTERYN_GAME_ERROR_CODE_REGISTRY.json` for the rest.
- Validation, with fixture logs:
  - `--trace` prints exactly that trace's lines in order;
  - `--report` finds the matching trace inside ±5 s and none outside it;
  - a line whose `detail` contains a forged ` trace=` does not match;
  - a report with `trace=` prints that trace without the time match;
  - `E1104` prints its protocol registry name and an `E3004`-style code its Game registry
    entry;
  - an unreadable file exits with its 6xxx code.

### 2.6 ERR-TRACE-5 (connection trace on the wire)

- Worker: hard-worker (protocol wire format). Needs protocol review before merge.
- Depends on ERR-NODE-1 (the server trace) and ERR-CLIENT-2 (the report).
- Owned paths:
  - `docs/contracts/protocol-oteryn/v1/foundation.proto` (the three fields and their reserved
    ranges);
  - `docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json` (the capability entry and the
    `foundation_schema` sha256);
  - `docs/contracts/CROSS_REPOSITORY_CONTRACT_LOCK.json` (only the `schema_sha256` of
    `foundation.proto`, which must match the registry);
  - the FND-02 §18 amendment marker, changed from pending to in force;
  - `crates/protocol-oteryn/src/lib.rs` and `crates/session/src/lib.rs` (encode and decode of
    the three fields);
  - `apps/game-server/src/foundation/protocol.rs` and
    `apps/game-server/src/gameplay_transport/connection.rs` (the server sets the field);
  - the client's protocol handling and the report in `apps/client/src/`;
  - the golden and cross-version fixtures of these messages.
- Scope: §1.10 item 6, and `trace=` in the report (§1.10 item 3). The capability is offered
  once both sides pass the tests below; the packet then sets `offered: true`.
- Validation:
  - golden fixtures of the three messages with and without the field;
  - an older client fixture (capability not listed) receives no field from a new server,
    including on a pre-acceptance refusal;
  - a new client against a server that does not offer the capability decodes as before;
  - a pre-acceptance `ProtocolError` carries the trace only when the capability was listed;
  - no trace before a bootstrap or resume is decoded;
  - the client rejects a field of 15 or 17 bytes, a nil value, a non-v7 value, and a field
    on a connection where it did not list the capability;
  - the trace in `ServerAccepted` equals the `trace` of that connection's server lines;
  - the report carries `trace=` after a refusal and omits it when none was received.

### 2.7 ERR-TOOLS-HOOK-6 (panic hook in the Rust tools)

- Depends on ERR-NODE-1 (the hook and the build id).
- Owned paths: `src/main.rs` and the `oteryn-error-codes` dependency line in `Cargo.toml` of
  `tools/architecture-check`, `tools/synthetic-asset-compiler`,
  `tools/synthetic-client-harness` and `tools/world-bundle-compiler`.
- Scope: §1.10 items 1 and 2 for these binaries: the hook install, `process_start` and
  `--version`. Their other output is unchanged.
- Validation: a forced panic in one tool writes exactly one E4001 line with `build`, and each
  tool's existing tests pass.

## 3. Rejected options

- **Error library adoption (`thiserror`, `anyhow`, `miette`).** These change how errors are
  written, not how they are identified, and would touch about 190 enums. Codes need only one
  method per boundary enum.
- **Codes only as names.** Names alone cannot be range-checked or read out by a player, and
  short numbers are easier to report.
- **One merged registry file holding the wire codes too.** That would move FND-02-owned data
  into a new owner and break the existing protocol test. Two files read as one space keep each
  owner.
- **Building on `oteryn-diagnostics` `DiagnosticCode`.** Its categories, u64 correlation and
  client session types do not match the vocabulary or ANL-01. Bending it would change a crate
  that three other crates depend on, for no playable-path gain.
- **`tracing` and `tracing-subscriber` for levels.** Upstream is the default, but the line
  format is already fixed (§1.5) and one level check in the existing writer covers the ruling.
  Adopting `tracing` would rewrite every call site and its output. It is reconsidered together
  with an observability backend (§1.9).
- **Matching the bug report by code, time and scope only.** It keeps the wire unchanged, but
  two players hitting the same code in the same channel within seconds are ambiguous. The
  owner chose the trace on the wire (§1.10 item 6); the time match stays as the fallback.
- **The trace as a mandatory core field.** Older peers decode strictly, so an ungated field
  would break them. A capability gates it (FND-02 §8).
- **JSON log lines.** Node-boot D6 already fixed single-line key=value. A key=value line is
  greppable by a person and parseable by the tool.

## 4. Checklist and decision test

### 4.1 Checklist

1. Owning contract: the code space is an amendment to `FOUNDATION_ERROR_VOCABULARY.md`. Wire
   codes stay in the FND-02 registry. The connection trace on the wire is an amendment to
   FND-02 §18, and its fields and capability are registered by ERR-TRACE-5 in the FND-02
   schema and registry.
2. Concurrency: allocation is serialized by the merge queue through the uniqueness check
   (§1.3).
3. Restart-sufficient: codes are static data. A trace lives only in log lines and is never
   needed for recovery.
4. Typed references: `ErrorCode` is a typed pair; registry entries name their owner module and
   contract.
5. Older peers: an older client shows a generic text for a new code (§1.6). An older Platform
   keeps empty bodies, and Game maps the status code (§1.8 item 3). The connection trace is
   gated by `CONNECTION_TRACE_V1` in both directions, so an older client or server never sees
   the new field (§1.10 item 6).
6. Split work: each packet leaves `main` consistent. The registry lands first, and every later
   packet only adds entries and the code that uses them. ERR-DIAG-4 reads only lines that
   ERR-NODE-1 already writes. ERR-TRACE-5 lands the schema, the registry, both codecs and the
   fixtures in one PR, and offers the capability only in that PR's final state.

### 4.2 Five questions

1. **Why now:** each new packet adds untraceable failures. The node, admission and runtime
   status are being wired now, and the cheapest time to give them codes is before more call
   sites exist.
2. **What it unblocks:** fast identification by the owner and the agents (one code, one
   command); N8's codes become part of a system; FND-04C, ANL and GI get a place to register.
3. **What gets harder later:** nothing structural. Blocks and the append-only rule only
   constrain new numbers.
4. **What evidence would supersede it:** a chosen observability backend that needs a different
   correlation scheme, or a block that cannot hold its owner's codes. Either is handled by
   amendment, not renumbering.
5. **Not decided:** §1.9.
