# questpc4 implementation decisions

## Physical admission and floor assembly

- Admit only the seven helmet parts3222..3228 and the uncharged helmet3229 in the first slice. Existing native profiles and accepted canonical facts retain authority. Source: Crystal201189f `data/items/items.xml`, SHA256 `8a4a526c6e45b5a282679334eb1e1e25a5b72af13aa92c8a0d409755ebdbc509`; exact per-item lines are in the generated overlay.
- Physical custody/drop/replay qualification is separate from assembly. The independent eight-item PG case uses an ordinary tile. The existing full assembly assertion remains strict and is retained for the following runtime slice.
- The altar must be the actual current composed, non-House AID40040 placement. Seven complete physical floor operands become one fresh3229 ItemInstance in the same transaction as the triggering drop. Neither carried-only cooking nor transforming one surviving part satisfies this source action.
- A bounded recipe registry and the existing ordinary move, fencing and type-2 audit owners will provide reusable assembly. Preserve every original move proof before admitting the complete successor; failure rolls back the move and assembly together. Historical replay returns the original move receipt before resolving current custody.
- Repeated indistinguishable plain parts select the first existing Ground stack ordinal for each exact definition. Selection is deterministic and the receipt preserves the chosen physical identities. Pinned handler: `data-global/scripts/quests/the_ancient_tombs/movements_craft_helmet.lua:20-39`, SHA256 `1c7ea8ca397cb6bff7b0ac59a6b0d2514820cc2556cfe0de677527f83182da84` at Crystal201189f.
- The ruby3030/helmet3229 transformation to timed3230 remains part of the next implementation, with its actual source decay. The first physical slice grants no charges or timed state. It is not whole Ancient Tombs qualification.
- Investigations are limited to ten minutes. Resolve routine choices from the pinned source and existing owner conventions, record the decision here, and continue. Publish every completed green slice immediately and keep pushes within thirty minutes.
