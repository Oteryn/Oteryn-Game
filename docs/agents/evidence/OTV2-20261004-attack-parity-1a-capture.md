# OTV2-20261004-attack-parity-1a capture record

Packet: ATTACK-PARITY-1 §2.1 (`OTV2-20261004-attack-parity-1a-fixtures`). Fixture:
`content/combat/parity/tibiapal_auto_attack_v1.json`; tool: `tools/combat-parity/`.

## Capture

- Source: `POST https://tibiatools.io/api/v1/damage`, weapon catalogue `GET /api/v1/meta/weapons` (860 entries).
- Captured live, read-only: 2026-10-04T15:16:19Z to 2026-10-04T15:18:59Z (UTC). Each row has its own time.
- API description string recorded per row: "Weapon id 1 (fists) is the default when no weapon is sent."
- Rows: **1536** = 4 vocations x 6 levels x 4 skills x (fists + 15 weapons). Every row binds `OFFENSIVE` / 1.0 (§1.1). No monk row.

## Weapons (one Oteryn item key per TibiaTools weapon)

Axe, club and sword, one weapon per attack residue mod 5, one-handed, unique name in both the
catalogue and the Oteryn definitions, preferring a weapon whose Oteryn `weapon.attack` equals the
catalogue attack. **Missing residues: none** (all 15 class/residue cells are covered). The mapping
and each `tibiatools_attack` are in the fixture's `grid.weapons`.

For every selected weapon the Oteryn attack equals `tibiatools_attack` at selection; 1b still
asserts it (§1.2) before any formula check.

## Integrity check (scratch, not checked in)

All 1536 rows reproduce exactly the pinned upstream expression of §1.3 (min, avg, max). No Rust
change and no comparison with Oteryn code is part of this packet.

## Deferred finding carried from #1768 (review comment 4177982269, P2)

ATTACK-0 §6 asks for fixtures across fight modes. The calculator has no fight-mode field (§1.1),
so this grid is the Offensive-bound grid only and does **not** satisfy the fight-mode part of
ATTACK-0 §6. The packet states that it amends nothing while narrowing §6; this record states the
gap: the Balanced (0.75) and Defensive (0.5) attack factors, monk melee, defence, armor, block,
creature melee, the attack interval and the in-fight deadline stay `PARITY_PENDING` and are
**unchecked**. Neither ATTACK-PARITY-1a nor 1b owns them; closing 1a/1b does not close ATTACK-PARITY-1.
The remaining fight-mode parity needs a control-plane allocation (a reliable source, or an official
CipSoft value, which governs where it exists).

## Live rerun

`python3 tools/combat-parity/capture_tibiapal_auto_attack.py capture` (see `tools/combat-parity/README.md`).
