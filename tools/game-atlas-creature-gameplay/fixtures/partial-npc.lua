local internalNpcName = "Partial Trader"
local npcConfig = {}
npcConfig.shop = {
  { itemName = "rope", clientId = 3003, buy = 50 },
}
table.insert(npcConfig.shop, buildDynamicOffer())
local route = getDynamicDestination()
local travelKeyword = keywordHandler:addKeyword({ "Somewhere" }, StdModule.say, { npcHandler = npcHandler })
travelKeyword:addChildKeyword({ "yes" }, StdModule.travel, { npcHandler = npcHandler, cost = dynamicCost(), destination = route })