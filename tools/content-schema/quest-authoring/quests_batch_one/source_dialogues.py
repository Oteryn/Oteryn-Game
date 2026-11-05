"""Preserve literal NPC messages without activating their held branches."""

from lua_tables import Parser, _string, as_python, assignments, tokenize

PIN = "zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:"


def messages(source, name):
    text = source.read("data-global/npc/" + name)
    tokens = [
        token for token in tokenize(text) if token[0] not in ("comment", "lcomment")
    ]
    for i in range(len(tokens) - 4):
        if [token[1] for token in tokens[i : i + 4]] not in [
            ["npcHandler", ":", "say", "("],
            ["npcHandler", ":", "setMessage", "("],
        ]:
            continue
        method = tokens[i + 2][1]
        depth, args, arg, j = 0, [], [], i + 4
        while j < len(tokens):
            token = tokens[j]
            if token[1] == ")" and depth == 0:
                args.append(arg)
                break
            if token[1] == "," and depth == 0:
                args.append(arg)
                arg = []
            else:
                arg.append(token)
                if token[1] in ("(", "{", "["):
                    depth += 1
                elif token[1] in (")", "}", "]"):
                    depth -= 1
            j += 1
        arg = args[0 if method == "say" else 1]
        expression = "".join(token[1] for token in arg)
        line, end = tokens[i][2], tokens[j][2]
        if len(arg) == 1 and arg[0][0] in ("string", "lstring"):
            parts = [_string(arg[0])]
        elif arg[0][1] == "{":
            parts = as_python(Parser(expression).value())
        elif name == "bozo.lua" and expression.startswith("targetValue.text["):
            # All36 config and4outfit source tables are already in authored JSON.
            continue
        elif (
            name == "simon_the_beggar.lua"
            and expression == "noResponse[npcHandler:getTopic(playerId)]"
        ):
            response = as_python(assignments(text, ["noResponse"])["noResponse"])
            parts = [response[8]]  # Source cookie topic8, not the other NPC services.
            line, end = 295, 308
        elif name == "hjaern.lua" and line == 205:
            # Source Norseman/ice-shard trade; not a What a Foolish reply.
            assert (
                expression
                == '"Here your are. "..count*2000.." gold coins for "..count.." shards."'
            )
            continue
        else:
            raise ValueError((name, line, expression))
        assert isinstance(parts, list) and all(
            isinstance(part, str) and part for part in parts
        )
        yield line, end, parts


def supplement(source, quest, document):
    replies = {
        key: reply
        for key, reply in document["replies"].items()
        if not key.startswith("source-only:")
    }
    for name in sorted(source.cohort(quest)):
        prefix = PIN + "data-global/npc/" + name + ":"
        for line, end, parts in messages(source, name):
            if any(
                reply["source"].startswith(prefix) and reply["lines"] == parts
                for reply in replies.values()
            ):
                continue
            replies[f"source-only:{name[:-4]}:{line}"] = {
                "lines": parts,
                "source": prefix + str(line) + "-" + str(end),
            }
    return {"schema": document["schema"], "replies": replies}
