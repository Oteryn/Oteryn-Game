"""Concrete shared-owner dependencies; never substitute a missing-test blocker."""
from pathlib import Path
import json

ROOT = Path('.')
WARDROBE = 'crates/gameplay/durability/src/durability/character_appearance/quest_grants.rs:11'
NPC = 'crates/gameplay/quests/src/quest/npc_rules/evaluate.rs:351'
CALLBACK = 'crates/gameplay/quests/src/quest/world_callbacks.rs:24'
MOVEMENT = 'crates/gameplay/dispatch/src/gameplay_transport/paced_movement.rs:212'
SOURCE = '.questpc6/NOTE-coordination.md:16'

def shared(piece, owner, *evidence):
    return dict(missing_piece=piece, reserved_engine_owner=owner, evidence=list(evidence), attempts=[])

BLOCKERS = {
    'Chakoya Iceberg Mini World Change': shared(
        'Personal iceberg encounter/access owner: the chosen fixed expedition needs a Character-scoped route and credited Chakoya encounter occurrences. The current callback schema has no encounter/access scope or spawn lifecycle. An NPC-only module cannot supply that shared map/combat authority; qengine owns encounter/access.',
        'qengine', CALLBACK, SOURCE),
    'Child of Destiny Quest': shared(
        'Training encounter owner for Party Skeleton combat, spear-target hit and spell-target hit events. USE/StepIn callbacks cannot observe those authoritative combat events or close a training encounter; qengine owns encounter/access. NPC talk progress alone would bypass the three training exercises.',
        'qengine', CALLBACK, SOURCE),
    'Chyllfroest Mini World Change': shared(
        'Stateful thaw/reset/pickup owner for the frozen Ursagrodon object and Tinder Box interaction. Existing taming rules have a single retained success/failure outcome, no staged thaw/reset state, and no Ursagrodon rule. A direct mount grant would bypass the frozen-object interaction. qengine owns reset/pickup.',
        'qengine', 'crates/gameplay/content/src/content/native_gameplay/mount_taming.rs:16', SOURCE),
    'Cults of Tibia Quest': shared(
        'Encounter/party access and summon lifecycle for the cult boss rooms and Essence of Malice completion. Orc entry/exit travel and source chests can execute; the callback schema cannot own boss participants, spawned entities, room cleanup or encounter completion. These are shared qengine authority, not missing tests.',
        'qengine', CALLBACK, SOURCE),
    'Dangerous Depths Quest': shared(
        'Encounter/party access owner for Warzone IV/V/VI entry, boss summon/cleanup and successful-room completion. Neither a personal quest track nor a literal USE callback supplies group admission and authoritative encounter lifecycle; qengine owns this shared piece.',
        'qengine', CALLBACK, SOURCE),
    'Deeplings World Change': shared(
        'Fiehonja encounter/access stage owner: the three served boss teleports only READ the crystal track at13; they do not own the world stage, boss encounter or stage reset. The shared encounter/reset owner is reserved to qengine. Writing13 directly would bypass the world change.',
        'qengine', CALLBACK, SOURCE, 'content/quests/world-callbacks.json:1'),
    'Demon Helmet Quest': shared(
        'Paired pressure-plate StepIn/StepOut owner for the Gate of the Lost Souls. Callback Trigger has no StepOut; a sequential track flag cannot preserve the original physical plate restoration. qengine owns this shared piece. All three reward claims are separately exercised.',
        'qengine', CALLBACK, MOVEMENT, SOURCE),
    'Demon Wars World Change': shared(
        'Encounter/access and faction-war reset owner for Shaburak Demon/Lord/Prince progression. A quest NPC module cannot establish the shared combat occurrence and faction encounter scope. The callback schema has no encounter lifecycle; qengine owns encounter/access and reset.',
        'qengine', CALLBACK, SOURCE),
    "Demon's Lullaby": shared(
        'Atomic carried-item hand-in and quest progression for the infant feeding requests, plus the infant item lifecycle. Storage-only NPC execution rejects item effects, and the sole native item exception is Tereban. qengine2 owns the general hand-in owner; do not perform separate item and quest writes.',
        'qengine2', NPC, 'crates/gameplay/quests/src/quest/npc_rules.rs:472'),
    "Devovorga's Essence Mini World Change": shared(
        'Encounter/party access owner for the Vengoth incarnation offering, Anmothra spawn, participant admission and terminal boss cleanup. Static callbacks cannot own the incarnation encounter lifecycle; qengine owns encounter/access.',
        'qengine', CALLBACK, SOURCE),
    'Down the Drain Mini World Change': shared(
        'Flooded-island encounter/access and reset owner for the Water Elemental dungeon. The accepted personal route still needs authoritative entry and encounter completion, while current callbacks only process individual map interactions. qengine owns encounter/access and reset.',
        'qengine', CALLBACK, SOURCE),
    'Draconia Quest': shared(
        'Atomic NPC Dragon Fetish item exchange/progress (An Old Dragonlord), and a source exit that conditionally consumes the offering/resets DragonFetish before choosing success/failure teleport. Storage-only NPC execution rejects the item effects; qengine2 owns hand-in. The two independent reversible levers and every claim are exercised separately.',
        'qengine2', NPC, 'docs/agents/evidence/quests-2-20261102/draconia.json:1'),
    'Devil Helmet Quest': shared(
        'StepOut paired with StepIn on plate uid25030, including creation/removal of435 on the other tile. Trigger only admits Use/StepIn, and passage effects operate on existing objects. qengine owns StepOut/reset. Reward/key receipts do not supply that pressure-plate authority.',
        'qengine', CALLBACK, MOVEMENT, SOURCE, 'docs/agents/evidence/quests-2-20261102/devil_helmet.json:1'),
    "Dreamer's Challenge Quest": shared(
        'Shared reset/pickup and conditional puzzle-world owner for the carrot/Fire Bug/Death Ring entrance and later plate/maze state. The seven stateless exits execute, but a teleport-only callback cannot atomically validate and consume the puzzle offerings, reset the physical puzzle and choose failure recovery. qengine owns reset/pickup; item exchanges additionally need qengine2.',
        'qengine', CALLBACK, SOURCE),
    'Elemental Spheres Quest': shared(
        'Encounter/party access owner for the four sphere participants, machines and Lord of the Elements summon/cleanup. Existing lord-room exit only resets four storage tracks; it cannot establish participant admission or own the boss lifecycle. qengine owns encounter/access.',
        'qengine', CALLBACK, SOURCE, 'content/quests/world-callbacks.json:1'),
    'Elvenbane Quest': shared(
        'Encounter/access owner for the mandatory Dragon and five upper-floor defenders before terminal loot. Literal callbacks cannot prove combat completion or own the chosen personal encounter route; qengine owns encounter/access. Giving terminal loot from an NPC would bypass the required defenders.',
        'qengine', CALLBACK, SOURCE),
}

