"""Project the already-known Blue Spectacles head registration into physical custody."""
import copy
import qengine_reward_item_profiles as shared


def qualify(number, node, obj, physical, bindings, definition):
    if number == 27522:
        script = node.find("attribute[@key='script']")
        patterns = definition['semantics']['equipment']
        if (script is None or script.attrib != {'key': 'script', 'value': 'moveevent'}
                or [child.attrib for child in script] != [{'key': 'slot', 'value': 'head'}]
                or patterns.get('state') != 'KNOWN'
                or [p['primary_slot'] for p in patterns['value']['patterns']['value']]
                    != [dict(state='KNOWN', value='HEAD')]):
            raise ValueError('OPTICORDING_EXACT_HEAD_REGISTRATION')
        # The existing canonical HEAD pattern is retained by accepted_first.
        # This is registration metadata, not a custom USE/vision implementation.
        node = copy.deepcopy(node)
        node.remove(node.find("attribute[@key='script']"))
    return shared.accepted.qualify(number, node, obj, physical, bindings, definition)
