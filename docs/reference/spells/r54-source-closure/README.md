# R54: rzeczywiste definicje docelowe dla 175 slotów potworów

Pakiet zawiera **130 Ability, 141 Effect i 64 wystąpienia Formula**, zgodne z istniejącą `monster.schema.json`. Są to rzeczywiste struktury docelowe, bez kopii AST i bez nowego interpretera. Formuła wspólnego caster magnitude występuje w kilku samodzielnych fragmentach; przy scalaniu można deduplikować identyczne `(family, key, revision)`.

| Status danych docelowych | Sloty |
|---|---:|
| Kompletne kandydaty schemy danych, z docelowym schedule | 10 |
| Częściowe definicje bazowych Combatów | 62 |
| Brak bezpiecznej definicji docelowej | 103 |
| Razem | 175 |

Dziewięć kandydatów korzysta z **już zaakceptowanej reguły D25**, `OTERYN_MONSTER_AUTHORING_SCHEMA_V1.md` §8.7: jedna najlepiej dopasowana zdolność wiki, wynik co najmniej 3 i brak remisu. Są to Grimeleech slot 4 (`life_drain`) i 6 (`mana_drain`) oraz Rhindeer slot 2 (`physical`) w trzech wariantach donorów. Użyto istniejącego matchera i lokalnych cache Fandom z 27.09.2026; wspólne dane gatunku są współdzielone przez warianty, a cache Crystal uzupełnia wspólny. **Nie podstawiono poprawnych enumów pod literówki donora.** Surowe `COMBAT_UNDEFINEDDAMAGE` i parametry zachowano w R51 i rekordach pakietu. D25 jest jawną normalizacją docelową; zgodność źródłowego typu/liczb i natywnego providera pozostaje niekwalifikowana.

Dziesiąty kandydat, przypięty Crystal Ratmiral Ball, odwzorowuje zerowy bazowy heal z MAGIC_RED oraz następujący callback: górna istota na kaflu, monster, jedna z trzech nazw sojuszników, niezależne `math.random(0, 1000)` i addHealth. Użyto istniejących `Effect.affects.named_creatures` oraz `top_creature_only`, a referencje Creature odczytano z niezmiennego archiwum R28. Nieużywane `param.removeCaster` nie powoduje usunięcia rzucającego.

Pozostałe 165 slotów mają konkretne blokady projekcji: brakujący controller całego cast, callback, opóźnienie, operacja świata, source Condition SUBID albo signed/undefined health. Bazowe definicje Combat są oznaczone jako częściowe; nie pominięto tych blokad przy awansowaniu statusu. Dla literalnego `COMBAT_FORMULA_DAMAGE` zachowano zakres z `mina/maxa`, bez zastępowania go monster caster magnitude. Identyfikatory nowych definicji mają rewizje R54 właściwego donora; istniejące referencje Creature zachowują oryginalną rewizję R28.

**Żaden z 175 slotów nie otrzymał kwalifikacji natywnego wykonania.** Dziesięć kompletnych kandydatów oznacza kompletność danych według schemy i jawnej D25, a nie działające już czary na serwerze. Wszystkie runtime/native/canonical-selection flagi pozostają wyłączone. Badanie sieciowe nie było potrzebne: źródła Canary/Crystal oraz wiki odczytano lokalnie, a hashe policy/cache i źródeł zapisano w receipt.
