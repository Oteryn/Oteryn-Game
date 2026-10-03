"""D289 holds: main's accepted Item state wins over these packets' source facts.

D289 is the control-plane decision recorded on Oteryn/Oteryn-Game#1622. A held Item keeps its
accepted Native state; the packet records the hold instead of promoting the fact. A hold never
widens what a packet promotes, and every packet requires each of its holds to hit exactly once.
"""

DECISION = "D289"

# i2820 keeps main's accepted writeable=true/readable=true; the source writeable=false is held.
DOCUMENT_HOLDS = {
    "oteryn:item.tibia.i2820": {
        "reason": "HELD_SOURCE_CONFLICT",
        "source_facts": {"writeable": False},
    },
}

# i901 keeps main's native_core_hold (accepted NonStackable against a StackCapable source);
# no market, movable, physical or description enrichment is applied to it.
NATIVE_CORE_HOLD = "D289_ACCEPTED_NATIVE_CORE_HOLD"
NATIVE_CORE_HOLD_KEYS = frozenset({"oteryn:item.tibia.i901"})


def require_hits(expected, hit, packet):
    """Fail closed when a D289 hold no longer matches exactly one source row."""
    if sorted(hit) != sorted(expected):
        raise ValueError(f"{packet}: D289 hold scope drift: {sorted(hit)}")


# D310 (extends D289): where main's hand-sealed qualification evidence froze an Item's Native
# state, that sealed state wins over this PR's promotion. i36586: main's sealed
# tools/content-migration/samples/engine-family-navigation-265.json records no description.
SEALED_STATE_DECISION = "D310"
SEALED_STATE_HOLD = "D310_MAIN_SEALED_NATIVE_STATE"
SEALED_STATE_DESCRIPTION_KEYS = frozenset({"oteryn:item.tibia.i36586"})
# i3450: main's sealed tools/content-schema/reward-claim-authoring/reward_stack_normalization.json
# pins its definition with trade restrictions unknown.
SEALED_STATE_MARKET_KEYS = frozenset({"oteryn:item.tibia.i3450"})
