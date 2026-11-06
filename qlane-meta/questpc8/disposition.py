"""Record only concrete shared-engine boundaries; preserve qualified evidence."""
import json
from pathlib import Path

root = Path('.')
status = Path('.questpc8/QUEST-STATUS.json')
rows = json.loads(status.read_text())

def at(path, needle):
    lines = Path(path).read_text().splitlines()
    return f'{path}:{next(i for i, line in enumerate(lines, 1) if needle in line)}'

appearance = at('crates/gameplay/durability/src/durability/character_appearance/quest_grants.rs', 'enum QuestOutfitGrant')
program = at('crates/gameplay/durability/src/durability/quest_state/npc_program.rs', 'source.plan_storage(&input)')
refusal = at('crates/gameplay/quests/src/quest/npc_rules/evaluate.rs', 'return Err(NpcRuleError::MissingNativeResolution')
father = at('crates/gameplay/durability/src/durability/quest_state/npc_program/fathers_burden.rs', 'NpcRuleEffect::TakeItem')
callback = at('crates/gameplay/quests/src/quest/world_callbacks.rs', 'pub struct Callback')
consume = at('crates/gameplay/quests/src/quest/world_callbacks.rs', 'item.count == 0')
lever = at('crates/gameplay/dispatch/src/gameplay_transport/quest_callbacks/execute.rs', 'if close && !callback.reversible')
occupancy = at('crates/gameplay/dispatch/src/gameplay_transport/quest_callbacks/execute.rs', 'map_door::doorway_occupied')
limit = at('crates/gameplay/quests/src/quest/world_callbacks.rs', 'pub const MAX_EFFECTS')
binder = at('crates/gameplay/content/src/content/world_reward_claims.rs', '.ok_or(UnboundReason::NoCrystalUniqueId)')
world = at('crates/gameplay/world-change/src/content.rs', 'pub fn require_w1_bindings')

