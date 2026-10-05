Current accepted main ad7a08f: 3263 transitions; 387 unique refused transitions.
168 COMPUTED plus 221 inexact-from, overlapping on two transitions: 166 computed-only, 219 guard-only, two both.

The audit preserves every original transition, owner, Source track, requested_by, callback, server list and Source transition digest. Every write is anchored to the pinned donor capture/container and raw byte span: 679 donor occurrences. AST categories are evidence, not executable approximations. Enclosing Source guard spans are retained with branch path; no compound guard is removed, folded or admitted.

Five current closed ADD effect candidates:
- Killing in the Name Of BossPoints: direct same-player same-track read minus1.
- Killing in the Name Of QuestLogEntry: two direct self-writes, ADD0.
- Oramond ToTakeRoots.Count: direct same-player same-track read minus5.
- What a Foolish Quest PieBuying: direct same-player same-track read minus1.
All already have from_exact=true. No local snapshot, non-player storage, clamp/random, conditional expression or Source guard is converted. The existing current Rust ADD operator performs checked addition and track bounds checks. This is a lowering of the effect within accepted native track/binding semantics, not a claim of complete donor callback equivalence. NPC/item/caller conditions remain the caller's responsibility; this packet introduces no runtime caller binding.

Integration, if independently approved: copy effect_refinements.py plus sibling schema; samples/state-effect-refinements/refinements.json. Call apply(root,quests) in quest_state_lowering.expected immediately before validate(quests), register module/schema/packet proof inputs as authoring_sources. Counts are computed AFTER refinements. This changes exactly five generated effects, leaves Source definitions/progress, track bounds, completion, comparisons and requested_by untouched. A required frozen packet rejects missing/tampered files. After these candidates: ADD138/COMPUTED163, 382 unique refused transitions. Never set from_exact from a partial guard.

Remaining work by category is recorded per transition in audit.json. Existing operator cannot evaluate compound Source predicates (221), randoms(2), clamps(14), conditional values(7), bitmasks(4), cross/dynamic expressions(5), dynamic fields(11), other helpers/composites(9). 47 affine-local and37 local-only cases need exact binding/read-version/mutation/call-interference proofs before using an existing operation. 25 writes use global/boss/target creature storage: they must not be remapped to player QuestState without an owning domain decision. One legacy helper needs its concrete binding/body proof; one row has differing donor category. Counts overlap only on the two dual-blocked transitions.

Reproduce Source audit locally without fetching: scan.py supplies exact original Source transitions and donor lines; builder.py --scan scan.json --state main-state.json --ast-root existing-cached-or-portable-AST --authoring tools/content-schema/quest-authoring --out audit.json. AST capture loader supports six portable archive shards, validates container digests and does not extract a mass corpus. Four tests PASS: full387 partition/actual5 changes, protected fields/guards/drift, direct arithmetic boundaries and foreign/local exclusion,679 proof fences.
