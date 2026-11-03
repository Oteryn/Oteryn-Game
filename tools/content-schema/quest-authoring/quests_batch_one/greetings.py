"""Qualified static source greetings; no Bozo sex default is inferred."""

from lua_tables import _string, tokenize


def build(source):
    prefix = "zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:"

    def literal(donor_path, line):
        values = [
            _string(token)
            for token in tokenize(source.read(donor_path).splitlines()[line - 1])
            if token[0] in ("string", "lstring")
        ]
        assert len(values) == 1, (donor_path, line, values)
        return {"text": values[0], "source": prefix + donor_path + ":" + str(line)}

    base = "data/npclib/npc_system/npc_handler.lua"
    default_farewell = literal(base, 82)
    default_walkaway = literal(base, 90)
    records = []
    for name, greet, farewell, walkaway in [
        ("ezebeth", 118, None, None),
        ("gnomilly", 278, None, None),
        ("gnombold", 274, None, None),
        ("gnomargery", 275, None, None),
        ("mazarius", 189, 190, 191),
    ]:
        donor_path = f"data-global/npc/{name}.lua"
        records.append(
            {
                "npc": "oteryn:npc." + name,
                "greet": literal(donor_path, greet),
                "farewell": literal(donor_path, farewell)
                if farewell
                else default_farewell,
                "walkaway": literal(donor_path, walkaway)
                if walkaway
                else default_walkaway,
            }
        )
    return {
        "content/dialogues/quests/batch-one-greetings.json": {
            "schema": "OTERYN_QUESTS1_GREETINGS_V1",
            "records": records,
        }
    }
