# World callback item reward contract

Owner scope: reusable quest engine support required by questpc4; explicit owner
instruction authorizes implementing shared reward/binding support in this lane.
This extends the existing fenced Quest transaction and complete reward MINT;
no protocol, authority, identity or persistence-schema change is introduced.

A structured `item_reward` on a USE callback supplies one bounded root reward
and at most20 direct children. Its optional positive fluid subtype describes a
fresh variable-fluid vessel, count1, admitted by the active native generation.
Intake/present evidence must not specify fluid subtype: exact fluid input
matching is outside this contract and cannot be approximated by item identity.
Missing fields preserve all previous canonical receipt bytes.

Fresh commits require the current physical world occurrence, source selector,
all locked storage gates, current session fence and generation-bound wheel-aware
capacity participant. Generic transition callers cannot advance the reward flag.
The physical occurrence, complete item MINT, filled child state, once claim and
quest storage/revision/receipt commit or roll back together. Failed capacity or
stale authority writes nothing. Existing exact receipt replay wins before fresh
source/effect execution and never refills a subsequently emptied vessel.

The ordinary world-callback API remains source compatible and admits no item
reward; an explicit admitted-item variant composes the existing owner. Runtime
USE and pending USE recovery supply the same native generation and weights.
The immutable world callback JSON (including the entire reward plan) enters the
existing occurrence binding. Existing outfit and positional effects are preserved.

Proof: focused real PostgreSQL native-room USE case, missing-gate/wrong-barrel
refusal, whole bag and child-fluid facts, repeated command/new-command behavior,
no-room rollback and stale fence. Regression: existing complete fluid MINT and
native Small Ruby pickup. Independent exact-head review remains required for
final landing; the prior review service authentication failure is not a clean review.
