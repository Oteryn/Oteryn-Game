"""Record concrete shared-owner boundaries; never turn absent tests into blockers."""
import json
from pathlib import Path

ROOT = Path('.')
STATUS = ROOT / '.questpc7/QUEST-STATUS.json'
rows = json.loads(STATUS.read_text())
assert len(rows) == 33
assert {r['title'] for r in rows} == set(Path('.questpc7/MY-TITLES.txt').read_text().splitlines())

outfits = {
    'Feaster of Souls Quest', "Ferumbras' Ascension Quest", 'Festive Outfits Quest',
    'Fiend Slayer Outfits Quest', 'Fire-Fighter Outfits Quest', 'Formal Dress Outfits Quest',
    'Formorgar Mines Quest', 'Glooth Engineer Outfits Quest', 'Golden Outfits Quest',
    'Grave Danger Quest', 'Hand of the Inquisition Outfits Quest', 'Hunter Outfits Quest',
    'Illuminated Warrior Outfits Quest',
}
world_changes = {
    'Fire from the Earth Mini World Change', 'Fury Gates Mini World Change',
    'Grimvale Mini World Change', 'Hive Born World Change',
    'Hive Outpost Mini World Change', 'Horse Station World Change',
}
p23 = {'Falconer Outfits Quest', 'Feral Trappers Quest'}

# These are existing interfaces, not invented names for missing per-quest handlers.
OUTFIT = 'crates/gameplay/durability/src/durability/character_appearance/quest_grants.rs:11'
NPC = 'crates/gameplay/quests/src/quest/npc_rules/evaluate.rs:350'
WRITER = 'crates/gameplay/durability/src/durability/quest_state/npc_program.rs:113'
CALLBACK = 'crates/gameplay/quests/src/quest/world_callbacks.rs:98'
WORLD = 'crates/gameplay/world-change/src/content.rs:117'

