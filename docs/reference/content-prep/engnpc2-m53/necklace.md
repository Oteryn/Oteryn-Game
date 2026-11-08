# M53 — live necklace admission

Checked Crystal source: https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/julius.lua

Source SHA-256: `a3184eb9261079bc3fc08b97a5437fc34bf57b7df84b95dba39432316e775ea4`. Lines 145–150 inspect `CONST_SLOT_NECKLACE`, require item 3083 and advance Mission01 to 2. A carried necklace does not satisfy that branch.

The native equipment owner uses GAME-ITEM-01 durable slot 7 (AMULET), distinct from protocol Necklace value 2. This engine slice admits the existing declarative `equipped` guard for that slot and reads it through the locked equipment owner. The dedicated PG case also rejects slot 2, stale fencing and divergent replay.

This slice does not yet register the production Julius/Blood Brothers programme.
