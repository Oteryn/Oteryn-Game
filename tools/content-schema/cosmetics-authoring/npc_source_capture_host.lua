-- Closed test-only donor callback host. Unknown actor methods fail explicitly;
-- it never executes in production and supplies no gameplay authority.
FACTS={storage={},inventory={},looks={},mounts={},now=1800000000,sex=1,premium=true,level=100,money=2000000000,bank=2000000000}
EVENTS={};TOPIC=0
-- Fixed test-only source RNG seed makes incidental callback branches reproducible.
math.randomseed(0)
local function event(kind,data) data=data or {};data.kind=kind;table.insert(EVENTS,data) end
os.time=function()return FACTS.now end
MsgContains=function(message,word)return string.find(string.lower(message),string.lower(word),1,true)~=nil end
CONTAINS=MsgContains
PLAYERSEX_FEMALE=0;PLAYERSEX_MALE=1;CALLBACK_MESSAGE_DEFAULT='MESSAGE_DEFAULT'
CALLBACK_GREET='GREET';CALLBACK_FAREWELL='FAREWELL';MESSAGE_GREET='MESSAGE_GREET';MESSAGE_FAREWELL='MESSAGE_FAREWELL';MESSAGE_WALKAWAY='MESSAGE_WALKAWAY'
CONST_ME_MAGIC_BLUE=13;CONST_ME_MAGIC_RED=14;CONST_ME_MAGIC_GREEN=15;CONST_ME_POFF=3;CONST_ME_BLOCKHIT=2;CONST_ME_STUN=32
local pos={x=100,y=100,z=7,sendMagicEffect=function(_,id)event('magic_effect',{id=id})end}
Position=function(x,y,z)return pos end
local methods={}
function methods:getId()return 1 end
function methods:getStorageValue(key)return FACTS.storage[key] or -1 end
function methods:setStorageValue(key,value)FACTS.storage[key]=value;event('set_storage',{key=key,value=value});return true end
function methods:getItemCount(id)return FACTS.inventory[id] or 0 end
function methods:removeItem(id,count)if self:getItemCount(id)<count then return false end FACTS.inventory[id]=self:getItemCount(id)-count;event('consume_item',{item=id,count=count});return true end
function methods:addItem(id,count)count=count or 1;FACTS.inventory[id]=self:getItemCount(id)+count;event('mint_item',{item=id,count=count});return self end
function methods:getSex()return FACTS.sex end
function methods:isPremium()return FACTS.premium end
function methods:getLevel()return FACTS.level end
function methods:getMoney()return FACTS.money end
function methods:getBankBalance()return FACTS.bank end
function methods:setBankBalance(amount)FACTS.bank=amount;event('set_bank',{amount=amount})end
function methods:removeMoney(amount)if FACTS.money<amount then return false end FACTS.money=FACTS.money-amount;event('consume_money',{amount=amount});return true end
function methods:removeMoneyBank(amount)if FACTS.money+FACTS.bank<amount then return false end local cash=math.min(FACTS.money,amount);FACTS.money=FACTS.money-cash;FACTS.bank=FACTS.bank-(amount-cash);event('consume_money_bank',{amount=amount});return true end
function methods:hasOutfit(look,addon)return FACTS.looks[look..':'..(addon or 0)] or false end
function methods:canWearOutfit(look,addon)return self:hasOutfit(look,addon)end
function methods:addOutfit(look,addon)event('grant_look_addon',{look=look,addon=addon or 0});return true end
function methods:addOutfitAddon(look,addon)event('grant_look_addon',{look=look,addon=addon});return true end
function methods:hasMount(id)return FACTS.mounts[id] or false end
function methods:addMount(id)event('grant_mount',{id=id});return true end
function methods:addAchievement(name)if name==226 then name='Aristocrat'end;event('achievement',{name=name});return true end
function methods:addAchievementProgress(name,target)event('achievement_progress',{name=name,target=target});return true end
function methods:getPosition()return pos end
function methods:sendTextMessage(mode,text)event('text_message',{mode=mode,text=text})end
function methods:isPlayer()return true end
function methods:getName()return 'Source Oracle' end
local player=setmetatable({}, {__index=function(_,key)return methods[key] or function()error('UNSUPPORTED_PLAYER_METHOD:'..key)end end})
Player=function(_)return player end
local handler={topic={}}
function handler:checkInteraction(...)return true end
function handler:getTopic(id)return TOPIC end
function handler:setTopic(id,value)TOPIC=value;self.topic[id]=value end
function handler:say(text,...)event('reply',{text=text})end
function handler:setCallback(kind,fn)if kind==CALLBACK_MESSAGE_DEFAULT then CALLBACK=fn end end
function handler:setMessage(...)end
function handler:addModule(...)end
function handler:resetNpc(...)TOPIC=0 end
function handler:releaseFocus(...)end
function handler:removeInteraction(...)end
NpcHandler={new=function(...)return handler end}
KeywordHandler={new=function(...)return setmetatable({}, {__index=function()return function()return setmetatable({}, {__index=function()return function()end end})end end})end}
FocusModule={new=function()return {}end};ShopModule=FocusModule;StdModule=setmetatable({}, {__index=function()return function()end end})
NpcSystem={parseParameters=function()end}
Game={createNpcType=function(name)return setmetatable({}, {__index=function()return function()end end})end}
Npc=function(_)return player end
configKeys=setmetatable({}, {__index=function(_,key)return key end});configManager={getBoolean=function()return false end,getNumber=function()return 0 end}
ITEM_WEDDING_RING=3004
function table.contains(t,value)for _,v in pairs(t)do if v==value then return true end end return false end
local function node()return setmetatable({}, {__index=function()return function()return node()end end})end
KeywordHandler={new=function()return node()end}
Blessings={getInquisitionPrice=function()return {},0 end}
getPlayerLevel=function(_)return FACTS.level end
getPlayerStorageValue=function(_,key)return methods.getStorageValue(player,key)end
setPlayerStorageValue=function(_,key,value)return methods.setStorageValue(player,key,value)end
Game.getStorageValue=function(key)return FACTS.world_storage and FACTS.world_storage[key] or -1 end
Game.setStorageValue=function(key,value)FACTS.world_storage=FACTS.world_storage or {};FACTS.world_storage[key]=value end
function methods:getFamePoints()return math.max(0,self:getStorageValue(44313))end
function methods:removeFamePoints(amount)self:setStorageValue(44313,self:getFamePoints()-amount);event('consume_owner_points',{owner='SpikeFame',amount=amount});return true end
function methods:getStoreInbox()return {addItem=function(_,id,count)event('mint_store_item',{item=id,count=count or 1});return player end}end
Condition=function(kind)return setmetatable({kind=kind},{__index=function()return function()end end})end
function methods:addCondition(condition)event('condition',{condition_kind=condition.kind});return true end
function __source_topic(value)
 if type(value)=='number'then TOPIC=value;handler.topic[1]=value end
 if not CALLBACK then return end
 for i=1,99 do local name,upvalue=debug.getupvalue(CALLBACK,i);if not name then break end
  if type(upvalue)=='table' and (name=='talkState'or name=='talk_state')then upvalue[1]=value end
 end
