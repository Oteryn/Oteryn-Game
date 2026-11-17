# QUESTPC run 8 NPC migration evidence

Base HEAD: 1fa04bbbddb5d96e21fecca0bc456f3c5e1109b7; local uncommitted AUTHORING only.
Owned runtime titles: A Father's Burden, A Pirate's Tail, Bigfoot's Burden,
Hero of Rathleton, Hot Cuisine, In Service of Yalahar; unchanged Stag retained.
No new N..Z source program activation. Migration 1001 owns the sole Father
source acceptance exception; the ordinary eight-track cap remains eight.

DUR-04 classification: READ_COMPATIBLE_NORMALIZE. The exact catalogue before
NPC extension supplies each of the six legacy definition hashes and native
absent defaults. The active catalogue carries this bounded metadata separately
from its definition hash and serialization. Existing stored pins, explicit
track values and historical receipts are not rewritten. Unknown hashes refuse.
Existing native track initial values stay unchanged globally. NPC source
planning alone observes donor -1 for fresh absent tracks; a mapped old pin
observes its historical native initial0 before planning and locked reevaluation.
The native writer also evaluates mapped old absent defaults. Existing ranges
are widened, never narrowed, and old transitions remain byte-for-byte equal.

Forward proof: unit compares every retained original transition, initial and
range against the six original definitions. Exact old hashes accepted; wrong
hashes rejected. Reverse safety: an old catalogue has no adapter for new hashes,
so incompatible old-artifact rollback is refused. Operational rollback must
retain migration1001, the compatibility reader and its verified definitions;
this change does not authorize rolling back to an incompatible binary.

Fresh wiki fetch: helper command `python3 /home/mole/oteryn-cache/wikiapi.py
.questpc/r8-npc-wiki "A Father's Burden Quest" "Tereban" "In Service of Yalahar
Quest" "A Pirate's Tail Quest" "Bigfoot's Burden Quest" "Hero of Rathleton Quest"
"Hot Cuisine Quest"`; output `OK 7`. Source donor remains pinned
Crystal201189f871d23597b82caf88fd611454a2ddefb8. Its exact Tereban mission-accept
sets quest_log/status/eight material flags to1 and progress0, all eleven in one
operation; no split. Fresh wiki corroborates Tereban's four sons and old cape
quest, not the eleven native track representation, which remains donor evidence.

Red: `QPC_SLOT=2 /home/mole/oteryn-cache/codex-bin/cargo --locked --offline test
-p oteryn-gameplay-quests --lib storage_owner_commits_exact_father_acceptance --
--nocapture`, build23.65s, test0.10s, 0passed/1failed (missing Father plan).
Initial green: same cargo with `-p oteryn-gameplay-quests --lib npc_rules --
--nocapture`: 25passed/0failed, 1.19s. This preceded the native-initial0 refinement;
fresh final unit result is pending below.

Filtered PG tests added through real production NPC dispatch:
`catalogue_npc_father_fresh_acceptance_is_one_eleven_track_transaction` and
`catalogue_npc_father_legacy_hash_preserves_defaults_and_restart_receipts`.
Both use the existing quest-world scenario, independent database, real actor,
conversation, fenced revision owner, final-track injected failure and rollback,
then recovery/retry, exact eleven-track receipt and one Character revision.
The legacy case creates an actual old pin through the original native writer.
No POSTGRES_CASE_COMPLETED marker yet: first build blocked by concurrent Viola
module hook, before either test executed. Command: `QPC_SLOT=2
/home/mole/oteryn-cache/codex-bin/cargo --locked --offline test
-p oteryn-gameplay-dispatch --lib catalogue_npc_father_ -- --nocapture`.

Unfinished: await stable content and Viola hook; rerun final unit/filtered PG,
Clippy and formatter; legacy/fresh PG markers and durations not yet qualified.

Final source review refinement: legacy native track initial0 is retained globally;
source-only fresh absent facts remain donor-1. Added real native-writer assertions
after dispatch: mapped old-pin status1->2 succeeds with the same old pin; a fresh
new pin passed to the old artifact refuses REVISION_MISMATCH (reverse safety).
All field/constructor sites enumerated in r8-npc-constructor-sites.txt. Full owned
Rust files were read with the owner checklist; no new enum/trait methods, one
bounded metadata field with new/with_quest/loader constructors updated. Owned
files formatted using pinned-toolchain rustfmt with skip_children=true. Root is
responsible for final combined checks while other source modules settle.

Final pure unit validation on SOURCE_STABLE code: exact `npc_rules` command above,
25 passed / 0 failed / 234 filtered, 1.12s. Source default count assertion retained.

