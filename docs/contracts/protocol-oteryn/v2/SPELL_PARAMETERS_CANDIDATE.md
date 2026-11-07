# SPELL-PARAMETERS-2 candidate

The owner request in this session authorizes parameter-bearing native spells,
including Find Person, Creature Illusion, Levitate and Aleta. This additive
candidate preserves the accepted v1 command and its 32-byte strict codec.

`ACTOR_SPELL_PARAMETERS_V2` and `WORLD_ACTOR_SPELL_CAST_PARAMETERS_V2` require
explicit registry allocation by the protocol owner. This candidate allocates no
numeric IDs. The existing Hello negotiation must advertise and select the
allocated capability before the connection accepts the new command. An unknown
v1 field never enables v2. The server must preserve the admitted connection's
actual selected capability set; a payload flag is not negotiation authority.

The new payload is described by `actor_spell_parameters_v2.proto`: required
revision 2, required strict v1 intent bytes, and optional UTF-8 text. The text has
an explicit candidate bound of 128 bytes and forbids Unicode control characters;
the complete payload is at most 192 bytes. Unknown/repeated fields reject. An
absent parameter differs from an explicitly empty parameter. Current content's
spell-specific parser applies the source name/direction constraints afterwards.

The current book resolves the nested index, including alias selection. The
caster's actual session, Character generation, actor, map/content pin and target
owner remain authoritative. Parsing a parameter grants no target authority.
Find Person reads the existing Character/admission/position owners and target
privacy in the same sealed database transaction. Unknown staff projection or
relations cannot become public access. The common cast owner stages cost and
cooldown exactly once; a returned message plan is not a completed cast.

Aleta editor state is server-issued, bound to current HouseId, Character,
GameSession, list kind and ACL revision. Opening an editor grants no future save
permission: the save command independently rechecks the current ACL and session.
GUI and Aleta use the same revisioned world-global House owner, preserving the
existing house-item custody owner. Names resolve to native CharacterId; missing
guild/rank authority cannot become an unrestricted grant.

The strict v2 result carries the unchanged v1 disposition plus optional private
feedback and an optional server-issued House editor. Results are bounded to
8192 bytes, feedback to 1024, and editor text to 4096. Editor tokens are UUIDv7
and bind a native House key, typed list, ownership revision and ACL revision.
A failed disposition cannot carry an editor. Its text is presentation: the save
owner resolves names to actual CharacterId and rechecks current authorization.
The common owner must retain the exact command result in its existing result
outbox before committing cost; replay delivers that same result without
reopening an editor or spending again. No feedback is broadcast to other actors.

The candidate permits an effect-only private cue by encoding empty feedback text
with a non-None typed effect; empty text plus None rejects. This carries genuine
Rope/Levitate effects without adding a fabricated success message. It does not
change the accepted v1 result or allocate capability/command identifiers.

The implemented parameter compositor retains the original normalized intent,
private result, cost/training request and actual physical reservation before SQL
can COMMIT. Rejected Find Person name resolution writes a cooldown-only source
receipt, spends no mana/soul and produces no paid training. Actual source callback
false writes a result without payment. Aleta Guest/Subowner permission denial
retains its source successful return and ordinary cost; Door denial fails.
Migration 0044 binds private output to exact Character/World/session/command and
same-transaction successful source cost/editor; historical result bytes cannot
reopen editors or authorize another relocation. Current session, lease, scope,
Content, position and player state are independently rechecked on reconciliation.

For accepted source relocation, static and actual durable Item eligibility is
linearized at the common SQL source COMMIT. Item serialization ends at COMMIT;
it is not claimed to remain locked afterward. The genuine Foundation batch
reserves the exact actor and destination before SQL, retains those reservations
on an unknown outcome, and installs payment and movement in one physical commit
without another awaited call or destination selection after observed COMMIT.

The same candidate parameter port supports the authored `player_name` targeting
of Heal Friend, Nature's Embrace and Restore Balance. The existing durable naming
namespace supplies a bounded exact or `~` prefix query; current target admission,
source administrative group, actual colocated player slot and live health are
independently checked in the caster transaction. Exiva privacy does not grant or
deny healing. Cross-scope physical owners and missing source group facts remain
unavailable. The query is never translated into an attack-target command.

Successful named healing uses the ordinary combat owner, including source
range/floor/wall/self restrictions, target HP and dispel changes, common payment,
training and presentation. Its original encoded v2 parameter bytes and resolved
exact actor are sealed before SQL. An unknown outcome retains that prepared
decision; reconciliation performs neither another name lookup nor another roll.
The result row shares the source-cost transaction. A nonexistent/ambiguous name
starts only source cooldowns, with zero mana, soul, training and Harmony success.
Pinned Canary `spells.cpp` leaves `RETURNVALUE_NOERROR` unchanged when it hides
an administrative target or rejects a dead named target; its literal private
feedback is therefore `No error.` plus Poff. Missing physical-owner evidence is
unavailable rather than an invented offline result.
