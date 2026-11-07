"""Focused fixture tests for convert_store.py's full pipeline (lua_lite parsing, engine
default derivation, item-ref resolution). Builds a tiny synthetic engine checkout in a
temp dir; no dependency on the real pinned Canary/Crystal checkouts. Item identity still
resolves against the real committed `imports/crystalserver/bindings/items.json` (item
239 is a real, stable Great Health Potion binding), never a fixture copy of it."""

from __future__ import annotations

import tempfile
from pathlib import Path

import convert_store as cs

CANARY_CONSTANTS = """
local GameStore = { Version = "test" }
GameStore.OfferTypes = {
    OFFER_TYPE_NONE = 0,
    OFFER_TYPE_STACKABLE = 2,
    OFFER_TYPE_MOUNT = 6,
    OFFER_TYPE_BLESSINGS = 14,
    OFFER_TYPE_HIRELING_SKILL = 23,
    OFFER_TYPE_NAMECHANGE = 7,
}
GameStore.States = { STATE_NONE = 0, STATE_NEW = 1, STATE_SALE = 2, STATE_TIMED = 3 }
GameStore.CoinType = { Coin = 0, Transferable = 1 }
GameStore.SubActions = { BLESSING_TWIST = 1 }
return GameStore
"""

CATALOG_INIT = """
local modules = { "potions", "services" }
local basePath = CORE_DIRECTORY .. "/modules/scripts/gamestore/catalog/"
local inlineCategories = dofile(basePath .. "parent_categories.lua")
local catalog = {}
for _, moduleName in ipairs(modules) do
    local category = inlineCategories[moduleName]
    if not category then
        category = dofile(basePath .. moduleName .. ".lua")
    end
    table.insert(catalog, category)
end
return catalog
"""

PARENT_CATEGORIES = """
return {}
"""

POTIONS_LUA = """
return {
    icons = { "Category_Potions.png" },
    name = "Potions",
    rookgaard = true,
    state = GameStore.States.STATE_NONE,
    offers = {
        {
            icons = { "Great_Health_Potion.png" },
            name = "Great Health Potion",
            price = 18,
            itemtype = 239,
            count = 100,
            description = "Restores hit points.",
            type = GameStore.OfferTypes.OFFER_TYPE_STACKABLE,
        },
        {
            icons = { "Fake_Item.png" },
            name = "Unbound Item",
            price = 1,
            itemtype = 999999999,
            count = 1,
            type = GameStore.OfferTypes.OFFER_TYPE_STACKABLE,
        },
        {
            icons = { "Hireling_Cook.png" },
            name = "Hireling Cook",
            price = 900,
            id = HIRELING_SKILLS.COOKING[1],
            count = 1,
            type = GameStore.OfferTypes.OFFER_TYPE_HIRELING_SKILL,
        },
    },
}
"""

SERVICES_LUA = """
return {
    icons = { "Category_Extras.png" },
    name = "Services",
    rookgaard = true,
    offers = {
        {
            icons = { "Name_Change.png" },
            name = "Character Name Change",
            price = 250,
            id = 65002,
            type = GameStore.OfferTypes.OFFER_TYPE_NAMECHANGE,
        },
    },
}
"""


def write_fixture(root: Path):
    (root / "data/libs/gamestore").mkdir(parents=True)
    (root / "data/modules/scripts/gamestore/catalog").mkdir(parents=True)
    (root / "data/libs/gamestore/constants.lua").write_text(
        CANARY_CONSTANTS, encoding="utf-8"
    )
    (root / "data/modules/scripts/gamestore/catalog/init.lua").write_text(
        CATALOG_INIT, encoding="utf-8"
    )
    (root / "data/modules/scripts/gamestore/catalog/parent_categories.lua").write_text(
        PARENT_CATEGORIES, encoding="utf-8"
    )
    (root / "data/modules/scripts/gamestore/catalog/potions.lua").write_text(
        POTIONS_LUA, encoding="utf-8"
    )
    (root / "data/modules/scripts/gamestore/catalog/services.lua").write_text(
        SERVICES_LUA, encoding="utf-8"
    )


def digest_fixture(root: Path) -> dict:
    digests = {}
    for rel in (
        "data/libs/gamestore/constants.lua",
        "data/modules/scripts/gamestore/catalog/init.lua",
        "data/modules/scripts/gamestore/catalog/parent_categories.lua",
        "data/modules/scripts/gamestore/catalog/potions.lua",
        "data/modules/scripts/gamestore/catalog/services.lua",
    ):
        data = (root / rel).read_bytes().replace(b"\r\n", b"\n")
        digests[("canary", rel)] = cs.git_blob(data)
    return digests


def test_full_pipeline():
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        write_fixture(root)
        old = cs.EXPECTED_DIGESTS
        cs.EXPECTED_DIGESTS = digest_fixture(root)
        try:
            result = cs.convert("canary", root, repo_root=cs.REPO_ROOT)
        finally:
            cs.EXPECTED_DIGESTS = old

    catalog = result["catalog"]
    report = result["conversion_report"]
    assert len(catalog["categories"]) == 2, catalog["categories"]
    assert len(catalog["offers"]) == 3, [o["display_name"] for o in catalog["offers"]]

    by_name = {o["display_name"]: o for o in catalog["offers"]}

    good = by_name["Great Health Potion"]
    assert good["product"]["kind"] == "item"
    assert good["product"]["item"]["item_refs"][0]["key"] == "oteryn:item.tibia.i239"
    assert good["product"]["item"]["count"] == 100
    assert good["coin_type"] == "transferable", "Canary always defaults to transferable"

    assert "Unbound Item" not in by_name, (
        "unresolved item ref must be excluded, not invented"
    )
    assert len(report["unresolved_item_refs"]) == 1
    assert report["unresolved_item_refs"][0]["itemtype"] == 999999999

    cook = by_name["Hireling Cook"]
    assert cook["product"]["kind"] == "hireling_typed"
    assert cook["product"]["hireling_typed"]["typed_id"] is None, (
        "non-literal id must be null, never guessed"
    )
    assert any(
        e["offer"] == "Hireling Cook" for e in report["unresolved_engine_offer_ids"]
    )

    namechange = by_name["Character Name Change"]
    assert namechange["product"] == {"kind": "service"}
    assert namechange["engine_offer_id"] == 65002


def test_coin_type_default_divergence():
    assert cs.default_coin_type("canary", None) == "transferable"
    assert (
        cs.default_coin_type("canary", "has {transferableprice} marker")
        == "transferable"
    )
    assert cs.default_coin_type("crystal", None) == "coin"
    assert cs.default_coin_type("crystal", "no marker here") == "coin"
    assert (
        cs.default_coin_type("crystal", "cask price is {transferableprice}")
        == "transferable"
    )


def test_state_resolution():
    assert cs._state_name(None) == "none"
    assert cs._state_name(1) == "new"
    try:
        cs._state_name("nonsense")
        raise AssertionError("expected ConvertError")
    except cs.ConvertError:
        pass


def main():
    tests = [
        test_full_pipeline,
        test_coin_type_default_divergence,
        test_state_resolution,
    ]
    for t in tests:
        t()
        print(f"ok   {t.__name__}")
    print("PASS")


if __name__ == "__main__":
    main()
