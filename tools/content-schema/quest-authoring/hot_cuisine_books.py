"""Serve Jean Pierre's readable copies from Crystal's two upstairs chests."""
import hashlib
import json

CRYSTAL = 'crystal:201189f871d23597b82caf88fd611454a2ddefb8:'
BOOK_SOURCES = [(9093, 32531, '161-167'), (11541, 32529, '121-127')]


def build(root):
    region = 'content/world/placements/region-z05-x129-y127.b3'
    region_digest = hashlib.sha256((root / region).read_bytes()).hexdigest()
    course = json.loads((root / 'content/quests/hot-cuisine-batch3.json').read_bytes())
    books = {row['item']: row for row in course['books']}
    elixir = next(row for row in course['recipes'] if row['reward']['item'] == 'oteryn:item.tibia.i11588')
    bake = next(row for row in course['recipes'] if row['reward']['item'] == 'oteryn:item.tibia.i11584')
    coconut_milk = next(row for row in bake['ingredients']
                        if row['item'] == 'oteryn:item.tibia.i2874' and row['subtype'] == 15)
    quantities = {row['item']: row['count'] for row in elixir['ingredients']}
    track_prefix = 'oteryn:quest-progress/quest/u8_5/hot_cuisine_quest/'
    callbacks = []
    for number, (item, y, lines) in enumerate(BOOK_SOURCES, 1):
        key = f'oteryn:item.tibia.i{item}'
        book = books[key]
        text = book['text'].replace('Jalape�o', 'Jalapeño').replace('Rolls- 1 Fern', 'Rolls\n- 1 Fern')
        if number == 2:
            text = text.replace('- 5 vials of milk', f"- {coconut_milk['count']} vials of coconut milk")
            text = text.replace('- 100 eggs\n- 50 mangoes',
                                f"- {quantities['oteryn:item.tibia.i3606']} eggs\n"
                                f"- {quantities['oteryn:item.tibia.i5096']} mangoes")
        callbacks.append({
            'key': f'oteryn:quest-callback/hot-cuisine-cookbook-{number}',
            'quest': course['quest'], 'trigger': 'use',
            'selector': {'position': {'x': 33064, 'y': y, 'floor': -5}, 'appearance': 2472},
            'reversible': False, 'passages': [], 'offerings': [], 'return_on_failure': False,
            'storage': [
                {'track': course['cookbook_door_track'], 'minimum': 1, 'maximum': 1,
                 'set': 1, 'preserve': True},
                {'track': f'{track_prefix}cookbook-{number}-copy',
                 'minimum': -1, 'maximum': -1, 'set': 1},
            ],
            'item_reward': {'item': key, 'count': 1, 'contents': [], 'text': text},
            'sources': [
                *book['sources'],
                f'{CRYSTAL}data-global/startup/tables/container_quest.lua:{lines}',
                f'{region}:1#(33064,{y},-5)/1:sha256:{region_digest}',
            ],
        })
    return callbacks