specific = {
    'Exchange Quests': (
        'qengine2.atomic_npc_exchange',
        'The Sweaty Cyclops/Marina/Quentin exchanges require consumption of the exact carried inputs, '
        'output minting and quest progress under one receipt and Character fence. The generic NPC '
        'writer accepts storage effects only; its sole inventory exception is the separately pinned '
        "Father's Burden programme. This missing shared transaction owner is assigned to qengine2, "
        'and must not be copied into a questpc7 NPC handler.',
        [NPC, WRITER],
        'Traced plan_storage and the Father-only plan_father_items exception; neither can commit '
        'Bast Skirts, Honeycombs or the Blessed Wooden Stake exchange.'),
    'Forgotten Knowledge Quest': (
        'qengine.encounter_party_access',
        'The Lloyd/Lady Tenebris/Dragon King/Time Guardian/Last Lore Keeper path requires shared '
        'encounter admission and a frozen participant death-settlement owner before powerful '
        'imbuing access can be earned. The world callback interface has a fixed player teleport '
        'but no encounter admission/participant capture operation. That shared engine is assigned '
        'to qengine; a local NPC response or a reward claim cannot substitute for it.',
        [CALLBACK, 'crates/gameplay/dispatch/src/gameplay_transport/kill_reward.rs:92'],
        'Inspected the selected boss stages, current callback operation shape and frozen kill '
        'entry; selected all eight exact reward claims for independent real-USE qualification.'),
    'Formorgar Mines Hoist Quest': (
        'qengine.paired_portal_and_remote_lever_predicate',
        'UID3060 chooses floor9 or floor11 by inspecting the separate lever at '
        '(32156,31125,10); UID3059/3061 go to floor10. Both instruction flags must be checked '
        'without writing progress on each trip. Callback.teleport is one fixed Position, and '
        'read_only storage callbacks explicitly reject StepIn. Implementing this in an NPC '
        'module cannot supply the required movement ingress and remote-object predicate; '
        'paired portal/shared world callback work belongs to qengine.',
        [CALLBACK, 'crates/gameplay/quests/src/quest/world_callbacks.rs:178'],
        'Read both exact donor movements_hoist.lua bodies from the pinned corpus and fresh Wiki '
        'instructions; checked the closed callback parser rather than promoting a fixed-destination '
        'approximation. Tested the two existing instruction-book claims separately.'),
    'Giant Smithhammer Quest': (
        'qengine.asymmetric_lever',
        'UID30024 must remove rock389 only when lever2772 and the remote rock coexist, and '
        'recreate it only when lever2773 and the rock is absent. In the other two combinations '
        'it leaves both unchanged. The current callback always toggles its own lever before '
        'preparing the configured passage; it has no remote-presence branch/no-op predicate. '
        'The owner assigned asymmetric levers to qengine. Reward receipts cannot prove this '
        'required source callback.',
        ['crates/gameplay/dispatch/src/gameplay_transport/quest_callbacks/execute.rs:62', CALLBACK],
        'Extracted both donor lever.lua bodies; compared all four lever/rock states to the '
        'current executor. Found the three reward claims under Explorer Society and added '
        'their exact source-position/item/quantity PG cohort; retained the accepted PR2770 '
        'Q5/A Item12510 substitute rather than overriding that decision.'),
    'Goblin Merchant Quest': (
        'qengine.creature_capture_and_pickup',
        'The Guardcatcher/Adventurer stage consumes a carried tool while capturing/removing '
        'the exact live creature and delivering its captured form; related Rafzan tasks also '
        'change world objects. ItemOnTarget exposes one item string, count and replacements, '
        'and its target is a map object, with no transactional captured-creature/pickup '
        'operation. The shared pickup/reset/world-operation owner is assigned to qengine; '
        'a questpc7 NPC reply cannot authorize an actor despawn.',
        ['crates/gameplay/quests/src/quest/world_callbacks.rs:81', CALLBACK],
        'Read the chosen Guardcatcher/Adventurer and perfume/trunk stages; compared the '
        'single-carried-item map callback schema with the required actor capture operation.'),
    'Grimvale Quest': (
        'qengine.encounter_party_access',
        'Feroxa and the subsequent named were-creature bosses require encounter admission, '
        'frozen participants and repeat/reset-safe death settlement. The ordinary callback '
        'schema has no encounter/party entry operation. Those shared operations are assigned '
        'to qengine; completing local dialogue or counting unrelated ordinary Werewolf deaths '
        'cannot qualify the encounter path.',
        [CALLBACK, 'crates/gameplay/dispatch/src/gameplay_transport/kill_reward.rs:92'],
        'Checked the Liquid Silver Trap, Feroxa, four named bosses and ritual stages against '
        'the current fixed teleport and frozen kill-entry interfaces.'),
    'Heart of Destruction Quest': (
        'qengine.encounter_party_access_and_room_death',
        'The source boss-room death/vortex effects require encounter admission and the room '
        'participant capture/settlement adapter. The exact native-death packet is explicitly '
        'runtime_enabled=false and names the missing capture/settle functions. Re-enabling '
        'its JSON cannot provide that shared engine, and the owner assigns encounter/party '
        'access to qengine.',
        ['content/quests/native-death-bindings/quest-batch3-c-heart_of_destruction.json:1',
         'content/quests/heart-of-destruction-deaths.json:1', CALLBACK],
        'Read all six exact room-death rules and their scope block; traced the existing '
        'kill-settlement interface and checked that no production native-death consumer '
        'loads this packet. Kept runtime_enabled=false.'),
    'Hero of Rathleton Quest': (
        'qengine.encounter_party_access',
        "Jaccus Maxxen's dungeon path needs the shared encounter/party admission and boss "
        'settlement owner before the blueprint/bag/access rewards can be earned. Literal '
        'Strange Pipe greetings and the existing stateless terrain callbacks carry no '
        'participant or encounter admission operation. That engine is allocated to qengine; '
        'the earlier greeting receipt is retained only as a partial slice.',
        ['content/quests/world-callbacks-held/batch3-b-hero_of_rathleton.json:1', CALLBACK],
        'Read the full selected item/combat/dungeon stages and the held world callback '
        'packet; distinguished its literal callbacks and retained greeting proof from '
        'the shared encounter admission operation.'),
    'Hidden Threats Quest': (
        'qengine2.atomic_multi_item_exchange',
        'The selected mine/key route must consume the correct fragments/tools and commit '
        'the resulting quest progress/output atomically. The existing item-on-world callback '
        'accepts only a single item kind and the generic NPC programme accepts storage '
        'effects only. The shared multi-input hand-in/progress owner is assigned to '
        'qengine2; duplicating its item transaction here is forbidden.',
        ['crates/gameplay/quests/src/quest/world_callbacks.rs:81', NPC, WRITER],
        'Compared the key-fragment/metal-file source rewards and full selected mine route '
        'with both current transaction interfaces; selected all three source rewards '
        'for independent USE/replay/refusal qualification.'),
    'Hot Cuisine Quest': (
        'qengine2.atomic_npc_exchange',
        'Each Jean Pierre cooking operation must consume all exact ingredient quantities '
        'and fluid subtypes, deliver the dish and commit cooking progress under one '
        'receipt/fence. The generic NPC writer only executes storage effects and the '
        'full cooking planner is mounted under cfg(test). This required transaction '
        'owner is assigned to qengine2. Calendar/bonus facts and delayed outside-August '
        'farewell must still be composed with that owner; fixed month1 or August inputs '
        'are not production calendar evidence.',
        [NPC, WRITER, 'crates/gameplay/quests/src/quest/mod.rs:21',
         'crates/gameplay/quests/src/quest/batch3_a/hot_cuisine.rs:148'],
        'Read the source recipe/bonus calendar planner and the real generic writer; '
        'traced the retained Jean Pierre hi rejection to unsupported CalendarMonth '
        'rather than interpreting it as an absent test. Did not duplicate qengine2 '
        'or activate the test-only cooking planner.'),
    'Hunting for Tokens Quest': (
        'qengine2.atomic_npc_exchange',
        'Jorge must atomically consume exactly twenty Silver Raid Tokens and deliver '
        'one chosen Abacus replica with the quest receipt. The generic NPC writer '
        'has no item consumption/output-mint transaction for Jorge; its sole item '
        "exception is Father's Burden. This shared transaction is allocated to "
        'qengine2, so a local response or independent MINT cannot safely finish it.',
        [NPC, WRITER],
        'Read the exact twenty-token/one-Abacus recipe and item-resolution hold; '
        'traced the generic writer and its Father-only exception before considering '
        'a lane NPC hook. No independent consume-then-mint transaction was introduced.'),
    'Hydra Egg Quest': (
        'qengine.reset_pickup',
        "The nest needs a native quest pickup/replenishment owner for Hydra Egg, not "
        'an invented UID reward chest. The source catalogue has no reward-claim '
        'placement for this nest, Item4839 is not admitted as materializable, and '
        'the existing map callback has no arbitrary pickup/item-output operation '
        '(only the separately closed Monk reward). The owner assigns reset/pickup '
        'to qengine. An NPC hook cannot supply the missing map pickup lifecycle.',
        [CALLBACK, 'crates/gameplay/quests/src/quest/world_callbacks.rs:200',
         'content/items/definitions/items-26500-26999.json:1#/records/465/definition'],
        'Read the fresh Wiki route (explicitly permits ignoring the Hydra), searched '
        'all reward-claim shards and both pinned donor trees for the nest owner, '
        'and checked Item4839 native admission. Did not fabricate a selector or '
        'use a test-only mint as the quest reward.'),
}