end
TAG_PLAYERNAME='|PLAYERNAME|';TAG_TIME='|TIME|';TAG_BLESSCOST='|BLESSCOST|';TAG_PVPBLESSCOST='|PVPBLESSCOST|';TAG_TRAVELCOST='|TRAVELCOST|'
IsTravelFree=function()return false end;getFormattedWorldTime=function()return '12:00'end
Blessings.getBlessingCost=function()return 0 end;Blessings.getPvpBlessingCost=function()return 0 end
function handler:parseMessage(text,...)return text end
NpcHandler.new=function(_,keywords)handler.keywordHandler=keywords;HOST_KEYWORDS=keywords;return handler end
function __dispatch_message(message)
 if HOST_KEYWORDS and HOST_KEYWORDS:processMessage(Npc(1),Player(1),message)then return end
 if CALLBACK then return CALLBACK(Npc(1),Player(1),1,message)end
end
local previous_set_callback=handler.setCallback
function handler:setCallback(kind,fn)if kind==CALLBACK_GREET then GREET=fn else previous_set_callback(self,kind,fn)end end
function handler:setMessage(kind,text)if kind==MESSAGE_GREET then GREETING=text end end
function __dispatch_message(message)
 if (message=='hi'or message=='hello') and FACTS.fresh_greeting~=false and GREET then GREET(Npc(1),Player(1));if GREETING then event('reply',{text=GREETING})end;return end
 if HOST_KEYWORDS and HOST_KEYWORDS:processMessage(Npc(1),Player(1),message)then return end
 if CALLBACK then return CALLBACK(Npc(1),Player(1),1,message)end
end
ITEM_DECORATION_KIT=23398;ITEM_ATTRIBUTE_DESCRIPTION='description'
ItemType=function(id)return {getName=function()if id==31510 then return 'golden outfit display'else error('UNSUPPORTED_ITEM_NAME:'..id)end end}end
function methods:getStoreInbox()return {getItems=function()return {}end,getMaxCapacity=function()return 20 end,addItem=function(_,id,count)event('mint_store_item',{item=id,count=count or 1});return player end}end
function methods:setAttribute(key,value)event('attribute',{key=key,value=value});return true end
function methods:setCustomAttribute(key,value)event('custom_attribute',{key=key,value=value});return true end
Tile=function(_)return {getTopCreature=function()return player end,getTopVisibleCreature=function()return player end}end

function methods:setActionId(value)event('action_id',{value=value});return true end
