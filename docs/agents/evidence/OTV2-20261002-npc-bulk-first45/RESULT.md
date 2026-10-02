# Basic NPC batch:133 partial definitions

All133 remaining actors have generated candidate definitions and basic original Oteryn name/job/greet/farewell templates. Catalogue:1282 NPCs,836 Dialogue.20 appearances are documentary literal donor data;113 use an explicit neutral placeholder. Missing NPC functionality remains disabled. There are0 loaded runtime NPCs and0 tested NPC gameplay interactions.

Existing test map: `apps/game-server/src/content/project/native_entry_room.json`; use its native mechanics tests and actual Oteryn path. No second map or alternate server was created. The four-cell room supports sequential future NPC fixtures, not an already implemented simultaneous133-NPC grid.

## Source access

- Canary47dfd51f / current04b83b51 and Crystalff7ede59 / summer00ce02: retained complete public Git tree manifests, read from the existing local research cache. Exact known filename leads checked; this is no global absence claim. Dragon Ancestor Spirit has matching Crystal summer source bytes.
- [TibiaWiki BR](https://www.tibiawiki.com.br/): committed public API snapshot from2026-09-28, SHA0773232ddd356be273474be7b3aea645ed5dbbf93e5832a94d565ad9e579657a;133 identities, documentary positions/offers. Read offline from the existing snapshot.
- [Tibiopedia](https://tibiopedia.pl/): committed public NPC snapshot from2026-09-28, SHA43bfc91ec7721df150d3803f7123df1909a167c6606e54b112075a433fda8651;132 identities after stripping only the `(NPC)` disambiguator. Wyrdin’s Apprentice has no exact match and retains TODO. Read offline from the existing snapshot.
- Other donor repositories below: previously captured public GitHub raw bodies, read from cache, SHA checked against actual bytes and pinned revisions. Related aliases/dynamic names are not used as target actor facts.
- No Remote Desktop or browser fallback was used in this round. No fresh wiki-page reads or100% real-Tibia fidelity is claimed. Historical stricter appearance qualifications are preserved.

| NPC | Exact public donor reference | Actual body SHA256 |
|---|---|---|
| Agostina | [ValeriaOT/Test](https://raw.githubusercontent.com/ValeriaOT/Test/a5fc1e96fc185d2f9a1e8652daffdc3c99ae32e0/data-otservbr-global/npc/drome_npc_ankrahmun.lua) | `489bfbb5640ede492c51fa796c35c14c4231df9a4f1b56dd5a1821bfc5e26c03` |
| Blubster | [Johncorex/otg-premium-version](https://raw.githubusercontent.com/Johncorex/otg-premium-version/650d5f2f3039611a56b4331a3993c661ca93cd52/data/npc/Blubster.xml) | `d5fe035972f6d536f971fee84153df8a22cad7186cde450c0e0f7fce68aab421` |
| Boasty Twoeyes | [Johncorex/otg-premium-version](https://raw.githubusercontent.com/Johncorex/otg-premium-version/650d5f2f3039611a56b4331a3993c661ca93cd52/data/npc/Boasty%20Twoeyes.xml) | `ffb8d12fef2e68bb4a65db93da25108db5023a2482cf7ba71dffa3ae71360575` |
| Captain Coohan | [dakotaotserver-glitch/dakbugs](https://raw.githubusercontent.com/dakotaotserver-glitch/dakbugs/c74bf2380718b4f659a6a19a5f42bf76e7abe63f/data-otservbr-global/npc/Captain_Coohan.lua) | `b281f11534630af1c495b6bbd44fbb2525a28a1498a376745ba90f68029eba5c` |
| Cheesy The Chosen | [Johncorex/otg-premium-version](https://raw.githubusercontent.com/Johncorex/otg-premium-version/650d5f2f3039611a56b4331a3993c661ca93cd52/data/npc/Cheesy%20The%20Chosen.xml) | `7f3da8fabe998e3e3cb9857d4f4ebf92be795c5bccc5d39d8cff794ee2ec5c66` |
| Dwarf Deep Guard | [Qwizer/realmap11](https://github.com/Qwizer/realmap11/blob/1d8a62c523869e6535397ac53df7dd973344e52c/data/npc/Dwarf Deep Guard.xml) | `70595e506edb72990a8016e0c3f4d554edbc2e8432df7e6bb03ea2e6bd5fcdee` |
| Dwarf Guard Night Shift | [Qwizer/realmap11](https://github.com/Qwizer/realmap11/blob/1d8a62c523869e6535397ac53df7dd973344e52c/data/npc/Dwarf Guard Night Shift.xml) | `7776137f6972dd82aa7e5cd6f3ac860c9c712a6e47f58d608187155d0e357f7e` |
| Dragon Ancestor Spirit | [zimbadev/crystalserver](https://raw.githubusercontent.com/zimbadev/crystalserver/00ce02a57ca5a12e48f32a3476e37471167e4c3f/data-global/npc/dragon_ancestor_spirit.lua) | `b7ad237012e48305f8ea9aff03436adfbed979e88ca6aee2748d002e2fc3bd14` |
| Gilmak Copperbeard | [GustavoContreiras/TheForgottenTibia](https://raw.githubusercontent.com/GustavoContreiras/TheForgottenTibia/dab99722d38253c80a8a233641a6dc9f9063c532/Server/data/npc/Gilmak%20Copperbeard.xml) | `e220525b8cfec3c0590993cf5eec4130f2b05276b961dc6234a65483c5c70636` |
| Gnome Guard | [Qwizer/realmap11](https://raw.githubusercontent.com/Qwizer/realmap11/1d8a62c523869e6535397ac53df7dd973344e52c/data/npc/Gnome%20Guard.xml) | `e24c4a5e9fd0adae32e0a252444c5fa9f0fbd04472dfb5e1a4f99af02ab4f99e` |
| Gnome Outer Guard | [Qwizer/realmap11](https://raw.githubusercontent.com/Qwizer/realmap11/1d8a62c523869e6535397ac53df7dd973344e52c/data/npc/Gnome%20Outer%20Guard.xml) | `0dbe5d66e4403abfee21bfd1bece5059f7f1ad57e5351aca1475d27b5d6725fa` |
| Gnomenace | [Johncorex/otg-premium-version](https://raw.githubusercontent.com/Johncorex/otg-premium-version/650d5f2f3039611a56b4331a3993c661ca93cd52/data/npc/Gnomenace.xml) | `3b9a765206627457e7dc01dd4f71bb040c60410660bd07ecbeac3a87b725d022` |
| Gnomevisor | [dakotaotserver-glitch/dakbugs](https://raw.githubusercontent.com/dakotaotserver-glitch/dakbugs/c74bf2380718b4f659a6a19a5f42bf76e7abe63f/data-otservbr-global/npc/gnomevisor.lua) | `ffb4f6e7cbdd06cca9216fa5942e6868764803be9594119d8b96f79cb4bf0457` |
| Gnominimus | [dakotaotserver-glitch/dakbugs](https://raw.githubusercontent.com/dakotaotserver-glitch/dakbugs/c74bf2380718b4f659a6a19a5f42bf76e7abe63f/data-otservbr-global/npc/gnominimus.lua) | `d3a5c9bf1965c10bd0ab6be4c24df8bad757a219c84e0519cef4adf626b6418d` |
| Hoaxette | [adrunkhuman/tibia-playerbots-project](https://raw.githubusercontent.com/adrunkhuman/tibia-playerbots-project/1afc43cf7efbf4ccc32df28ff45b726617050fa0/server/data/npc/Hoaxette.xml) | `b2a4f5a9c675c8fa9ed4ca99b26dc5482ddce5471a0092d2dfcf0b96717f9cc4` |
| Izzkl | [Johncorex/otg-premium-version](https://raw.githubusercontent.com/Johncorex/otg-premium-version/650d5f2f3039611a56b4331a3993c661ca93cd52/data/npc/Izzkl.xml) | `61f7d58054ced946a4590dec97e93058e60d185476c0afafd2e79df75b66bc0b` |
| Mandly | [Johncorex/otg-premium-version](https://raw.githubusercontent.com/Johncorex/otg-premium-version/650d5f2f3039611a56b4331a3993c661ca93cd52/data/npc/Mandly.xml) | `c4d1dd4b23ecc8a4443ff5c08a3fe395c52f7a58668b4ae587d6506795e0c83f` |
| Ned Nobel | [DrewWeth/SaveThaisServ](https://raw.githubusercontent.com/DrewWeth/SaveThaisServ/43bba803a417f7a790647a011eacbd0c78da9dee/TFS7.6/data/npc/Ned%20Nobel.xml) | `c510988847a3c5d3ae89aa01f1f858396968873850e25754c75c1411ddc84c9a` |
| Olhan | [Johncorex/otg-premium-version](https://raw.githubusercontent.com/Johncorex/otg-premium-version/650d5f2f3039611a56b4331a3993c661ca93cd52/data/npc/Olhan.xml) | `86a4c49ddbb12d640aee65a596c7739fefec7e6fc492bc9d6e261c094a713904` |
| Yrlin | [Johncorex/otg-premium-version](https://raw.githubusercontent.com/Johncorex/otg-premium-version/650d5f2f3039611a56b4331a3993c661ca93cd52/data/npc/Yrlin.xml) | `3c4f896d3a1096d9986d1dffcd9c26cbd1cecd63a1ce50ea3f1bac68155bd5a8` |

Economic references are retained as documentary fields; no token conversion, gated callback execution, item settlement or trip was enabled. Default radius2 and stationary interval0 are labelled defaulted where the source lacks those fields. Original templates are placeholder dialogue, never attributed to Tibia transcripts. Donor profiles remain donor rather than verified appearance. Exact native packet and whole-predecessor pins bind the generated data; existing native validators enforce references/identity/profiles atomically.
