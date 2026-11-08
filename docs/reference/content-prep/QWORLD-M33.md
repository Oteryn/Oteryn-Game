# M33 — Feaster STEP phase selection

Reused the two Feaster of Souls STEP callback records from questpc7 commit
`2caf66704f735bde02cb68370a5b5e00dd2d87a6`, rather than recreating its quest.
The retained authoring witness is recorded in each callback's sources.

Sources retained from that checked branch:
- https://www.tibiawiki.com.br/index.php?title=Feaster_of_Souls_Quest&oldid=421868
- https://www.tibiawiki.com.br/index.php?title=Para_Norbert&oldid=419674

Both records use the same entrance tile and appearance. Programme phase 0 leads
to the initial destination, and phase 5 leads to purification. Level 250 and an
active premium account remain required. Production callback selection uses the
personal quest copy, with all guards checked again by the fenced durable writer.
Overlapping phase ranges remain invalid registry entries. A pending replay keeps
its originally selected callback.

The two `qworld_phase_steps` PG cases execute real STEP dispatch against the
production callback registry and premium latch. Their fenced setup transitions
arrange programme phases; they do not claim to qualify the preceding NPC story
or the whole quest. They check phase refusal, destinations, level/premium refusal,
flag writes and command replay. No new database migration is needed.
