"""SRV7 W05d: source-bound seasonal raid admissions, separate from donor raids.

The retained current-Tibia observation wins over Crystal's unbound 16:00
Game.startRaid("Halloween Hare") call. PR2781 already accepted the Oteryn
server-save boundary for this annual event (event-schedule/schedule.json).
"""

HARE = "oteryn:raid.kazordoon_raids.the_halloween_hare"


def admit_halloween_hare(record: dict, observed: dict) -> None:
    if record["key"] != HARE:
        return
    facts = observed["facts"]
    observations = facts["waves"]["value"].get("wiki_wave_observations", [])
    source = next((text for text in observations if
                   "right after the server save on October 31 in a single wave" in text
                   and "{{Mapper Coords|127.112|125.18|7|2|text=here}}" in text), None)
    timeline = facts["broadcast_timeline"]["value"]
    if source is None or timeline != [{"offset": "00:00:00", "message": "Beware, beware the halloween hare."}]:
        raise ValueError("SRV7_HALLOWEEN_SOURCE_CHANGED")
    # Mapper's integer/fraction pair is sector/tile, not a decimal world
    # coordinate. Its fourth argument is map zoom, not a spawn radius.
    at = {"x": 127 * 256 + 112, "y": 125 * 256 + 18, "floor": 7}
    area = {"from": dict(at), "to": dict(at)}
    record["schedule"] = {"kind": "calendar", "interval_ms": 86_400_000,
                          "window": {"start_month": 10, "start_day": 31,
                                     "end_month": 10, "end_day": 31}}
    record["end"] = {"kind": "until_reset", "despawn_after_ms": None}
    record["sources"] = [{"source": "wiki:20165:1135570", "selector": "The Halloween Hare / full section",
                          "retained_path": "docs/reference/content-prep/raid-content-1/admission-evidence/evidence-00000-00121.json",
                          "retained_key": HARE}]
    record["source_resolution"] = {
        "decision": "PR2781 annual Halloween server-save boundary; current-Tibia retained wiki over unbound donor raid name",
        "donor": "zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/globalevents/others/raids_schedule.lua:21-23",
        "coordinate": "retained source Mapper sector/tile marker; no replacement coordinate or inferred spawn rectangle",
    }
    record["variants"] = [{"key": "wiki:20165:1135570/halloween-hare", "announcements": [
        {"delay_ms": 0, "text": timeline[0]["message"], "kind": "advance"}], "waves": [
        {"delay_ms": 0, "entries": [{"creature": "oteryn:creature.the_halloween_hare",
          "source_name": "The Halloween Hare", "amount": {"min": 1, "max": 1},
          "area_anchor": "raid-anchor:32624_32018_7-32624_32018_7", "area": area,
          "boss": False, "spawn_mode": "single", "sources": record["sources"]}]}], "unresolved": []}]
    record["inactive_reasons"] = []