for r in rows:
    title = r['title']
    if title in outfits:
        path, rest = r['definition'].split(':1#/records/')
        index = int(rest.split('/')[0])
        definition = json.loads(Path(path).read_text())['records'][index]['definition']
        recipe = definition.get('recipe', definition.get('oteryn_recipe', {}).get('payload', {}).get('recipe', {}))
        rewards = ', '.join(v['name'] for v in recipe['reward_intents'] if v['kind'] == 'outfit')
        dep = 'qengine2.quest_outfit_and_addon_grant'
        missing = (f'{rewards}: missing the shared receipt-bearing quest outfit/addon grant '
                   'owner and the mapping consumed by account appearance selection. The existing '
                   'QuestOutfitGrant is closed to Revenant base and DecayingDefender base, with '
                   'no addon grant. A local NPC branch cannot create those entitlements or '
                   'extend their selection provenance. This owner is explicitly allocated to '
                   'qengine2; do not implement a second appearance/grant owner in this lane.')
        refs = [OUTFIT, NPC]
        attempt = ('Read every selected outfit milestone and its source/recipe references; '
                   'traced QuestOutfitGrant and its production callers plus the generic NPC '
                   'storage-only evaluator. Kept reward-claim evidence separate from outfit ownership.')
        kind = 'shared_engine_dependency'
    elif title in world_changes:
        dep = 'W14'
        kind = 'owner_exclusion'
        missing = ('Owner-excluded W14 world-change activation: this title requires the native '
                   'world-change layer/spawn/NPC/raid binding and activation/reset lifecycle, '
                   'not only selection-source compilation. require_w1_bindings currently '
                   'returns WorldChangeBindingsUnavailable. The current task explicitly '
                   'allows W14 as an owner exclusion. The personal chosen recipe remains '
                   'unimplemented; it is not promoted by excluding the original world service.')
        refs = [WORLD, 'docs/agents/tasks/EVENTS-WORLD-1.md:1']
        attempt = ('Read the exact title recipe and original source world-change scope; '
                   'traced the typed W1 boundary and its six explicit binding gaps. '
                   'Did not activate selection-only data or claim the personal adaptation runs.')
    elif title in p23:
        dep, kind = 'P23', 'owner_exclusion'
        missing = ('Owner-excluded P23 Prey/Hunting Tasks: the chosen path starts at the '
                   'Task Board and redeems its Hunting Task Shop outfit/addon offers. '
                   'The owner exclusions apply even though the task-board catalogue '
                   'already lists those cosmetics; shop metadata is not a quest completion.')
        refs = ['crates/gameplay/quests/src/quest/task_board_catalogue.rs:39',
                '.questpc7/OWNER-SCOPE-ALL.md:4']
        attempt = ('Read the Task Board/kill/shop stages and found the exact Falconer/Feral '
                   'Trapper offers in the existing catalogue; applied the explicit P23 '
                   'owner exclusion rather than requalifying that other mechanic.')
    else:
        dep, missing, refs, attempt = specific[title]
        kind = 'shared_engine_dependency'
    r['status'] = 'BLOCKED'
    r['scope'] = 'complete pinned quest path'
    r['blocker'] = {'kind': kind, 'dependency': dep, 'missing_piece': missing,
                    'evidence': [r['definition'], *refs],
                    'attempts': [attempt],
                    'boundary': ('Current user reserves the shared engine to qengine/qengine2; '
                                 'the lane may not build a duplicate owner.' if kind == 'shared_engine_dependency'
                                 else 'Current user explicitly permits this named owner exclusion.')}
    for ref in r['blocker']['evidence']:
        path, line = ref.split(':', 1)
        assert Path(path).is_file(), ref
        assert int(line.split('#')[0]) <= len(Path(path).read_text().splitlines()), ref
assert len(outfits | world_changes | p23 | set(specific)) == 33
STATUS.write_text(json.dumps(rows, indent=1) + '\n')
print('33 final dispositions; no absent-test blockers; source reward evidence pending PG results')
