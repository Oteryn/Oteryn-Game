# Items Cyclopedii — wspólny zapis offline

Zapis łączy dostępne dane lokalnego klienta 15.33 z wcześniej odczytanymi kartami.
Do korzystania z tych plików nie potrzeba zalogowanej sesji. Jest to baza dowodów
i uzupełnień danych, a nie automatyczne przyjęcie danych do runtime Oteryn.

## Co zapisano

| Zakres | Pokrycie |
|---|---:|
| Lokalne obiekty powiązane z Cyclopedią | 6470 appearances / 6228 typów |
| Nazwy i zachowane odczytane flagi | 6470 nazw / 54 klucze / 59546 wystąpień |
| Dodatkowe surowe tagi 60–64 | wszystkie obecne wartości w tych 6470 rekordach |
| Odwołania do proficiency | 667 odwołań / 431 profili |
| Lokalne profile proficiency | 444 profile / 2216 poziomów / 3680 perków |
| Główna pula odczytów UI | 2623 karty / 10 kategorii / 7736 pól |
| Starsze odczyty Armors/Amulets i wyszukiwań | 314 obserwacji źródłowych, z nakładaniem |
| Zachowane zestawy dopasowań obrazu do klienta | 797 obserwacji; wszystkie kwalifikacje i niejednoznaczności |

W głównej puli zapisano Weight dla 2623 kart, Armor dla 242, Protection dla 150,
Mantra dla 15, Attack i Defence dla 4. Pozostałe pola i pokrycie każdej kategorii
są w gaps.json. Nie każde pole ma zastosowanie do każdego itemu; nieobserwowanego
pola nie zastępujemy zerem, false ani N/A.

Główne kategorie: Creature Products980, Decoration462, Food169, Gold3,
Helmets and Hats128, Legs66, Others603, Boots67, Containers109, Potions36.
Dla pierwszych dziewięciu zachowano dowód końca odczytanej listy Basic.
Potions36 jest zakresem częściowym z nieznaną całkowitą liczbą pozycji.
Koniec listy Basic nie dowodzi kompletności NPC, animacji ani zgodności runtime.

Starsze 314 obserwacji to 33 wcześniejsze odczyty, 138 odczytów Basic,
51 nakładających się ponownych odczytów, 87 Amulets i 5 wyszukiwań celowanych.
Nie jest to 314 różnych itemów. Globalna kolejność tych obserwacji jest UNKNOWN;
warianty wolf/rain i enchanted theurgic pozostają osobno.

## Jak czytać dane

- catalogue-index.json: indeks wszystkich 6470 appearances i referencje do
  zachowanych obserwacji UI; zawiera hashe wejść i zależności.
- client6470-flags.json: nazwy, surowe hashe rekordów, sparse flagi z jawną
  obecnością oraz metadane opisów. Brak wpisu flagi oznacza FIELD_ABSENT.
- client6470-five-tags.json: obecność i wartości tagów 60–64 oraz profile.
  Lista tag62 zachowuje kolejność i jawne zera. Etykiety kolektora nie dowodzą
  znaczenia kodów profesji lub typów broni.
- proficiency-profiles.json: wszystkie lokalne wartości liczbowe perków,
  wersje i stan kwalifikacji; 3679 przechodzi istniejący codec source-only,
  jeden perk pozostaje HELD. Nazwy są przechowane przez hash.
- ui2623-observations.json: literalne pola kart i ich stany, opisy przez
  custody oraz opaque identyfikatory dowodów i SHA256.
- legacy-armors-amulets.json: wcześniejsze obserwacje ze swoim zakresem
  źródłowym; nie należy dopisywać ich do mianownika różnych itemów.
- client-ui-relations.json: istniejące kwalifikacje statycznego obrazu,
  w tym zestawy kandydatów i HELD; bez przypisywania serverID.
- gaps.json: pola i kategorie oraz pozostałe grupy braków.

Referencje UI w indeksie są indeksami wierszy liczonymi od zera.
Wszystkie 797 relacji sprawdzono przez kategorię, ordinal i hash dowodu obrazu.
753 mają zapis jednego kandydata, 29 zestaw kandydatów, 15 pozostaje HELD.
Jednoznaczny kandydat nie staje się automatycznie tożsamością serwerową.
813 lokalnych appearances występuje w tych zestawach; to nie liczba itemów
z kompletnymi statystykami. Pusta lista relacji nie oznacza braku dowodów globalnie.

Pełne surowe oferty NPC, aliasy i wcześniejsze porównania do źródeł Oteryn są
zależnościami o przypiętych hashach z PR3170, commit
54c930ab3771d28d7d5584beed6b3e69125bf5a7. Indeks podaje dokładne ścieżki.
Dotyczy to 14235 ofert w bieżącej puli, 15350 globalnie. Nie kopiujemy tych
samych wielomegabajtowych tabel. Te zależności są w niepołączonym draft PR;
nie zakładamy, że znajdują się już na main. Nazwa pomocnicza
npc-whitelist-lossless-columns.json wskazuje na client-npc-tuples.json.
Identyczne nazwy i grafiki oraz appearance/Cyclopedia IDs nie dowodzą serverID.

## Czego nadal brakuje

1. Pełnych statystyk dla pozostałych kart. Pliki lokalne nie zapewniają
   kompletnego Weight/Armor/Attack/Defence/Protection/Mantra dla każdego itemu.
   Zebrane wartości zachowano; nie wyliczamy brakujących z nazwy lub grafiki.
2. Dokończenia zakresu Potions, globalnej kolejności Armors/Amulets oraz
   potwierdzenia wszystkich pozostałych kategorii UI. Nie odejmujemy 2623
   obserwacji od 6228 typów: są to różne populacje.
3. Rozstrzygnięcia niejednoznacznych i wstrzymanych relacji obraz–appearance,
   a osobno kwalifikacji powiązania z itemami źródłowymi/serwerowymi Oteryn.
4. Dwóch opisów klienta z nieznanym kodowaniem: appearance9093 i11541;
   niepewnego glifu opisu Others547; znaczenia części kodów liczbowych.
5. Znaczenia/jednostki jednego nowego perku: profil499, poziom3, perk0,
   Type33, raw Value0.05. Nie interpretujemy tego automatycznie jako5%.
6. Pełnej prezentacji list NPC, kwalifikacji waluty/ilości i znaczenia
   surowych zer cenowych, pełnych animacji oraz danych zależnych od instancji
   i sesji. Surowe lokalne dane nie dowodzą wszystkich tych zachowań.

Opis nieobecny w źródle (3697) nie jest brakującym odczytem; 2771 poprawnych
opisów zachowano prywatnie, a dwa niepoprawnie zakodowane oznaczono UNKNOWN.
Opisów narracyjnych, screenshotów, ścieżek obrazów i oryginalnych assets nie
ma w repo. Prywatne powiązanie opaque dowodów z plikami pozostaje na Ubuntu.
Ten zapis nie wymaga nowych kliknięć, logowania ani uruchomienia gry.

## Walidacja i stan

Uruchom python3 docs/agents/evidence/items-offline-consolidated-20261113/validate.py --self-test.
Sprawdza hashe, obecność/zero, referencje, kwalifikacje i granice prywatności.
Zmiana nie modyfikuje runtime, content, pins ani dotychczasowych PR.
Publikacja jest draftem; ogólny audyt statycznych zakładek pozostaje PARTIAL.
