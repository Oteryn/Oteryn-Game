"""Derive the new sceptre expectation from the accepted recipe, independently of profiles."""
import json


def declared_keys(root):
    callbacks = json.loads((root / 'content/quests/authored-world-callbacks.json').read_text())['callbacks']
    crystal = [c for c in callbacks if c['key'].startswith('oteryn:quest-callback/authored-wrath-crystal-')]
    if not crystal:
        return set()
    packet = json.loads((root / 'tools/content-schema/quest-authoring/samples/server-completion/chosen-source-events-rewards/packet.json').read_text())
    def quests(value):
        if isinstance(value, dict):
            if value.get('wiki_title') == 'Wrath of the Emperor Quest':
                yield value
            else:
                for child in value.values():
                    yield from quests(child)
        elif isinstance(value, list):
            for child in value:
                yield from quests(child)
    recipe, = quests(packet)
    stage, = [s for s in recipe['stages'] if s['stage_key'] == 's8']
    target, = [t for t in stage['targets'] if t['target'] == 'Snake Sceptre']
    key = target['refs'][0]['key']
    if stage['count'] != 4 or len(crystal) != 4 or key != 'oteryn:item.tibia.i11371':
        raise ValueError('Wrath crystal recipe identity/count disagreement')
    if any(c['item_on_target']['item'] != key for c in crystal):
        raise ValueError('Wrath crystal callback tool differs from accepted recipe')
    return {key}