Stable-content PG attempt2 did not execute: compiler E0277 in concurrent
`durability/tutorial_viola.rs:136`, EquipmentError conversion; no PG marker.
The scoped non-Father case is now mounted via the existing scenario. It drives
Hero's greeting access write, Palimuth's ordered initialization, Hot Cuisine
apprenticeship three-track acceptance, Pirate's pre-access greeting/name, and
Bigfoot Xelvar's atomic refusal where four crystal items need another owner.
Exact pending filter: `catalogue_npc_scoped_a_m_starts_are_source_ordered_and_atomic`.
The Bigfoot refusal is qualification of safe refusal, not completion of that
item-grant branch or the quest.

Newly admitted six A..M programs contain 765 source rules and 156 track
declarations. These are parsed/execution-scope counts, not a claim that every
rule/effect/quest is qualified. The unchanged Stag adds99 rules/6 tracks.

Attempt3 after Viola repair compiled dependencies but dispatch lib-test rustc was
terminated by SIGTERM before any test ran. The lead observed the same simultaneous
termination in three worker builds and consolidated to one warm slot6 job.
Attempt4 slot2 was explicitly interrupted at lead request (exit130); it yielded
no test result or PostgreSQL qualification. Source is stable; no further worker
Cargo or source writes will occur until lead validation results.
Final root requested PG filters: `catalogue_npc_father_` (2 cases) and
`catalogue_npc_scoped_a_m_starts_are_source_ordered_and_atomic` (1 case).

Root's consolidated dispatch lib-test binary compiled and reached real PostgreSQL
on the final source. Father fresh, Father legacy and scoped cases then all failed
before scenario setup because migration1002 (another lane) declared unsupported
`TRUNCATE FOR EACH ROW` triggers at line381. There are still no successful NPC
POSTGRES_CASE_COMPLETED markers on run8; root owns the consolidated rerun after
that migration repair and final Clippy. No test was skipped or weakened.
Precision: fresh absence uses each source's declared initial:180 tracks=-1 and
50 Hot Cuisine tracks=0, rather than unconditionally -1.

Actual mechanic RED after migration1002 repair: root's consolidated real PG
freshFather and legacyFather fail at command2501 `hi`, and scoped case at2600
`hi`: disposition NoConversation. Root cause traced to npc_command.rs548:
NpcTalkBook has no entry for source-backed Tereban/StrangePipe. Exact actor,
placement(+0,+1), floor and helper match the qualified Stag route; the book's
literal Dialogue admission was hardcoded to six Stag keys only.

One whole-class Rust repair (no content changes): added a caller-scoped book
constructor retaining default admission unchanged. Production bundle_npcs and
the exact production-model fixture supply unique keys from the seven active
programs plus explicitly owned Viola. The existing literal greet/farewell are
read from the model; stateful keywords remain unadmitted, native dispatch owns
all writes. Viola owner receives the same constructor for its fixture.
New unit filter: `native_quest_book_admits_only_requested_source_greetings`.
Added legacy reverse proof: after new source and current-native writes, execute
the original artifact's original native quest_log transition; assert old hash
retained and all material values unchanged. Fresh incompatible rollback still
refuses RevisionMismatch. No own Cargo was run during root's active PG batch.

Same-batch sibling sweep found four Hero Greet callbacks whose source reply is
empty because the upstream NpcHandler emits MESSAGE_GREET after the callback
returns true. Verified pinned strange_pipe.lua59-63 and npc_handler.lua398-410
(read-only source copies r8-strange-pipe-source.lua, r8-npc-handler-source.lua).
Owned adapter now captures the already-computed ordinary literal greeting only
for Greet+empty source reply, retains it in CommitStep/PendingCommit, and emits
it only after successful commit/reconciled resume if the committed source reply
is still empty. No source-plan reply or source pin is edited. The scoped PG case
asserts the exact model greeting and replays the same Hero command occurrence
after reopening the conversation, retaining one Hero receipt/write.
New fields and all construction/pattern sites enumerated in r8-npc-fallback-sites.txt.

Public-surface sweep: pure `plan` is cfg(test) pub(super) for sibling source tests;
pure `plan_with_native` is cfg(test) private. Tests/native-resolution checks kept.
Remaining public APIs have production callers, including the new book constructor
in the server's bundle composition root. No dummy production calls added.
Latest source repair is SOURCE_STABLE; root runs the final qualification.

Root book-unit RED: native_quest_book_admits_only_requested_source_greetings
failed because one model has no literal farewell, while the existing native
book owner correctly uses its established "Farewell." fallback. The new test
incorrectly required Some(literal). Corrected its exact assertion to model's
first farewell or that existing fallback, retaining all exact greeting and
closed-admission assertions. Production behavior unchanged. Root reruns the unit
after its current PG compilation; no worker Cargo run.
