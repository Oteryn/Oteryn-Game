# Next N quest source packet

Research only; no runtime or quest-completion claim.

Wiki captures: `.questpc2/wiki/Newhaven Quest_Spoiler.wiki` and `Newhaven Legion Helmet Quest_Spoiler.wiki`. The helmet needs both Gustavo and Viola missions; existing tutorial-islet support does not establish them.

Source conflict: Wiki gives Guardian Ring plus800XP for Gustavo and Scarf plus800XP for Viola. The pinned Crystal Gustavo script gives Item3052 (a placeholder healing ring) and noXP. Prefer the Wiki rewards; do not blindly promote the donor placeholder.

The source packet has runtime_admission=false and no admitted callbacks. Start by wiring the two NPCs and their real kill/item/reward owners. Migration1020–1039 remains unused, no command/capability allocation allowed.

## gustavo_the_guard
zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/gustavo_the_guard.lua:35-137
    local NEWHAVEN_GOBLIN_QUEST_START = Storage.Quest.U15_12.newhavenGoblinQuestStart
    local NEWHAVEN_GOBLIN_QUEST_DONE = Storage.Quest.U15_12.newhavenGoblinQuestDone
    local MUGLEX_KILL_COUNT = Storage.Quest.U15_12.newhavenMuglexKillCount
    player:setStorageValue(NEWHAVEN_GOBLIN_QUEST_START, 1)
    player:setStorageValue(MUGLEX_KILL_COUNT, 0)
    player:setStorageValue(NEWHAVEN_GOBLIN_QUEST_DONE, 1)
    player:addItem(3052, 1) -- Ring of Healing or appropriate reward
## viola
zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/viola.lua:53-292
    local VIOLA_BOOK_QUEST_START = Storage.Quest.U15_12.newhavenViolaBookQuestStart
    local VIOLA_BOOK_QUEST_DONE = Storage.Quest.U15_12.newhavenViolaBookQuestDone
    player:addItem(3031, 50)
    player:addItem(rewardItem, 1)
    player:removeItem(BOOK_OF_WISDOM_ID, 1)
    player:setStorageValue(VIOLA_BOOK_QUEST_DONE, 1)
    player:setStorageValue(VIOLA_BOOK_QUEST_START, 1)
## anna
zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/anna.lua:53-147
    player:setStorageValue(Storage.Quest.U15_12.newhavenCitizen, -1)
    player:setStorageValue(Storage.Quest.U15_12.newhavenTutorialHunting, -1)
    player:setStorageValue(Storage.Quest.U15_12.newhavenNewLootTheCorruptor, -1)
## a_seagull
zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/a_seagull.lua:53-115
    player:setStorageValue(Storage.Quest.U15_12.newhavenCitizen, -1)
    player:setStorageValue(Storage.Quest.U15_12.newhavenTutorialHunting, -1)
    player:setStorageValue(Storage.Quest.U15_12.newhavenNewLootTheCorruptor, -1)
## avriel
zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/avriel.lua:53-78
## flavius
zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/flavius.lua:53-116
