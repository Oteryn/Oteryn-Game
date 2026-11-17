# Support intake V1 (T153, T154)

Owner decision: cg-support, 2026-11-09. Platform owns web/control-plane intake;
no accepted crash/survey endpoint exists in Game's contracts or platform-client.
This is the minimum Game-owned authenticated session fallback, not a web API.
Reference: Tibia interface manual (capture 2026-09-28) and parity audit T153/T154
CrashReportDialog / SurveyInvitationDialog evidence. The manual notes do not
specify a diagnostic payload; Oteryn retains its existing minimal previews.

Capability 462 SUPPORT_INTAKE_V1 is offered in all worlds. Mechanic 5132;
migration 2712. Commands 6144 CRASH_SUBMIT and 6145 SURVEY_SUBMIT share the
following schema. Assigned command 6146 remains unused.

## Request

Byte 0 must be 1 (explicit consent), followed by 1–4096 bytes of valid UTF-8
preview text, without NUL. The dialog displays exactly this text before opting
in, including a submission reference minted for that decision. Declining emits
no command and creates no approved file. Nothing is uploaded without explicit
consent; crash capture stays local until approved. There is no panic text, memory dump, environment, email, IP, account or
character data in this payload. Crash source paths are reduced to a basename.
The server still authenticates and fences the transport session normally.

Only approved-crash.txt and approved-survey.txt in the client settings directory
are queued at the next play-channel creation. Consent explicitly covers delivery
on the next signed-in game session. At most one approved file per kind is retained;
a new decision cannot overwrite pending approved evidence. Local-only save APIs
remain available. Pending crash evidence is retained if preparing delivery fails.

## Reply

Exactly byte 1 acknowledges durable receipt. Unselected capability, malformed or
unconsented payload and stale runtime/session/lease/scope/node/recovery authority
are refused; unavailable storage never acknowledges delivery. An unsupported
session or failed/refused/malformed reply leaves the approved file for another
session. Only an accepted/duplicate result with byte 1 removes the exact file
that was sent. Gameplay delivery uses the existing play command queue.

The append-only store contains kind, displayed preview and SHA-256(kind || preview)
only. It stores no actor, session, address or account linkage. The displayed
reference makes separately approved identical answers distinct; exact retries
are deduplicated across sessions. Hash conflicts never overwrite evidence.
Writes prove current runtime authority and recheck durable fences in a short
transaction. Runtime has SELECT/INSERT only; control has SELECT only. Platform
export, staff tooling, automated surveys and retention services are outside this
slice. No external destination or credential is introduced.
