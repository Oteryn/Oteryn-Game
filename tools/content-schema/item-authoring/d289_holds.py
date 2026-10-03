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
