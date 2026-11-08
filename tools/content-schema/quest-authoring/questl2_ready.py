"""Finished L2 passage and Djinn stories, isolated from the questl2b WIP packet."""
import questl2_djinn as djinn
import questl2_passages as passages

NPCS = tuple(sorted(set(djinn.NPCS) | set(passages.NPCS)))
ITEMS = djinn.ITEMS | passages.ITEMS


def append(add, state, callbacks, offers, inputs, pin):
    passages.append(add, state, callbacks, offers, inputs, pin)
    djinn.append(add, state, callbacks, offers, inputs, pin)
