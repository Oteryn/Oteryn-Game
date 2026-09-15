local mType = Game.createMonsterType("Test Dragon")
local monster = {}
monster.experience = 700
monster.health = 1000
monster.speed = 200
monster.loot = {
  { name = "gold coin", id = 3031, chance = 80000, maxCount = 100 },
  { name = "dragon ham", chance = 25000 },
}
monster.defenses = {
  defense = 20,
  armor = 25,
}
monster.elements = {
  { type = COMBAT_FIREDAMAGE, percent = -10 },
  { type = COMBAT_ICEDAMAGE, percent = 20 },
}
monster.immunities = {
  { type = "paralyze", condition = true },
  { type = "invisible", condition = false },
}