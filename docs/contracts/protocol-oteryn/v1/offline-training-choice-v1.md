# Offline training choice V1

Capability 604 `OFFLINE_TRAINING_CHOICE_V1` requires 54 `TRAINING_V1` and
is offered with it when the Bundle World training owner is enabled. Command
66 remains a statue-handle request with a skill derived from that statue.
Beds retain their own command and custody checks.

## Compare skills

Command 6369 `OFFLINE_TRAINING_SKILLS` takes an empty request. An accepted
reply is seven little-endian u16 values. The first six are the actor's
persisted base levels in selection order, followed by the offline training
pool in whole minutes (0–720). The current character/session fence protects
the read. An unavailable read or a nonempty request is rejected with an empty
payload. Viewing the skill levels neither logs out nor reserves a skill.

## Choose a skill

Command 6370 `OFFLINE_TRAINING_CHOOSE` takes exactly one byte. IDs are
0 magic, 1 fist, 2 club, 3 sword, 4 axe and 5 distance. The client makes a
single choice and supplies no target, rate, duration, or progression value.
The server uses the existing Premium, combat, position and logout
requirements. If accepted, the existing fenced logout transaction stores
exactly that choice, then the ordinary offline settlement owns all skill
gains and pool charges. The wire result is the command 66 `TrainingResult`
format, with disposition 3 on logout and 4 on refusal. Without the capability,
the command is rejected by normal admission with an empty payload.

Crystal 201189f8 provides the six-skill choice in
[ProtocolGame::parseStartOfflineTraining](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/src/server/network/protocol/protocolgame.cpp#L9161-L9189)
and checks the combat lock and starts logout in
[Game::playerStartOfflineTraining](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/src/game/game.cpp#L10922-L10953).
The [Tibia Fandom archive](https://tibia.fandom.com/wiki/Training_School) describes the
skill choices, statue interaction, and offline time pool. Existing Oteryn
Premium and training rules still apply.
