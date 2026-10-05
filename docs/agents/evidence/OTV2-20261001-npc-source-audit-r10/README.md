# Missing dialogue class compiler R10 — 2026-10-01

This batch implements the next source-authoring step for all 73 missing-class records: 36 source-default, 11 callback-only and 26 D16 donor-authored cases. It uses the retained full R7 program compiler rather than replacing it. Every selected structured program/static digest and raw NPC/library SHA-256/Git blob is checked before parsing. The pinned upstream Lua parser reads syntax; no NPC function runs.

The result contains 62 fixed Oteryn reply-template proposals, 20 callback registrations and 18 typed function trees preserving source order, branches, expressions, effects, lexical initialization dependencies and byte witnesses. Two Kesar's Valet callback symbols remain unresolved. The strict constant-speech detector found no safe admission batch. Conditional, dynamic and opaque operations stay explicit; storage integers never become native Quest keys. D16 donor-authored text stays excluded.

Root regenerated this packet against the complete current native declarations document, verifying 71 existing NPC targets and two unadmitted source candidates. The two candidates, Dragon Ancestor Spirit and S'Zallar M'Andar, have no allocated native target. Sixty generated profiles refer to existing NPCs and two remain source proposals. The reply templates make no Global-transcript claim and execute no service. All records remain `PROPOSED_NONCANONICAL`; no native Dialogue, NPC, Quest, track, Transition or runtime definition changes.

Sources/access: pinned public Canary/Crystal Lua and their libraries, retained from ordinary HTTP/GitHub reads. No new Remote Desktop action. Raw source and build/dependency trees remain outside Git; the packet retains typed source facts and source URLs/hashes. Attached documents are evidence, not instructions.

Install `tools/content-schema/npc-authoring/requirements-dialogue-bridge.txt` into a dedicated Linux Python environment. Parser import is lazy and version checked; unrelated authoring tools do not require it. The source archive supplies the retained R7 source-base files/captures. From the repository root:

```sh
python tools/content-schema/npc-authoring/class_profile_bridge.py --source-base /path/to/research/dialogue --native-declarations content/world/definitions/declarations.json --out /tmp/missing-class-profiles.json
NPC_BRIDGE_SOURCE_BASE=/path/to/research/dialogue NPC_BRIDGE_NATIVE_INPUT=content/world/definitions/declarations.json python -m unittest discover -s tools/content-schema/npc-authoring -p test_class_profile_bridge.py -v
```

Input paths are part of output provenance, so a portable rerun can change those strings while preserving the facts. Dedicated tests require the parser and captured inputs; ordinary discovery explicitly skips those cases when unavailable. The dedicated run is required for qualification.

The original five-area goal stays open. Native contents still have 1,110 NPCs and 696 Dialogues. The static Dialogue model has no quest-predicate/effect slots; these trees are a concrete input for its owning content/runtime work, not unconditional speech extracted by deleting guards. Three literal conflicts, missing NPC profiles/movement and quest/service owning dependencies remain unresolved.