outfits = {
'Illuminator Outfit Quest': 'Illuminator base outfit from the archive display',
'Illuminator Outfits Quest': 'Illuminator addons from Scallop Shell and Sand-Filled Horn',
'Insectoid Outfits Quest': 'Insectoid base/addons at cumulative War Exp 300/500/1000',
'Knight Outfits Quest': 'Knight outfit/addon ownership beyond the Ramsay helmet reward',
'Mage Outfits Quest': 'Mage potion-belt addon, including the Winning Lottery Ticket hand-in',
'Makeshift Warrior Outfits Quest': 'Makeshift Warrior base/addons from planks and Tinged Pot',
'Measuring Tibia Quest': 'Discoverer base/addons after 10/15/20 distinct regional surveys',
'Liquid Black Quest': 'Deepling outfit at the memorial after presenting Lagatos book',
}
exchanges = {
'Into The Bone Pit Quest': 'Muriel must consume the recovered bone and commit the Bone Pit quest/XP outcome atomically',
'Loose Ends Quest': 'the pinned chosen Tibia Tales anthology includes Muriel bone hand-in, holy-water supply and Boozer ale exchange before final6500XP',
'Kissing a Pig Quest': 'Budrik consumes five picks for tunnel access; Elvith/Lynda also consume the poem/ice cream before lesson progress',
'Krailos Quest': 'the camp menu hand-ins consume food/teeth/Onyx Chip while advancing the matching quest tracks',
'Marlin Trophy Quest': 'Pemaret must consume one Marlin and mint its trophy atomically through the retained NPC source dialogue; the chosen Goroma stand is not that NPC source obligation',
'Koshei the Deathless Quest': 'four separately claimed amulet fragments must be consumed together and yield the assembled amulet before the timed Koshei seal',
'Kilmaresh Quest': 'Yonan must consume the four Regalia of Suon pieces and mint the assembled regalia atomically with quest progress',
}
worlds = {'Insectoid Invasion World Change', 'Jungle Camp Mini World Change', 'Kingsday Mini World Change', 'Lumberjack Mini World Change'}
authored = {'Kingdom of Kormarak Quest': 'dead-human reward (Brass Armor, Brass Helmet, Hatchet,13 Throwing Stars)', 'Magic Sword Quest': 'two southern Mintwallin vault rewards, with the chosen once-only sword/armour/wand allocation', 'Mastermind Potion Quest': 'two personal Formorgar caches and chosen consolidated201-gold reward', 'Mystery of the Valley Quest': 'three distinct shrine fragments and final Guardian Statue reward'}
extras = {
'Life Ring Quest': ('qengine', 'The drawbridge must relocate all occupants when its six tiles close. The shared callback closes passages only when unoccupied and teleports only the triggering player. Implementing a private quest callback cannot add the missing multi-actor sealed movement/map commit. This relocation/reset feature is expressly reserved to qengine.', [callback, occupancy, '.questpc8/wiki/Life Ring Quest_Spoiler.wiki:16']),
'Mintwallin Cyclops Quest': ('qengine', 'The one-shot source lever removes itself, creates two blocking walls and removes nine others until floor reset. Callback has a two-appearance lever, a symmetric close flag and MAX_EFFECTS8; neither self-removal/reset nor the11 map effects fit. This asymmetric lever/reset engine is expressly reserved to qengine.', [callback, lever, limit, '.questpc8/wiki/Mintwallin Cyclops Quest_Spoiler.wiki:27']),
'Jack to the Future Quest': ('qengine', 'Placing five used furniture kits must change carried kits into world objects, retain each placement and support the source pickup/reset path. Callback can consume one applied source item and transform its clicked target, but has no inventory-to-world furniture placement/pickup transaction. Pickup/reset is expressly reserved to qengine.', [callback, consume]),
'In Service of Yalahar Quest': ('shared-world-callback-owner', 'Clearing the four sewer pipes applies a reusable Crowbar without consuming or replacing its item identity. ItemOnTarget requires count>0 and the current durable source-tool path consumes/transforms that item; a count0 callback is rejected. Reusable-tool proof plus target mutation needs the common callback transaction, not an NPC routing hook. Later Azerus encounter access is also reserved to qengine.', [callback, consume]),
'Machinery of War Quest': ('qengine', 'Burning the ballistae/catapults/siege wheels with the reusable Fire Bug requires resetting world-object state and non-consuming tool application. Current ItemOnTarget requires positive consumption and Callback has no reset epoch/timer; count0 is rejected. Reset/pickup world owner is expressly reserved to qengine.', [callback, consume]),
'Last Creep Standing': ('qengine', 'Eight Shapechanger forms must grant only their matching portal admission and clear/restore temporary disguise on leaving the trial. Callback has no observed temporary-disguise predicate or paired encounter-return state. NPC storage transitions and permanent outfit unlocks cannot supply those facts. Paired portals and encounter access are expressly reserved to qengine.', [callback, appearance]),
'Make Believe Quest': ('qengine', 'Mimar Haffar needs short-lived crystal/cannon charge cycles and shield windows; Maior Domus needs vocation-specific charge windows; Phosphorus requires eight simultaneous petrified adds and persistent arena hazards. Current callback effects have neither encounter lifecycle nor timed multi-actor state authority. Encounter/party access is expressly reserved to qengine; no boss-victory storage is fabricated.', [callback]),
"Lion's Rock Quest": ('qengine', 'The source treasure route has timed/resettable trial objects and the Lion Heart cache. Existing entry-refusal test proves refusal at Questline0 only. Callback has no reset/pickup epoch owner, and a native authored reward cannot borrow a Crystal UID. Reset/pickup is expressly reserved to qengine; the native placement binder also needs a shared contract.', [callback, binder, 'crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/quest_world_callbacks/lions_rock.rs:1']),
}
for row in rows:
    title = row['title']
    if row['status'] == 'QUALIFIED':
        continue
    if title in outfits:
        owner = 'qengine2'
        piece = f'Missing durable outfit/addon grant owner for {outfits[title]}. QuestOutfitGrant currently admits only Revenant and DecayingDefender; the generic NPC durable executor uses plan_storage and refuses outfit effects. The owner expressly reserves outfit/addon grants to qengine2; a lane NPC module cannot modify that account-entitlement transaction or introduce a second grant owner.'
        evidence = [appearance, program, refusal]
    elif title in exchanges:
        owner = 'qengine2'
        piece = f'Missing atomic item hand-in/output plus quest-progress owner: {exchanges[title]}. Generic NPC source execution is storage-only; its one native item exception is the closed Tereban/Fathers Burden handler, not a general exchange. This shared engine is expressly reserved to qengine2; separate item and progress writes would break rollback/replay invariants.'
        evidence = [program, refusal, father]
    elif title == 'Killing in the Name of... Quest':
        owner = 'shared-chosen-NPC-reward-owner / qengine2'
        piece = 'Missing chosen-recipe NPC reward policy and atomic payout owner. The pinned overlay explicitly selects10000XP and5000GoldCoin, whereas the closed native Grizzly source catalogue pays800/2500/30000XP per selected task and no such gold grant. Its source-native progression test cannot certify those chosen payouts. A lane routing hook cannot override the source-pinned task XP contract or commit a separate unreceipted gold/completion write: the generic native NPC executor uses plan_storage and refuses GiveItem/GiveMoney/Experience effects; its only item exception is closed to Tereban. This requires the shared authored/native NPC reward transaction and policy binding, not another private grant owner.'
        evidence = [program, refusal, at('crates/gameplay/quests/src/quest/grizzly_adams/catalogue.rs', 'let experience = match action'), at('crates/gameplay/quests/src/quest/npc_rules.rs', 'pub fn plan_father_items'), '.questpc8/definitions/Killing in the Name of... Quest.json:1']
    elif title in worlds:
        owner = 'W14 owner exclusion'
        piece = 'W14 owner exclusion: no executable W1 world-change layer/activation/reset bindings. The selection-source compiler cannot produce them: require_w1_bindings returns WorldChangeBindingsUnavailable for validated candidates. A personal quest flag must not impersonate the world-wide activation/geometry/NPC/spawn owner. W14 is explicitly an allowed owner exclusion in the current task.'
        evidence = [world, 'docs/agents/tasks/EVENTS-WORLD-1.md:307']
    elif title in authored:
        owner = 'shared-native-placement-owner'
        piece = f'Missing native authored RewardClaim placement authority for the {authored[title]}. The shared binder requires a crystalserver legacy unique ID and rejects native authored selectors with NoCrystalUniqueId. The selected authored recipe has no donor claim that can legally be reused for these chosen rewards. A lane-only NPC/callback module cannot change the trusted map/RewardClaim binding contract and boot consumer; assigning a Crystal ID would counterfeit source provenance. Current regression test exercises that rejection on the actual binder.'
        evidence = [binder, 'crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/questpc8_reward_claims.rs:16']
    else:
        owner, piece, evidence = extras[title]
    row['status'] = 'BLOCKED'
    old = row.get('blocker', {})
    row['blocker'] = {'missing_piece': piece, 'owner': owner, 'evidence': [row['definition'], *evidence], 'attempts': [
        'Reopened the inherited no-test/not-composed row; read this titles complete pinned chosen recipe and retained source obligations.',
        'Traced the real production consumer and its durable/map authority, including existing native special-case adapters; no test-only planner was treated as a production owner.',
        'Checked whether a lane-only module could supply the required fact atomically. The cited shared transaction/type boundary or explicit owner exclusion prevents that route; no equivalent shared owner was copied and no guard was weakened.',
    ], 'recheck': f'Reopen when {owner} supplies the cited engine/contract; then compose the quest and run filtered real-dispatch PG. This row is not qualification.'}
status.write_text(json.dumps(rows, indent=1, ensure_ascii=False) + '\n')
