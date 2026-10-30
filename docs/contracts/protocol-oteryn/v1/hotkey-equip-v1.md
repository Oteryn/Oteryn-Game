# Hotkey equip V1

PKG5 I17 candidate. Capability 179 requires equipment (12) and container views (14).
Command 348 resolves a definition under the active Channel content pin on the server.
The equipment owner remains the only equipment custody writer.

## Request

Exactly four bytes: a nonzero unsigned big endian item definition reference, using
the same generation index as domain 9. Zero, unknown references and other lengths
refuse before a write. No item instance identifier comes from the client.

## Reply

The existing `ItemMoveResultV1` with equipment results enabled, carried in an
accepted command result as for command 9. Domain 9 and open container domain 14 deltas follow
a successful move. An unknown durable outcome disconnects for reconciliation.

This initial slice admits direct backpack sources and a worn-item toggle into the
main backpack. Nested/pouch sources, managed destinations, automatic hand conflicts,
quiver routing and equip timing are not implemented by this candidate.
