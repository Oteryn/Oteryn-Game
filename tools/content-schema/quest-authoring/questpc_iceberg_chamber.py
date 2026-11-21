"""The authored fixed expedition selects an intact northern ice-walled alcove."""
import questpc_iceberg_approaches as expedition
from questpc_native_anchors import NativeAnchors

REFERENCE = 'iceberg-chamber:northern-alcove'


def resolve(root, quest, stage, reference, *, definition):
    if (quest,stage,reference) != (expedition.QUEST,'s3',REFERENCE):
        raise ValueError('Not the selected personal chamber survey')
    _, _, sources = expedition.resolve(
        root,quest,'s1',expedition.REFERENCE,definition=definition)
    matches = [s for s in definition['recipe']['stages'] if s['key']=='s3']
    if (len(matches)!=1 or (matches[0]['kind'],matches[0]['count'],matches[0]['objective'])
            != ('explore',1,'Inspect the hidden igloo chamber')
            or matches[0]['targets'] != ['Chakoya Iceberg igloo']):
        raise ValueError('Selected chamber survey contract changed')
    native = NativeAnchors(root)
    # The original furniture and enclosing walls are evidence, never replacements.
    for x,y,stack in (
            (32620,32554,[799,7107,7163]),
            (32620,32553,[6869,6729,6952]),
            (32618,32555,[6869,6728]),
            (32622,32555,[6591,7110,6728])):
        native.require(dict(x=x,y=y,floor=-6),stack)
    return (dict(x=32621,y=32555,floor=-6),6585,
            [*sources,'content/world/placements/region-z06-x127-y127.b3'])
