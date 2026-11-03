"""Closed authoring reader for reviewed NPC branches, not a Lua runtime.

Every unsupported condition or operation raises an error. Branch order and an
explicit empty terminal branch preserve if/elseif short circuiting. The caller
selects an exact quest-only function/block range before lowering it.
"""

import copy
import re

from capture import inventory, topic
from lua_blocks import parse
from lua_tables import Parser, _string, tokenize
from lua_writers import mask_code

OPERATORS = {"==": "eq", "~=": "ne", "<": "lt", "<=": "le", ">": "gt", ">=": "ge"}


def split_boolean(expression, operator):
    tokens = [token for token in tokenize(expression) if token[0] not in {"comment", "lcomment"}]
    parts, current, depth = [], [], 0
    for token in tokens:
        value = token[1]
        if depth == 0 and value == operator:
            parts.append(" ".join(current))
            current = []
        else:
            current.append(value)
            depth += value in {"(", "[", "{"}
            depth -= value in {")", "]", "}"}
    parts.append(" ".join(current))
    return parts


def compact(expression):
    return "".join(token[1] for token in tokenize(expression) if token[0] not in {"comment", "lcomment"})


class Lowering:
    def __init__(self, capture, npc, storage_prefix):
        self.c = capture
        self.npc = npc
        self.storage_prefix = storage_prefix + "."
        self.lines = capture.read(npc).splitlines()
        for index in range(len(self.lines)):
            line = mask_code(self.lines[index], literals=False).strip()
            if re.match(r"^(?:else)?if\b", line) and not re.search(r"\bthen\s*$", line):
                continuation = index + 1
                while continuation < len(self.lines):
                    line += " " + mask_code(self.lines[continuation], literals=False).strip()
                    self.lines[continuation] = ""
                    continuation += 1
                    if re.search(r"\bthen\s*$", line):
                        break
                self.lines[index] = line
        self.aliases = {}
        self.clamped_aliases = set()
        self.max_topic = max([0] + [int(match[1]) for line in self.lines if (match := re.search(r"setTopic\(playerId,\s*(\d+)\)", line))])
        self.statement_overrides = {}

    def track(self, expression):
        expression = compact(expression)
        if expression in self.aliases:
            return self.aliases[expression]
        if not expression.startswith(self.storage_prefix):
            raise ValueError(f"Foreign storage requires explicit reviewed mapping: {expression}")
        return self.c.track(expression[len(self.storage_prefix):])

    def condition(self, expression):
        """Disjunctive normal form for the closed supported condition atoms."""
        source = compact(expression)
        match = re.fullmatch(r'table.contains\(\{(.*)\},message:lower\(\)\)', source)
        if match:
            values = Parser("{" + match[1] + "}").value()["fields"]
            return [{"keywords": [field["value"]], "guards": [], "effects": [], "topics": None, "keyword_mode": "exact"} for field in values]
        match = re.fullmatch(r'table.contains\(\{(.*)\},player:getStorageValue\(([^)]*)\)\)', source)
        if match:
            values = Parser("{" + match[1] + "}").value()["fields"]
            return [{"keywords": [], "guards": [{"kind": "storage", "track": self.track(match[2]), "op": "eq", "value": field["value"]}], "effects": [], "topics": None} for field in values]
        alternatives = []
        for disjunction in split_boolean(expression, "or"):
            result = {"keywords": [], "guards": [], "effects": [], "topics": None}
            valid = True
            for atom in split_boolean(disjunction, "and"):
                atom = compact(atom)
                match = re.fullmatch(r'MsgContains\(message,("(?:\\.|[^"\\])*"|\'(?:\\.|[^\'\\])*\')\)', atom)
                if match:
                    keyword = _string(("string", match[1], 1))
                    result["keywords"].append(keyword)
                    continue
                match = re.fullmatch(r"npcHandler:getTopic\(playerId\)==(-?\d+)", atom)
                if match:
                    result["topics"] = [int(match[1])]
                    continue
                match = re.fullmatch(r"npcHandler:getTopic\(playerId\)(>=|<=|>|<)(\d+)", atom)
                if match:
                    comparison, value = match[1], int(match[2])
                    predicates = {">=": lambda x, bound=value: x >= bound, "<=": lambda x, bound=value: x <= bound,
                                  ">": lambda x, bound=value: x > bound, "<": lambda x, bound=value: x < bound}
                    result["topics"] = [number for number in range(self.max_topic + 1) if predicates[comparison](number)]
                    continue
                # Lua's `not number == 54` compares a boolean to a number and is
                # always false. Do not silently rewrite it as `number ~= 54`.
                if re.fullmatch(r"notplayer:getStorageValue\([^)]*\)==-?\d+", atom):
                    valid = False
                    break
                match = re.fullmatch(r"(?:player|Player\(creature\)):getStorageValue\(([^)]*)\)(==|~=|>=|<=|>|<)(-?\d+)", atom)
                if match:
                    result["guards"].append({"kind": "storage", "track": self.track(match[1]), "op": OPERATORS[match[2]], "value": int(match[3])})
                    continue
                match = re.fullmatch(r"player:getStorageValue\(([^)]*)\)(==|~=|>=|<=|>|<)os.time\(\)", atom)
                if match:
                    result["guards"].append({"kind": "storage_time", "track": self.track(match[1]), "op": OPERATORS[match[2]], "offset_seconds": 0})
                    continue
                match = re.fullmatch(r"([A-Za-z_]\w*)(==|~=|>=|<=|>|<)(-?\d+)", atom)
                if match and match[1] in self.aliases:
                    op, value = OPERATORS[match[2]], int(match[3])
                    if match[1] in self.clamped_aliases and value == 0 and op == "eq":
                        op = "le"
                    result["guards"].append({"kind": "storage", "track": self.aliases[match[1]], "op": op, "value": value})
                    continue
                match = re.fullmatch(r"player:getItemCount\((\d+)(?:,(\d+))?\)(>=|>|==)(\d+)", atom)
                if match:
                    count = int(match[4]) + (match[3] == ">")
                    result["guards"].append(inventory("item", int(match[1]), count, int(match[2]) if match[2] else None))
                    continue
                match = re.fullmatch(r"player:removeItem\((\d+),(\d+)(?:,(\d+))?\)", atom)
                if match:
                    args = (int(match[1]), int(match[2]), int(match[3]) if match[3] else None)
                    result["guards"].append(inventory("item", *args))
                    result["effects"].append(inventory("take_item", *args))
                    continue
                match = re.fullmatch(r"player:removeMoney(Bank)?\((\d+)\)", atom)
                if match:
                    money = {"amount": int(match[2]), "include_bank": bool(match[1])}
                    result["guards"].append({"kind": "money", **money})
                    result["effects"].append({"kind": "take_money", **money})
                    continue
                match = re.fullmatch(r"player:getMoney\(\)\+player:getBankBalance\(\)>=(\d+)", atom)
                if match:
                    result["guards"].append({"kind": "money", "amount": int(match[1]), "include_bank": True})
                    continue
                match = re.fullmatch(r"player:getLevel\(\)(==|~=|>=|<=|>|<)(\d+)", atom)
                if match:
                    result["guards"].append({"kind": "level", "op": OPERATORS[match[1]], "value": int(match[2])})
                    continue
                raise ValueError(f"Unsupported condition {self.npc}: {atom}")
            if valid:
                alternatives.append(result)
        return alternatives

    def operations(self, numbers):
        replies, effects, variants = [], [], []
        covered = set()
        for line in numbers:
            if line in covered:
                continue
            if line in self.statement_overrides:
                effects.extend(copy.deepcopy(self.statement_overrides[line]))
                continue
            raw = self.lines[line - 1].strip()
            code = mask_code(raw, literals=False).strip()
            normalized = compact(code)
            if not code or normalized.startswith("return"):
                continue
            if "npcHandler:say(" in normalized:
                replies.extend(self.c.say(self.npc, line))
                suffix = "\n".join(self.lines[line - 1:])
                argument = suffix[suffix.index("npcHandler:say(") + len("npcHandler:say("):]
                parser = Parser(argument)
                parser.value()
                end_line = parser.tokens[parser.i - 1][2]
                covered.update(range(line + 1, line + end_line))
                continue
            match = re.fullmatch(r"(?:player|Player\(creature\)):setStorageValue\(([^,]+),(.+)\)", normalized)
            if match:
                key, value = self.track(match[1]), match[2]
                if re.fullmatch(r"-?\d+", value):
                    effects.append({"kind": "set", "track": key, "value": int(value)})
                elif re.fullmatch(re.escape(f"player:getStorageValue({match[1]})+") + r"\d+", value):
                    effects.append({"kind": "add", "track": key, "value": int(value.rsplit("+", 1)[1])})
                elif value == "os.time()+configManager.getNumber(configKeys.BOSS_DEFAULT_TIME_TO_FIGHT_AGAIN)":
                    effects.append({"kind": "set_now_plus", "track": key, "value": 72000})
                elif re.fullmatch(r"os.time\(\)\+\d+", value):
                    effects.append({"kind": "set_now_plus", "track": key, "value": int(value.rsplit("+", 1)[1])})
                elif value == f"player:getStorageValue({match[1]})>=0andplayer:getStorageValue({match[1]})+1or0":
                    index = len(effects)
                    effects.append(None)
                    variants.append((index, [({"kind": "storage", "track": key, "op": "ge", "value": 0}, {"kind": "add", "track": key, "value": 1}),
                                             ({"kind": "storage", "track": key, "op": "lt", "value": 0}, {"kind": "set", "track": key, "value": 0})]))
                elif value == f"math.max(1,player:getStorageValue({match[1]})+1)":
                    index = len(effects)
                    effects.append(None)
                    variants.append((index, [({"kind": "storage", "track": key, "op": "ge", "value": 0}, {"kind": "add", "track": key, "value": 1}),
                                             ({"kind": "storage", "track": key, "op": "lt", "value": 0}, {"kind": "set", "track": key, "value": 1})]))
                else:
                    raise ValueError(f"Unsupported storage effect {self.npc}:{line}: {value}")
                continue
            match = re.fullmatch(r"npcHandler:setTopic\(playerId,(-?\d+)\)", normalized)
            if match:
                effects.append(topic(int(match[1])))
                continue
            match = re.fullmatch(r"player:(addItem|removeItem)\((\d+),(\d+)(?:,(\d+))?\)", normalized)
            if match:
                effect = inventory("give_item" if match[1] == "addItem" else "take_item", int(match[2]), int(match[3]), int(match[4]) if match[4] else None)
                if match[1] == "removeItem":
                    effect["optional"] = True
                effects.append(effect)
                continue
            match = re.fullmatch(r"player:removeMoney(Bank)?\((\d+)\)", normalized)
            if match:
                effects.append({"kind": "take_money", "amount": int(match[2]), "include_bank": bool(match[1])})
                continue
            if normalized == "player:checkGnomeRank()":
                effects.append({"kind": "authoring_expand_bigfoot_rank"})
                continue
            match = re.fullmatch(r"player:addMoney\((\d+)\)", normalized)
            if match:
                effects.append({"kind": "give_money", "amount": int(match[1])})
                continue
            match = re.fullmatch(r"player:addExperience\((\d+),true\)", normalized)
            if match:
                effects.append({"kind": "experience", "amount": int(match[1])})
                continue
            match = re.fullmatch(r"player:addOutfit\((\d+),(\d+)\)", normalized)
            if match:
                effects.append({"kind": "outfit", "look_type": int(match[1]), "addon": int(match[2])})
                continue
            match = re.fullmatch(r"player:addOutfitAddon\((\d+),player:getStorageValue\(([^)]*)\)==1and1or2\)", normalized)
            if match:
                key, index = self.track(match[2]), len(effects)
                effects.append(None)
                variants.append((index, [({"kind": "storage", "track": key, "op": "eq", "value": 1}, {"kind": "outfit_addon", "look_type": int(match[1]), "addon": 1}),
                                         ({"kind": "storage", "track": key, "op": "ne", "value": 1}, {"kind": "outfit_addon", "look_type": int(match[1]), "addon": 2})]))
                continue
            match = re.fullmatch(r'player:addAchievement\(("(?:\\.|[^"\\])*")\)', code.replace("\t", ""))
            if match:
                effects.append({"kind": "achievement", "name": _string(("string", match[1], line))})
                continue
            match = re.fullmatch(r"player:getPosition\(\):sendMagicEffect\((CONST_ME_[A-Z_]+)\)", normalized)
            if match:
                effects.append({"kind": "magic_effect", "effect": match[1]})
                continue
            match = re.fullmatch(r"localdestination=Position\((\d+),(\d+),(\d+)\)", normalized)
            if match:
                destination = {"x": int(match[1]), "y": int(match[2]), "floor": -int(match[3])}
                continue
            match = re.fullmatch(r"(?:player|Player\(creature\)):teleportTo\(Position\((\d+),(\d+),(\d+)\)\)", normalized)
            if match:
                effects.append({"kind": "teleport", "position": {"x": int(match[1]), "y": int(match[2]), "floor": -int(match[3])}})
                continue
            match = re.fullmatch(r"doTeleportThing\(creature,Position\((\d+),(\d+),(\d+)\)\)", normalized)
            if match:
                effects.append({"kind": "teleport", "position": {"x": int(match[1]), "y": int(match[2]), "floor": -int(match[3])}})
                continue
            if normalized in {"player:teleportTo(destination)", "Player(creature):teleportTo(destination)"}:
                effects.append({"kind": "teleport", "position": destination})
                continue
            match = re.fullmatch(r"destination:sendMagicEffect\((CONST_ME_[A-Z_]+)\)", normalized)
            if match:
                effects.append({"kind": "magic_effect", "effect": match[1]})
                continue
            match = re.fullmatch(r'player:addMapMark\(Position\((\d+),\s*(\d+),\s*(\d+)\),\s*(\d+),\s*("(?:\\.|[^"\\])*")\)', code)
            if match:
                effects.append({"kind": "map_mark", "position": {"x": int(match[1]), "y": int(match[2]), "floor": -int(match[3])}, "mark": int(match[4]), "text": _string(("string", match[5], line))})
                continue
            match = re.fullmatch(r"local([A-Za-z_]\w*)=player:getStorageValue\(([^)]*)\)", normalized)
            if match:
                self.aliases[match[1]] = self.track(match[2])
                continue
            if normalized in {"goodSide=0", "badSide=0"}:
                self.clamped_aliases.add(normalized.split("=")[0])
                continue
            if normalized in {"},npc,creature)", "npc,", "creature", "creature)", "}", "),", ")"}:
                continue
            if normalized.startswith(("localplayer=", "localplayerId=")):
                continue
            raise ValueError(f"Unsupported statement {self.npc}:{line}: {code}")
        result = [([], effects)]
        for index, choices in variants:
            expanded = []
            for guards, current in result:
                for guard, effect in choices:
                    changed = copy.deepcopy(current)
                    changed[index] = effect
                    expanded.append((guards + [guard], changed))
            result = expanded
        return replies, result

    def paths(self, nodes, conditions=(), statements=()):
        if not nodes:
            yield list(conditions), list(statements)
            return
        node, rest = nodes[0], nodes[1:]
        if node[0] == "stmt":
            yield from self.paths(rest, conditions, [*statements, node[1]])
        elif node[0] == "return":
            yield list(conditions), list(statements)
        elif node[0] == "if":
            for _, line, body in node[1]:
                original = mask_code(self.lines[line - 1], literals=False).strip()
                expression = re.sub(r"^(?:else)?if\s+|\s+then$", "", original)
                # These local clamps do not mutate quest state. The related
                # comparisons are lowered against max(0, stored value).
                if expression in {"goodSide < 0", "badSide < 0"}:
                    self.clamped_aliases.add(expression.split()[0])
                    yield from self.paths(rest, conditions, statements)
                    return
                yield from self.paths([*body, *rest], [*conditions, (expression, line)], statements)
            yield from self.paths([*node[2], *rest], conditions, statements)
        else:
            raise ValueError(f"Unsupported opaque block {self.npc}: {node}")

    def lower(self, first, last, label="branch", continue_matching=False):
        nodes = parse(self.lines, list(range(first, last + 1)))
        for sequence, (conditions, statements) in enumerate(self.paths(nodes), 1):
            replies, effect_variants = self.operations(statements)
            combinations = [{"keywords": [], "guards": [], "effects": [], "topics": None}]
            for expression, _ in conditions:
                alternatives = self.condition(expression)
                combined = []
                for previous in combinations:
                    for alternative in alternatives:
                        topics = alternative["topics"] if alternative["topics"] is not None else previous["topics"]
                        if previous["topics"] and alternative["topics"] and previous["topics"] != alternative["topics"]:
                            continue
                        combined.append({"keywords": previous["keywords"] + alternative["keywords"], "guards": previous["guards"] + alternative["guards"], "effects": previous["effects"] + alternative["effects"], "topics": topics,
                                         "keyword_mode": alternative.get("keyword_mode", previous.get("keyword_mode", "msg_contains"))})
                combinations = combined
            for branch, combination in enumerate(combinations, 1):
                keywords = list(dict.fromkeys(combination["keywords"]))
                if len(keywords) > 1:
                    raise ValueError(f"Conjunctive keywords require review {self.npc}: {keywords}")
                for variant, (extra_guards, effects) in enumerate(effect_variants, 1):
                    all_effects = combination["effects"] + effects
                    if not conditions and not all_effects and not replies:
                        continue
                    source_line = conditions[-1][1] if conditions else (statements[0] if statements else first)
                    extras = {"continue_matching": True} if continue_matching else {}
                    if combination.get("keyword_mode", "msg_contains") != "msg_contains":
                        extras["keyword_mode"] = combination["keyword_mode"]
                    if not keywords:
                        extras["any_message"] = True
                    rule = self.c.rule(self.npc, f"{label}-{source_line}-{sequence}-{branch}-{variant}", source_line, keywords,
                                       combination["guards"] + extra_guards, all_effects, replies=replies, topics=combination["topics"], **extras)
                    rule["sources"] += [self.c.ref(self.npc, line) for line in statements if line not in {source_line} and ("setStorageValue" in self.lines[line - 1] or "addItem" in self.lines[line - 1] or "removeItem" in self.lines[line - 1])]
