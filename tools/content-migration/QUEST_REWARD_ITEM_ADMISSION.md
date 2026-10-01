# Quest Item core data admission

`quest_reward_item_semantics.py` is applied by normal `world_project_v2_to_tree.py`
generation and by its equivalence validator. It overlays protected legacy data;
no `--resolve`, external source checkout, network or wiki cache is needed. The
existing migration CI runs its focused regressions and canonical `--check`.

Two independently scoped evidence packets are applied together:

- `OTERYN_QUEST_REWARD_ITEM_CORE_ADMISSION/v1`: all 338 distinct reward Item IDs
  from all 336 source RewardClaims, with a digest-bound admitted 15.30 client,
  three exact pinned donor XML rows each and 314 exact-ID Fandom revision captures.
- `OTERYN_QUEST_INTERACTION_ITEM_CORE_ADMISSION/v1`: 187 additional exact portable
  client/XML candidates. Its `OTERYN_QUEST_ITEM_ROLE_INVENTORY/v1` records 228 IDs
  from hand_out, consume, world CREATE, Item-count conditions and literal calls in
  curated NPC source files. 202 IDs are outside the first reward packet; 15 remain
  source-only held and 117 computed call arguments remain UNKNOWN.

The packet namespaces differ from `OTERYN_ITEM_ADMISSION/v1`, which belongs to
StarterKit admission. The helper rejects overlapping first/extra Item scopes.
Inventory roles describe observed source references; a call elsewhere in a curated
NPC file does not establish a mandatory quest requirement or a branch prerequisite.
Namesake, family and disambiguation wiki pages do not supply facts without an exact
literal Infobox Object Item ID. Raw wiki revision content, timestamp and hash are
bound to each used observation. There are 160 exact-ID extra captures and 27
explicit missing matches.

The client's `take` proves portability. Omitted `unmove` and `cumulative` use the
pinned Canary proto2 getter interpretation; their false values are DERIVED rather
than explicitly observed. B3 section4.3 supplies stack maximum100. Unknown equipment,
use effects and instance text/fluid/charges remain unknown. A definition admission
does not prove that today's claim representation reproduces its source subtype.

An exact owning donor row may prove identity when another pinned donor lacks that
ID. Missing rows remain UNKNOWN; source disagreement is never resolved by a donor
majority. Extra i53074 has Crystal/summer XML capacity20 and exact wiki capacity22:
its capacity and new materialization remain held. Extra i25302 is stackable and
not a container in the client while XML declares capacity5: retain the conflict.
Extra i901 has accepted NonStackable native core versus StackCapable client/wiki;
the complete accepted definition is preserved with its digest and an explicit hold.

The first packet keeps i235's client/XML container conflict, charged nonstackable
and fluid instance holds. Existing i3048/i3081 admissions remain intact; the source
claims independently retain subtype/charge holds. Four extra nonconflicting
containers add capacities20,24,32,32. No runtime qualification is promoted.

```sh
python tools/content-migration/test_quest_reward_item_semantics.py
python tools/content-migration/quest_reward_item_semantics.py --check
python tools/content-migration/world_project_v2_to_tree.py
python tools/content-migration/validate_world_project_v2_to_tree.py
```

Regenerate RewardClaim/Quest readiness after integration. Their remaining gaps also
include source placement, instance-state and engine contracts outside this Item core.
