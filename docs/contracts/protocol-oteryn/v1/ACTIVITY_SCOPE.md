# Activity scope projection (domain 99)

Authority: PR2769 answers 6543120139 and 6543189919. Capability 117 selects this
domain; capability 117 selects this new state domain; no new command or capability identifier is introduced.
The strict binary payload is defined by `crates/protocol-oteryn/src/activity_scope.rs`.

The payload identifies one committed handoff: its UUID, source and destination
GameSession IDs, exact tagged Channel or Instance scope, ownership generation,
connection generation, Character lease generation and retained capability
selection. UUIDs, nonzero generations, canonical capability order and the exact
scope tag are mandatory. A Channel UUID never stands in for an Instance UUID.

## Live handoff

The server sends the domain-99 snapshot in the source stream, after the initiating
command's real result if this participant issued that command. Other participants
receive no fabricated command result. On complete snapshot commit, a client still
on `source_session` switches to `session` and its advertised generation and
selection, then consumes the destination's full initial snapshot. Buffered source
input is discarded at this barrier. An interrupted write loses the already
committed destination's actual control, not the terminal source session.

## Initial snapshot and requested recovery

The initial full snapshot of an already admitted activity session projects the
same retained binding as its first domain, before spatial and item state. If the
client is already on payload `session`, this is an idempotent current-scope
projection: it does not reset the command counter, server sequence or generation
again. A client on neither the source nor destination session rejects the
binding. The domain revision is the admitted destination connection generation.

`ServerResumeAccepted` is sent only in response to a genuine `ClientResume`.
Recovery reauthenticates the credential against current account, Character,
World, scope and policy facts and preserves the immutable capability selection.
An unacknowledged handoff can return a different, proven destination session;
its ensuing initial snapshot carries domain 99 as described above. This uses
neither the House domain 51 nor an unsolicited command or resume message.

Historical handoff lookup is discovery only. A started or later controlled
session is never attached by that lookup alone. Genuine owning control loss
uses the existing complete-reconnect owner. An unused destination without a
genuine loss uses the explicit, verified activity-recovery journal, which creates
no fabricated control-loss observation or reconnect benefit. Each real first
command journals its current transport and generation before gameplay dispatch,
closing the race with that recovery path.

The runtime preserves its actual actor, vitals, spells, cooldowns and current
placement during recovery. Authored destination coordinates are provenance,
not permission to place the actor again or restore old health.
