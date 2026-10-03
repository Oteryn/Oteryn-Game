# Klasyfikacja stworzeń

`monster-classification.schema.json` opisuje katalog metadanych authoringu dla każdej definicji Creature. Katalog jest powiązany SHA indeksu, digestem każdej paczki oraz SHA wejść Encounter. Nie zmienia statystyk, zachowania ani zamkniętego kontraktu runtime. Trzeba zachować go obok odpowiedniego snapshotu; użycie z innym indeksem lub zmienionymi paczkami jest odrzucane.

Role i konteksty są niezależne i mogą się nakładać. Boss może być questowy i raidowy, a zwykły potwór także używany jako summon. Role obejmują `creature`, `boss`, `summon`, `familiar`, `trainer`, `mechanic_actor` oraz jawne `unknown`. Konteksty obejmują world/quest/raid/event/dawnport/historical/encounter. Uczestnictwo Encounter i relacje transformacji zapisano osobno, z konkretnymi identyfikatorami i dowodami. Rola summon oznacza referencję w danych zachowania, nie potwierdzone wykonanie czaru podczas rozgrywki.

Każde twierdzenie ma pewność `confirmed` lub `inferred` oraz dowód. `confirmed` oznacza potwierdzenie w przypiętych danych/źródłach, nie pełną weryfikację Global. Bosstiary i reward_boss są dowodami roli boss. Katalog `bosses` daje jedynie wskazówkę. Brak Bestiary nie określa roli. Ogólny folder `quests` jest wskazówką kontekstu; użycie w skrypcie może je potwierdzić, jeżeli nazwa identyfikuje dokładnie tę definicję. Nazwy wspólne dla kilku wariantów pozostają wskazówkami.

Mitygacja ma osobny stan: `present`, `unknown`, `not_applicable`. Zero jest wartością `present`. Brak pola daje `unknown`, również dla bossa. `not_applicable` wymaga pozytywnego dowodu źródłowego; żaden rekord bieżącego katalogu nie został automatycznie tak oznaczony. Klasyfikacja nie wybiera wartości statystyk ani nie kopiuje ich między formami.

Przygotowanie z przypiętych publicznych cache i już przygotowanego snapshotu:

```bash
python tools/content-migration/build_monster_classification_evidence.py \
  --population "$population" --canary "$canary" --crystal "$crystal" --out "$work/source-role-evidence.json"
python tools/content-migration/classify_monster_population.py \
  --index "$population/population-index.json" --bundles "$population/bundles" \
  --annotations "$work/source-role-evidence.json" --out "$work/monster-classification.json"
```

Output musi być nowy. Generator weryfikuje bajty plików źródłowych względem przypiętych blobów Git; nie wykonuje callbacków Lua. Katalog przechodzi zamknięty JSON Schema, kompletność, SHA, kontrolę dowodów pól oraz relacji między identyfikatorami. Plik CSV w evidence jest widokiem do przeglądania; źródłem metadanych pozostaje katalog JSON i jego dowody.

Bieżący snapshot zawiera 1697 definicji. Role nieustalone pozostają jawne; suma kategorii może przekraczać populację. Źródła pochodzą z publicznych Canary/Crystal odczytanych jako przypięte lokalne cache Git i zachowanych wejść Encounter; nie przeprowadzono nowego odczytu wiki ani operacji Remote Desktop. Promocja metadanych do runtime/eksportu i wpływ klasyfikacji na reguły obowiązkowych pól pozostają pracą architektury/integracji w #162.
