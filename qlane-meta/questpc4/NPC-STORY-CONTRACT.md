# Shared authored NPC stories and completion

Owner scope: selected L6 quest recipes. Reuse the shared atomic NPC reward
transaction, premium gate and retained-possession checks. Keep generated donor
callbacks in their owning producer; merge authored shards before existing
closed validation, enforcing combined bounds and duplicate/ownership checks.
Authored read-only offer shards use the existing dialogue selector and check
completed stage before presenting earned conversations; they create no Quest
receipt or value grant.

The optional `completes` field defaults false and is omitted from serialized
legacy plans. A true field completes only the callback's owning quest through
the existing Quest writer. Migration1064 adds an exact deferred proof: source
plan and same-transaction Quest receipt must agree, and a true completion must
match the owning state's completed revision. Existing MINT, fee, participant,
consumption and native authority guards remain applicable.

Spirit is the first consumer. It keeps all three discovered11544 wills; the
confession grants conversation access and marks this selected quest completed.
It does not grant Wayfarer entitlement. Separate per-position discovery flags
and a final retained-possession check prevent substitute purchases or duplicate
boxes from bypassing the chosen collection route. Binding assumptions and pinned
facts are in content/quests/authoring/spirit-wills.md.

Authorization: owner's session instruction explicitly assigns shared engine
features and missing bindings to this lane. This contract does not claim whole
qualification before the full PostgreSQL story, refusal/replay checks and
changed-crate gates pass. No PR, deployment or main merge is authorized here.

Recognized quest callback keywords keep the existing precedence: an eligible
callback wins, then an eligible read-only offer. If neither is eligible, a known
callback keyword is sent to its existing owning writer for an explicit progress
refusal. It must not return ordinary-chat success for a failed quest request.
Unknown words and other NPCs still use ordinary conversation. This closes the
actual PG failure in /tmp/questpc4-spirit-flow-pg.log (93.353921244s); its strict
Rejected assertion remains in the test.
