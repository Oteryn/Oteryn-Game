# WCONTRIB decisions

Horse Station follows the owner's three-hour-or-zero-horses activation rule and three-hour raid expiry. A late observation preserves the original deadline; observing after expiry restores the stable immediately.

This rule intentionally differs from the retained rental-triggered 24-hour raid: Crystal 9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d `data-global/npc/palomino.lua:104-112,148-160`, SHA256 `595387918194d62554ba02e66c381e945fc065a0c0b3b2658ce773fe52f293da`. Native initial stables contain nine horses: `data-global/scripts/globalevents/spawn/horse_station_world_change.lua:1-9,18-22,39-43`, SHA256 `631b28ccc0120d2f8d9d74c1f9540b40989065b6785d7d4331f495389c92e07b`.
