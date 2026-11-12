"""Keep complete donor dialogue for branches held at an ownership boundary.

These records are not runnable NPC rules. In particular, a captured successful
reply is never emitted without the transaction which its donor branch requires.
"""

import hashlib
import json

from build_dialogues import REPO
from lua_tables import _string, tokenize

ANSWER = "https://github.com/Oteryn/Oteryn-Game/pull/2843#issuecomment-6589517205"
NPC_OWNER_QUESTION = "https://github.com/Oteryn/Oteryn-Game/pull/2843#issuecomment-6589624211"


def capture_evidence(capture, ranges, blocked, notes=()):
    npcs = []
    for npc, allowed in ranges.items():
        source = capture.read(npc)
        tokens = tokenize(source)
        fragments = []
        for index, token in enumerate(tokens):
            if token[1] not in {"say", "setMessage"} or index < 1 or tokens[index - 1][1] != ":":
                continue
            if not any(first <= token[2] <= last for first, last in allowed):
                continue
            cursor = index + 1
            if tokens[cursor][1] != "(":
                raise ValueError(f"Unexpected dialogue call: {npc}:{token[2]}")
            depth, parts = 1, []
            cursor += 1
            while cursor < len(tokens) and depth:
                value = tokens[cursor]
                if value[0] in {"string", "lstring"}:
                    parts.append(_string(value))
                elif value[1] == "(":
                    depth += 1
                elif value[1] == ")":
                    depth -= 1
                cursor += 1
            fragments.append({"line": token[2], "literal_parts": parts,
                              "source": capture.ref(npc, token[2])})
        npcs.append({"npc": "oteryn:npc." + npc, "fragments": fragments,
                     "source_sha256": hashlib.sha256(source.encode()).hexdigest()})
    document = {"schema": 1, "quest": capture.key, "runtime_enabled": False,
                "decision": ANSWER, "npc_owner_question": NPC_OWNER_QUESTION,
                "purpose": "Pinned full dialogue and atomic native branches held for an approved owner; not a second NPC handler.",
                "npcs": npcs, "blocked_branches": blocked, "notes": list(notes)}
    path = REPO / "content/dialogues/quest-source" / f"quest-batch3-a-{capture.quest}.json"
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(document, ensure_ascii=False, indent=2) + "\n")
