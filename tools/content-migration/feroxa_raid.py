"""M52: project the retained global Lua occurrence, not the unrelated XML timer."""
import hashlib
import json
import re
import tarfile

REVISION = "9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d"
CORPUS = "tools/content-schema/quest-authoring/samples/donor-source/corpus.tar.gz"
LIB = "data-global/lib/quests/grimvale.lua"
EVENT = "data-global/scripts/quests/grimvale/globalevents_grimvale_feroxa.lua"


def sources(root):
    result = {}
    with tarfile.open(root / CORPUS) as archive:
        manifest = json.load(archive.extractfile("corpus-manifest.json"))
        for row in manifest["files"]:
            if row["source"] != "crystalserver" or row["path"] not in {LIB, EVENT}:
                continue
            raw = archive.extractfile(row["cache_path"]).read()
            if (row["revision"] != REVISION
                    or hashlib.sha256(raw).hexdigest() != row["sha256"]
                    or hashlib.sha1(b"blob " + str(len(raw)).encode() + b"\0" + raw).hexdigest() != row["git_blob_sha1"]):
                raise ValueError("FEROXA_SOURCE_PIN")
            result[row["path"]] = dict(text=raw.decode(), sha256=row["sha256"])
    if set(result) != {LIB, EVENT}:
        raise ValueError("FEROXA_SOURCE_SET")
    return result


def variant(root, area_anchor):
    captured = sources(root)
    lib, event = captured[LIB]["text"], captured[EVENT]["text"]

    def ref(path, token):
        text = captured[path]["text"]
        offset = text.index(token)
        return dict(repository="zimbadev/crystalserver", revision=REVISION,
                    path=path, line=text[:offset].count("\n") + 1,
                    sha256=captured[path]["sha256"], evidence="OTS_HYPOTHESIS_ONLY")

    def minutes(text, function):
        match = re.search(r"addEvent\(" + re.escape(function) + r",\s*(\d+)\s*\*\s*60\s*\*\s*1000", text)
        if not match:
            raise ValueError("FEROXA_TIMER:" + function)
        return int(match[1]) * 60_000

    middle = minutes(event, "grimvaleSpectators")
    second = middle + minutes(lib, "grimvaleSpectators")
    final = second + minutes(lib, "final")
    spawn = re.search(r'Game.createMonster\("Feroxa",\s*Position\((\d+),\s*(\d+),\s*(\d+)\)', lib)
    if not spawn:
        raise ValueError("FEROXA_SPAWN")
    at = dict(zip(("x", "y", "floor"), map(int, spawn.groups())))
    area = {"from": at, "to": dict(at)}
    announcements = []
    first = re.search(r'Game.broadcastMessage\("([^"]+)",\s*MESSAGE_EVENT_ADVANCE', event)
    if not first:
        raise ValueError("FEROXA_INITIAL_ANNOUNCEMENT")
    announcements.append(dict(delay_ms=0, raw_delay_ms=0, kind="advance", text=first[1],
                              source=ref(EVENT, first[0])))
    for base, prefix in [(middle, "Half of the current full moon"), (second, "The full moon is completely exposed")]:
        match = re.search(r'addEvent\(Game.broadcastMessage,\s*(\d+)\s*\*\s*60\s*\*\s*1000,\s*"('
                          + re.escape(prefix) + r'[^"]+)",\s*MESSAGE_EVENT_ADVANCE', lib)
        if not match:
            raise ValueError("FEROXA_PHASE_ANNOUNCEMENT:" + prefix)
        delay = base + int(match[1]) * 60_000
        announcements.append(dict(delay_ms=delay, raw_delay_ms=delay, kind="advance", text=match[2],
                                  source=ref(LIB, match[0])))
    return dict(key=LIB + "#full-moon-occurrence", announcements=announcements,
                sources=[ref(EVENT, 'function grimVale.onThink'), ref(LIB, 'local function final()')],
                unresolved=[], waves=[dict(delay_ms=final, raw_delay_ms=final, entries=[dict(
                    creature="oteryn:creature.feroxa", source_name="Feroxa", amount={"min": 1, "max": 1},
                    area=area, area_anchor=area_anchor(area), spawn_mode="single", boss=True,
                    source=ref(LIB, spawn[0]))])])
