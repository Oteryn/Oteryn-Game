local internalNpcName = "Alice"
local npcConfig = {}
npcConfig.shop = {
  { itemName = "health potion", clientId = 266, buy = 50, sell = 25 },
  { itemName = "rope", clientId = 3003, sell = 15 },
}
npc:parseBank(message, npc, creature, npcHandler)
local travelKeyword = keywordHandler:addKeyword({ "Thais" }, StdModule.say, { npcHandler = npcHandler })
travelKeyword:addChildKeyword({ "yes" }, StdModule.travel, { npcHandler = npcHandler, premium = false, cost = 120, destination = Position(32369, 32241, 7) })
error("THIS LUA MUST NEVER EXECUTE")