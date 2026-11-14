# Game enforcement transport v1

Implemented default-off against Platform c5780e2cfc445a134cdb0ae0f9587837856c6769,
accepted revision 2. This records Game implementation choices, not activation approval.
The frozen command/result fixtures and Part-A deciding owner are unchanged.

## Configuration and activation

The composition root reads `OTERYN_PLATFORM_ENFORCEMENT_ENABLED` once: absent means
OFF, literal `true` means ON; every other value is a startup configuration error.
The resulting boolean is passed into serving state and inherited by Activity scopes.
OFF constructs no clients/tasks, reads no enforcement storage for player chat, and
requires no enforcement credentials. The two purpose identities are
`OTERYN_PLATFORM_ENFORCEMENT_READER_CLIENT_IDENTITY_PEM` (A1/A2/A4) and
`OTERYN_PLATFORM_ENFORCEMENT_RESULTS_CLIENT_IDENTITY_PEM` (A3). They must differ
from one another and every other configured Platform client identity. Endpoint,
TLS peer name and CA use the existing `OTERYN_PLATFORM_STREAMS_URL` and
`OTERYN_PLATFORM_STREAMS_PLATFORM_CA_PEM` configuration, without a new endpoint variable.

An enabled node also requires the existing Part-A durable configuration row to
be enabled with a reviewed `supported_policy_revision`. Startup refuses an
unconfigured row; the transport does not invent a policy ceiling or overwrite
operator policy. Literal flag enablement alone is not profile/policy acceptance.

Platform's section 6.4 restore-notice amendment remains an external activation
prerequisite. This Game implementation does not claim that the Platform amendment,
production PKI allocation or production activation has happened. Results must stay
disabled on a restore-enabled stack until that amendment is deployed.

## Task and authority lifetime

Three supervised jobs own separate pull/publish/reconcile state and the existing
abort-on-drop shutdown lifetime. A bounded three-request queue connects them to
the serving owner's existing sealed Character authority and incarnation proof.
The network jobs never retain, reconstruct or mint a Character authority. The
serving owner serializes each request through an ordinary bounded semantic pass;
the Part-A deciding transaction still performs its recovery, incarnation, relation
and revision checks. Dropping serving cancels the jobs and their queue before the
sealed authority is released.

A1 advances its durable cursor only after every command decision commits. Partial
page failures keep the previous cursor; immutable receipt replay handles restart.
Empty A1 pages preserve its cursor. A4 has an independent durable cursor and resets
it after an empty page, making reconciliation a recurring complete scan. Unknown
commands are fetched through A2; missing Platform results requeue original bytes;
different result digests alarm without changing authoritative state.

## Delivery and failures

All routes are compiled, HTTP/1.1 POST over the existing purpose-bound TLS 1.3
client builder, with Connection: close, one in-flight request per job and a
three-second whole-exchange deadline. Healthy idle polling is five seconds, like
the security-revocation puller. Retry delays are 1, 2, 4, 8, 16, 32, 60 seconds,
honouring a longer Retry-After. No timeout or unexpected/oversize acknowledgement
clears an outbox row.

Migration 2835 keeps immutable command bindings and immutable per-revision result
bytes. A3 marks acknowledgements; it never deletes history. A 409 holds its row;
a 404 durably stops publication for the operation, including later receipt revisions,
and marks operator reconciliation. Automatic reconciliation cannot release either
hold. A3 400/401 alarms and backs off the whole publisher without holding any row;
recovery retries the original stored bytes. A1/A2/A4 400, 401, 404 or 409 stops
the affected durable pull/reconcile cursor and alarms; fixing the defect or
credentials requires operator reconciliation before the stop is cleared.
Stopped read requests are not retried automatically.

Overdue delivery is claimed under the receipt lock at 300,000 ms after Game's
decision time, at most once per operation across retries/restarts. The publisher
checks overdue work at most once per 60-second alarm scan interval, independently
of publication throughput. Alarms use the
registered social `enforcement_alarm` event with a bounded reason; reconciliation
drift also increments the bounded `enforcement_reconcile_drift_total` counter.
No target identifiers, command/result bodies, client keys or bearer material are logged.
