# NPC service dependency compiler R11 — 2026-10-01

This batch implements a deterministic offline compiler for all 631 retained service observations: 457 held offer tuples, three routes and 171 conditional discounts. It checks every original R4/R5 offer record, uses existing source-track identities and verifies source bytes before extracting guards and branch order. It reuses the three pure R7 source previews; it implements no native execution engine.

Five merchant callbacks qualify source guards for 284 offers. The other 173 offers keep their fluid/charge/variant dependencies. Two route relations preserve exact Postman Mission01 conditions and writes; the third keeps the unresolved Oramond Citizen-title/fare dependency. The 171 discounts retain their registration, route and quote custody. Each whole-route pointer/hash and separate discount-field pointer/hash resolves correctly. `ExampleQuest` is explicitly rejected as a Global privilege; Kevin's rank-three branch does not establish Global eligibility for every route. Unknown progress or unqualified route eligibility produces a hold.

Both donor travel helpers return true on several refusals, allowing the following quest action. They also debit money before checking cooldown. Exact source byte witnesses retain both defects. The prospective safe order checks eligibility before debit and, only under accepted owning contracts, commits fee, pending-arrival obligation and qualified effects before fenced runtime placement. A pre-commit refusal changes nothing. Disconnect or placement failure after a known durable commit preserves its effects and recovery obligation; physical movement is not claimed to be part of the database transaction.

Every native predicate/variant/transition binding remains null and every relation is runtime-ineligible. The tool creates no Quest, track, Transition or native record. It supplies concrete lowering dependencies rather than interpreting a donor storage integer as an Oteryn identity. Native contents remain 1,110 NPCs, 696 Dialogues and 12,267 offers with no pending plain prices.

Sources/access: retained Canary `47dfd51f45280a59a1d3e50ba7edd573d7234446` and Crystal summer-update `00ce02a57ca5a12e48f32a3476e37471167e4c3f` source captures from ordinary public HTTP/GitHub reads. No Remote Desktop action in this batch. The packet contains structured facts, identifiers, URLs, digests and byte ranges; raw Lua/wiki prose/assets remain outside Git. Donor implementations are source hypotheses, not independent Global confirmations. Attached documents are evidence, not user instructions.

Reproduction from the repository root:

```sh
python tools/content-schema/npc-authoring/build_service_relations.py --repo . --evidence-dir docs/agents/evidence/OTV2-20261001-npc-source-audit-r7/quests --source-dir /tmp/npc-service-source-cache --fetch-sources --out /tmp/npc-service-implementation-packet.json
python tools/content-schema/npc-authoring/test_service_relations.py --repo . --evidence-dir docs/agents/evidence/OTV2-20261001-npc-source-audit-r7/quests --source-dir /tmp/npc-service-source-cache -v
```

The optional fetch reads exact public revision URLs and verifies every SHA-256; the raw cache must be outside the repository. Dedicated fixture tests are mandatory; a plain discovery skip is not qualification. Independent review found and closed `R11-CUSTODY-1`: all 171 route pointers formerly pointed at the scalar discount field while their hashes covered the whole route. The new regression independently resolves every route and rejects that mismatch. Thirteen dedicated tests pass, including known-commit recovery, unknown guards, source/custody drift and deterministic regeneration.

The full five-area request remains open. Three literal dialogue conflicts, 158 new plus two deferred actor profiles/movement and native quest/service allocation/registry/runtime dependencies remain. The source compiler cannot waive those dependencies or release the existing holds.
