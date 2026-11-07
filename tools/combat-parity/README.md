# combat-parity

Fixture capture for ATTACK-PARITY-1 (`docs/architecture/reviews/OTERYN_GAME_ATTACK_PARITY1_AUTO_ATTACK_PARITY_SCOPE_PACKET_2026-10-04.md` §1.1, §1.2).

`capture_tibiapal_auto_attack.py` posts the grid to the public TibiaTools damage API
(`POST https://tibiatools.io/api/v1/damage`, weapon catalogue `GET /api/v1/meta/weapons`) and writes
`content/combat/parity/tibiapal_auto_attack_v1.json`. The fixture is checked in; CI never calls the network.

## Offline

```text
python3 -m unittest tools/combat-parity/test_capture_tibiapal_auto_attack.py
python3 tools/combat-parity/capture_tibiapal_auto_attack.py verify
```

`verify` rebuilds every request body from the fixture's `grid` and compares it with the stored rows.

## Live rerun (not run in CI)

```text
python3 tools/combat-parity/capture_tibiapal_auto_attack.py capture [--out PATH] [--workers 4]
```

It is read-only and takes about three minutes for 1536 requests. Behind a proxy, set `SSL_CERT_FILE`
to the proxy CA bundle. A rerun re-selects the weapons from the live catalogue and the Oteryn item
definitions, so a changed catalogue changes the grid; review the diff before committing it.

## Fixture

- Grid: knight, paladin, sorcerer, druid (no monk) x levels 8, 50, 100, 300, 600, 1000 x skills
  10, 50, 100, 120, for fists plus 15 melee weapons (axe, club, sword; one per attack residue mod 5).
- Request: `{"stats": {vocation, level, skill}, "weapon": {"id": N}}`. No stances, perks, charms,
  imbuements, wheel, ammunition, shield, rotation or targets.
- Row: `key`, `request`, `raw` (`Auto-attack` min/avg/max), `captured_at`, `api_description`,
  `oteryn_fight_mode` (`OFFENSIVE`), `oteryn_attack_factor` (1.0); weapon rows add
  `tibiatools_attack` and `oteryn_item_key`.
