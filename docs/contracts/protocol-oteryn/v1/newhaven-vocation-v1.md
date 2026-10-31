# Newhaven vocation v1

Capability 238, command 456, bundle World only. The current session must own a living
actor on Newhaven and durable tutorial Island progress. First choice has no level cap.
The existing Character build writer converts cumulative skill progress, preserving the
saved current health and mana pools and the existing derived maximum pool contract.
The existing starter owner independently retries the atomic once-only kit; NoRoom
writes neither kit items nor completion, and a later eligible portal/login retries.

## Request

A canonical protobuf uint32 field 1, exactly two bytes: vocation 1 sorcerer, 2 druid,
3 paladin, 4 knight, 5 monk. Other payloads are rejected.

## Reply

A canonical protobuf uint32 field 1: 1 Chosen, 2 AlreadyChosen, 3 Rejected,
4 Unavailable. AlreadyChosen never changes vocation. Exact command replay reconciles
the existing build occurrence, while independently current session authority is still
required before live installation or starter minting. A changed selection on the same
command is rejected. A committed choice can coexist with a pending starter kit.