outfits = [
    'Children of the Revolution Quest', 'Citizen Outfits Quest',
    'Citizen Outfits Quest on Rook', 'Citizen of Issavi Outfits Quest',
    'Crystal Warlord Outfits Quest', 'Dark Trails Quest',
    'Decaying Defender Outfits Quest', 'Demon Outfits Quest',
    'Draccoon Herald Outfits Quest', 'Dragon Slayer Outfits Quest',
    'Dream Warden Outfits Quest', 'Dream Warrior Outfits Quest',
    'Druid Outfits Quest', 'Druid Outfits Quest on Rook',
    'Elementalist Outfits Quest',
]
for title in outfits:
    specific = ('The existing DecayingDefender grant is explicitly base-only and cannot grant either addon. '
                if title == 'Decaying Defender Outfits Quest' else '')
    BLOCKERS[title] = shared(
        f'{title}: general durable quest outfit/addon entitlement composed with its source objective/hand-in. '
        + specific + 'QuestOutfitGrant is closed to Revenant-base and DecayingDefender-base; NPC storage execution rejects Outfit/OutfitAddon/item effects. '
        'qengine2 owns the general outfit/addon and atomic hand-in engine, so a lane NPC module cannot truthfully finish this reward by setting a quest flag.',
        'qengine2', WARDROBE, NPC, 'crates/gameplay/quests/src/quest/npc_rules.rs:472')

rows = json.loads(Path('.questpc6/QUEST-STATUS.json').read_text())
assert set(BLOCKERS) == {r['title'] for r in rows} - {'Dawnport Quest', 'Deeper Fibula Quest'}
for row in rows:
    if row['title'] not in BLOCKERS:
        continue
    blocker = BLOCKERS[row['title']]
    blocker['evidence'] = [row['definition'], *blocker['evidence']]
    blocker['attempts'] = [
        'Read this canonical source definition, the separately chosen recipe stages and the inherited binding record.',
        'Traced the production callback and NPC durable APIs against the specific missing effect/event; source evaluation and a quest flag do not supply the absent authority.',
        'Checked the concrete shared engine reservation; retained the dependency rather than duplicating the qengine/qengine2 owner in a lane module.',
    ]
    if row['title'] == 'Chyllfroest Mini World Change':
        blocker['attempts'].append('Read cached Crystal201189f actions_ursagrodon.lua:1-62: target20358->20359->20360, broken20361, 600-second revertIce, item20355 and mount38. Confirmed taming.json has no item20355 rule and TamingRule cannot express staged target transforms plus timed reset.')
    for ref in blocker['evidence']:
        path, locator = ref.split(':', 1)
        line = int(locator.split('#')[0])
        assert Path(path).is_file(), ref
        assert 1 <= line <= len(Path(path).read_text().splitlines()), ref
Path('.questpc6/ENGINE-BLOCKERS.json').write_text(json.dumps(BLOCKERS, indent=2) + '\n')
print(f'Concrete shared-owner dependency packets: {len(BLOCKERS)}')
