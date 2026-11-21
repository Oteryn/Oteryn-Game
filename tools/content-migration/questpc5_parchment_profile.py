"""M15: qualify physical custody of the source cabinet's parchment, not writing.

The existing pinned XML/client importer owns portable facts and exact bindings.
The unresolved write-once target stays UNKNOWN; no transform or write hook is added.
"""
import copy
import json
import qengine_reward_item_profiles as shared

KEY = 'oteryn:item.tibia.i2833'
PROOF = 'tools/content-schema/native-gameplay/questpc5-parchment-profile.json'


def promote(existing, observed):
    old = copy.deepcopy(existing)
    if (old['production_definition']['production_key'] != KEY
            or old['admission'] not in (dict(materializable=False, stack_class='NonStackable',
                                            legal_destinations=['Ground']), observed['admission'])
            or observed['admission'] != dict(materializable=True, stack_class='NonStackable',
                                             legal_destinations=['CharacterInventory', 'Ground'])):
        raise ValueError('M15_PARCHMENT_CUSTODY_SCOPE')
    old['admission'] = copy.deepcopy(observed['admission'])
    return shared.reconcile(old, observed)


def admit(root, generated, profile_path, manifest_path, order, canonical):
    packet = shared.capture(root, ids={2833})
    packet.update(schema='OTERYN_M15_PARCHMENT_PROFILE/v1',
                  producer='tools/content-migration/questpc5_parchment_profile.py',
                  scope='Physical custody only. Unknown write-once target remains unresolved.')
    generated[PROOF] = (json.dumps(packet, ensure_ascii=False, indent=2) + '\n').encode()
    document = json.loads(generated[profile_path])
    existing = next(r for r in document['records'] if r['production_definition']['production_key'] == KEY)
    merged = promote(existing, packet['records'][0])
    existing.clear()
    existing.update(merged)
    generated[profile_path] = (json.dumps(document, ensure_ascii=False, indent=2) + '\n').encode()
    shared.admit_records(root, generated, profile_path, manifest_path, order, canonical, packet['records'])
