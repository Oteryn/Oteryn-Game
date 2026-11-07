local mType = Game.createMonsterType("Partial Beast")
local monster = {}
monster.experience = 10
monster.health = 30
monster.speed = 180
monster.loot = {
  { name = "gold coin", chance = 50000 },
}
table.insert(monster.loot, dynamicLoot())
monster.defenses = { defense = 2, armor = 3 }
monster.elements = { { type = unknownElement(), percent = 1 } }
monster.immunities = { { type = "paralyze", condition = dynamicCondition() } }