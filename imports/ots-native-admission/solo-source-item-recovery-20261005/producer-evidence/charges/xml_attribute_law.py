"""Pinned small primitive laws; these produce Source values, never Native fields."""

import re

XML_ENTITIES = {"amp": "&", "lt": "<", "gt": ">", "quot": '"', "apos": "'"}


def attribute_value(raw):
    # Own pugi default wconv_attribute/EOL; character refs retain their resulting whitespace.
    if not isinstance(raw, str) or "\x00" in raw:
        raise ValueError("raw literal input absent or NUL; explicit witness hold")
    raw = raw.replace("\r\n", "\n").replace("\r", "\n")
    result, position = [], 0
    while position < len(raw):
        char = raw[position]
        if char != "&":
            result.append(" " if char in "\t\n" else char)
            position += 1
            continue
        end = raw.find(";", position + 1)
        if end < 0:
            raise ValueError("unclosed entity requires own parse-result witness")
        entity = raw[position + 1 : end]
        if entity in XML_ENTITIES:
            value = XML_ENTITIES[entity]
        elif re.fullmatch(r"#(?:[0-9]+|x[0-9a-fA-F]+)", entity):
            codepoint = (
                int("0" + entity[1:], 0) if entity.startswith("#x") else int(entity[1:])
            )
            if not 0 < codepoint <= 0x10FFFF or 0xD800 <= codepoint <= 0xDFFF:
                raise ValueError("invalid/NUL character-reference parse held")
            value = chr(codepoint)
        else:
            raise ValueError("unknown entity requires own parse-result witness")
        result.append(value)
        position = end + 1
    return "".join(result)


def as_bool(parsed):
    return bool(parsed) and parsed[0] in "1tTyY"


def boolean_string(parsed):
    if parsed and ord(parsed[0]) >= 128:
        raise ValueError("own signed char/locale case-folding boundary held")
    return bool(parsed) and parsed[0].lower() not in "fn0"


def unsigned16(parsed):
    if re.fullmatch(r"[0-9]+", parsed):
        significant = parsed.lstrip("0") or "0"
        if len(significant) > 5:
            return 0, "OWN_FROM_CHARS_FALLBACK_ZERO"
        value = int(significant)
        if value <= 65535:
            return value, "FULL_FROM_CHARS_SUCCESS"
    return 0, "OWN_FROM_CHARS_FALLBACK_ZERO"
