"""Bind only the chosen fixed personal iceberg, not a dynamic World spawn."""
QUEST = 'oteryn:quest.authored.chakoya_iceberg_mini_world_change'
REFERENCE = 'iceberg-approach:port-hope'
ADAPTATION = ('Make the iceberg route a fixed personal expedition rather than '
              'dependent on a mini-world spawn.')


def resolve(root, quest, stage, reference, *, definition):
    recipe = definition.get('recipe', {})
    first = recipe.get('stages', [{}])[0]
    if ((quest, stage, reference) != (QUEST, 's1', REFERENCE)
            or definition['identity']['key'] != QUEST
            or recipe.get('repeat', {}).get('kind') != 'once'
            or ADAPTATION not in recipe.get('adaptations', [])
            or (first.get('key'), first.get('kind'), first.get('count'), first.get('objective'))
            != ('s1', 'explore', 1, 'Reach the iceberg north of Port Hope')):
        raise ValueError('Not the chosen personal iceberg expedition')
    source = 'https://tibia.fandom.com/index.php?curid=76213&oldid=887139'
    if not any(r.get('url') == source and r.get('content_sha256') ==
               '002b68c1746f89bbb19af0d2be47bc61ab81022b65f1941c5615d95c4cb070a5'
               for r in recipe.get('source_refs', [])):
        raise ValueError('Pinned personal iceberg source changed')
    from questpc_area_approaches import resolve as area_approach
    result = area_approach(root, 'area-approach:oteryn:area.hunting_place.chakoya_iceberg')
    from questpc_native_anchors import NativeAnchors
    # The Area marker retains the ice-and-snow edge, not an arbitrary coast tile.
    NativeAnchors(root).require(dict(x=32623, y=32584, floor=-7), [799, 7152])
    return result[0], result[1], [*result[2], source]
