# Qualified Item capacity repair

Evidence classification: `PROVEN` source agreement, bounded native-field repair.
Owning contract: `GAME-ITEM-01_ITEM_MODEL_AND_EQUIPMENT_CONTRACT.md` §7.2;
closed native field: `ReferenceItemContainer.capacity: ReferenceItemField<u16>`.

The packet promotes 17 previously UNKNOWN capacities and corrects only Adventurer
Backpack (`oteryn:item.tibia.i53074`) from 20 to 22. Every promoted Item has an
EXACT Crystal identity binding, affirmative client 15.30 container flag, no existing
Terrain/WorldObject owner, and agreement among every retained wiki page presenting
volume. The client flag corroborates container identity; it does not encode capacity.
No Item becomes materializable, movable, pickupable, stackable or equippable here.

Adventurer Backpack has one retained Fandom page, revision 1198148, and corroborating
[Wiki BR revision 433126](https://www.tibiawiki.com.br/index.php?title=Adventurer_Backpack&oldid=433126),
both stating volume 22. Crystal revisions ff7ede5 and 00ce02a5 state 20 and remain
OTS hypotheses under the accepted wiki-first policy. The correction is fenced to
known 20 or already-corrected 22. The packet retains page and extraction hashes.

The baseline contains 27 agreed-wiki versus known-content capacity differences,
including Adventurer Backpack. Key Ring/Jewelled Backpack ID 5801 adds a distinct
inter-page identity conflict (volumes 20 and 22), explaining a union count of 28.
After this correction the packet holds 26 known-value conflicts, that one inter-page
conflict, three existing map owners and four records lacking affirmative client
container corroboration. All 34 holds retain item keys and source revisions.

Regeneration: `python tools/content-schema/item-authoring/lower_wiki_capacity_packet.py --check`.
Qualification tests: `python tools/content-schema/item-authoring/test_lower_wiki_capacity_packet.py`.
The native applier validates all rows and known-value preconditions before mutation;
rerunning it is idempotent. Absent source facts remain UNKNOWN.
