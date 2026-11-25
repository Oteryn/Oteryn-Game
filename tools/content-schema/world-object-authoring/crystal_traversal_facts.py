"""Fixed WorldObject facts from Crystal 201189f8 items.xml and official appearances."""

import world_objects

SOURCE = {
    "engine": "crystal",
    "profile": "OTERYN_CRYSTAL_TRAVERSAL_201189F/v1",
    "repository": "zimbadev/crystalserver",
    "revision": "201189f871d23597b82caf88fd611454a2ddefb8",
    "items_xml_sha256": "8a4a526c6e45b5a282679334eb1e1e25a5b72af13aa92c8a0d409755ebdbc509",
}
# The coffins keep their canonical Item ownership (data/items/items.xml:218-228).
ITEM_FLOOR_CHANGES = {166: "down", 167: "down"}
# Existing fixed Items on the restored routes whose native profiles omit solidity.
WALKABLE_ITEM_FIXTURES = (775, 1894, 1951, 5022)
# data/items/items.xml:86051-86053, 87338-87344, 89703-89711.
ATTRIBUTES = {
    50564: {"type": "ladder"},
    51875: {"floorchange": "north"},
    51877: {"floorchange": "west"},
    53441: {"floorchange": "south"},
    54578: {"floorchange": "west"},
    54581: {"floorchange": "west"},
}


def records(appearances: dict, members: set):
    """Publish the six map-only WorldObjects."""
    result, static = [], []
    for identifier, attrs in ATTRIBUTES.items():
        if identifier not in members or identifier not in appearances:
            raise ValueError(f"missing official traversal appearance {identifier}")
        appearance = appearances[identifier]
        flags = appearance["flags"]
        if (
            not flags.get("flags.unmove")
            or flags.get("flags.unpass")
            or flags.get("flags.bank")
        ):
            raise ValueError(
                f"traversal appearance is not a walkable fixture: {identifier}"
            )
        record = world_objects.build_world_object(
            identifier,
            f"oteryn:item.tibia.i{identifier}",
            attrs.get("type", "object"),
            {"attrs": attrs},
            appearance,
            SOURCE,
        )
        result.append(record)
        del record["provenance"]["item_pointer"]
        blocks_projectile = bool(flags.get("flags.unsight"))
        record["collision"]["blocks_projectile"] = world_objects.known(
            blocks_projectile
        )
        record["provenance"]["fields"]["collision.blocks_projectile"] = [
            "appearance:flags.unsight"
        ]
        static.append(
            {
                "source_item_id": identifier,
                "identity": record["identity"],
                "policy": {
                    "ground": False,
                    "ground_speed": 0,
                    "block_solid": False,
                    "block_projectile": blocks_projectile,
                    "movable": False,
                    "pickupable": False,
                    "dynamic_kind": None,
                },
                "catalogue_record": record,
            }
        )
    return result, static


# Existing B3 ground at the south-stair and west-stair sources and landings.
GROUND_IDS = (53234, 53254, 54049, 54052, 54061)


def grounds(appearances: dict, members: set):
    result = []
    for identifier in GROUND_IDS:
        if identifier not in members or identifier not in appearances:
            raise ValueError(f"missing official stair ground appearance {identifier}")
        appearance = appearances[identifier]
        record = world_objects.build_terrain(
            identifier,
            f"oteryn:item.tibia.i{identifier}",
            "ground",
            None,
            appearance,
            SOURCE,
        )
        known = world_objects.known
        if (
            record["kind"] != known("ground")
            or record["walkable"] != known(True)
            or not appearance["flags"].get("flags.unmove")
        ):
            raise ValueError(
                f"stair ground is not an immovable walkable bank: {identifier}"
            )
        del record["provenance"]["item_pointer"]
        result.append(record)
    return result
