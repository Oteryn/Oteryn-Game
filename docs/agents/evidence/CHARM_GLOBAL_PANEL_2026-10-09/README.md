# Charmy — obserwacje Global i poprawki walidacji, 2026-10-09

Źródłowa wersja katalogu: `Oteryn/Oteryn-Game@1e77ca22b4fcb7869f8f1d169f4e12bc749b295d`. Właściciel polecił obejrzenie panelu bez wydawania punktów i następnie zapisanie pracy do repozytorium.

## Potwierdzone obserwacje

1. Odczytano wszystkie 25 charmów: 14 major i 11 minor. Kategorie, waluty, koszty i główne wartości pierwszego etapu są zgodne z katalogiem. Opisy w observations.json są parafrazami widocznego tekstu.
2. Adrenaline Burst opisuje wzrost prędkości o 150% na 10 s. Cleanse opisuje jeden losowy negatywny efekt oraz 11 s odporności. Cripple/Numb opisują 10 s paraliżu, Fatal Hold 30 s zapobiegania ucieczce.
3. Charm Expansion opisuje dowolną liczbę przypisanych odblokowanych charmów, 25% zniżki na usuwanie, ograniczenie do kupującej postaci i jeden zakup. Wyświetlona cena: 450 Tibia Coins. Obejrzano wyłącznie opis.
4. Panel pokazuje 2 dostępne przypisania. Dragon: 30 zabójstw, 0/3, Assign Charm nieaktywny. Nie dowodzi to całej reguły kategorii przypisań ani limitu premium.
5. Salda pozostały bez zmian. Nie wykonano unlock, upgrade, assign, unassign, reset ani zakupu.

## Granice dowodu

- Etapy 2–3 nie były dostępne do podglądu przy zablokowanych charmach. Ich zgodność z Global pozostaje UNKNOWN.
- Clear/reset były nieaktywne i pokazywały 0 gold; nie jest to dowód zerowej rzeczywistej opłaty.
- Nie przeprowadzono walki. Odporności, armor, capy, geometria AoE, wielocelowe proci oraz dokładne wzory critical/leech pozostają poza obserwacją.
- Dowód tekstu panelu nie zatwierdza formuł ani całych hipotez emulatorów. Proposed-fact-additions.json jest odrębnym kandydatem dowodowym; nie zmienia istniejących pinów ani accepted rulesets.
- Zrzuty zawierające grafikę klienta pozostają w pakiecie offline zgodnie z LICENSE-ASSETS.md; repo przechowuje identyfikatory i SHA256.

## Przygotowane poprawki

1. Schemat: procenty o dokładności 0.01, limity u32 dla kosztów/czasów/mnożników i klucz o maksymalnej długości 128 ASCII bajtów.
2. Validator: dokładna walidacja setnych przez Decimal, bez błędów binarnego zaokrąglania lub wyjątków dla skrajnych/nieskończonych wartości. Dodano przypadki graniczne i negatywne. Wartości 25 charmów i wygenerowany content są niezmienione.
3. Receipt-null-guard-draft.sql jest wyłącznie projektem NOWEJ migracji uszczelniającej NULL w receipt odblokowania. Nie jest zarejestrowaną migracją, nie zastępuje 0020 i nie był zastosowany do bazy. Badane na offline PostgreSQL 18.3/PGlite: 16 sprawdzeń; pełna kwalifikacja na docelowej bazie pozostaje wymagana.
4. Starsze prototypy silnika/widoku i ich wyniki dotyczyły f21a45d1; nie stanowią kwalifikacji obecnego gameplayu. Nie przeniesiono ich jako nowej implementacji runtime.

## Walidacja

Offline patch na 1e77ca22: 13 funkcji autora, 27 testów mechanik, 9 testów dowodów źródłowych; build --check, validate, Ruff i patch apply --check zakończone powodzeniem. Plik offline-validation.json zachowuje ten ograniczony wynik. Testy repozytorium są uruchamiane ponownie po zapisaniu zmian. Nie jest to wynik CI ani potwierdzenie integracji do protected main.

## Pierwszy etap — odczytane wartości

| Charm | Kategoria | Koszt | Główna wartość (%) |
|---|---|---:|---:|
| Adrenaline Burst | minor | 100 Echoes | 6 |
| Bless | minor | 100 Echoes | 6 |
| Carnage | major | 600 Points | 10 |
| Cleanse | minor | 100 Echoes | 6 |
| Cripple | minor | 100 Echoes | 6 |
| Curse | major | 360 Points | 5 |
| Divine Wrath | major | 600 Points | 5 |
| Dodge | major | 240 Points | 5 |
| Enflame | major | 400 Points | 5 |
| Fatal Hold | minor | 100 Echoes | 30 |
| Freeze | major | 320 Points | 5 |
| Gut | minor | 100 Echoes | 6 |
| Low Blow | major | 800 Points | 4 |
| Numb | minor | 100 Echoes | 6 |
| Overflux | major | 600 Points | 5 |
| Overpower | major | 600 Points | 5 |
| Parry | major | 400 Points | 5 |
| Poison | major | 240 Points | 5 |
| Savage Blow | major | 800 Points | 20 |
| Scavenge | minor | 100 Echoes | 60 |
| Vampiric Embrace | minor | 100 Echoes | 1.6 |
| Void Inversion | minor | 100 Echoes | 20 |
| Void's Call | minor | 100 Echoes | 0.8 |
| Wound | major | 240 Points | 5 |
| Zap | major | 320 Points | 5 |
