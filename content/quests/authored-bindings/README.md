# Explicit authored native bindings

Add closed schema-1 binding documents here, one owning quest per file where possible.
The quests build script registers regular lowercase ASCII JSON filenames. Native boot
combines the documents, verifies their exact canonical chosen recipe digests and data
holds, and attaches their bounded operations to the existing quest owners.

No target is inferred from a title, recipe prose, or a numeric donor storage. Use the
exact canonical Quest key and explicit NPC/item/creature/world selectors. Recipe hashes
use sorted compact UTF-8 JSON. Duplicate identities, stale recipes, foreign tracks and
unresolved data holds refuse boot. Empty registration preserves legacy quest hashes.

Binding registration is execution wiring, not whole-quest qualification. The owning
lane must prove every required acquisition stage through real-dispatch PostgreSQL cases.
Do not register synthetic engine fixtures as production quests.
