# Action Bar Cooldowns V1

Owner allocation: cg-abar T168, 2026-11-09. Optional capability 463 requires item-use capability 15 and spell-catalogue capability 287 (which identifies runes). Command 6147 reads the admitted actor's existing food/potion exhaustion, in-flight item use and rate gate. It never starts a cooldown, reserves an item, casts or uses anything. Spell and rune cooldowns continue to use the existing spell/group data.

## Request

A count byte (0–3), then that many little-endian u32 current-generation item-definition references, positive and below 2^28, strictly increasing. Exact length, maximum 13 bytes. Unknown definitions and stale actor/session fences are rejected. No actor identity or client time is accepted.

## Reply

An accepted reply repeats the count and ordered references, each followed by a little-endian u64 remaining milliseconds measured by the owner. Exact length, maximum 37 bytes. Zero means the existing item-use gate is ready for that definition. Other eligibility, durable special-item restrictions, inventory and target checks remain with the ordinary use command. Rejected replies are empty. The client must match the requested references and re-prove its slot assignment before acting.

The query is advisory; the existing use command remains authoritative and may refuse after the read. At activation, the client waits for the bounded read, combines it with its latest spell/group cooldown data, and sends at most one existing action. No automatic retry or action chain follows a server refusal.
