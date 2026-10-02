# Physical8 — co jeszcze wymaga pracy

Stan źródłowy po opublikowanym Use345 (`5ab88af6d43bff444ed85782a49d9d910855152d`): **8 itemów / 16 atomów, nadal wstrzymane**. ID: 10389, 17828, 50164, 50165, 50181, 50239, 50273, 52335. Wszystkie osiem aktualnych grup native ma `UNKNOWN`; żadnej nie uzupełniono częściowo. Źródłowe wektory pozostają zachowane w dowodzie57 (`fece04…`) i paczce49 (`eecfca…`).

Brakuje obsługi `ReferenceModifierElement.PHYSICAL`. Obecny zakres to Death1, Earth2, Energy3, Fire4, Holy5, Ice6. Istniejący Physical10 w osobnym enumie odporności nie zastępuje tego typu. Odzyskane cztery pliki prototypu proponują Physical7 i izolowany profil artefaktu5 (`OTRPA05\0`, body2); nie mają kwalifikacji nowej wersji, testów ani pomiarów dla v5.

Obecny eksport zawiera **34 031 Itemów / 57 320 rekordów wszystkich rodzin**. Zbiór identyfikatorów Item w reference i canonical jest identyczny. Stary artefakt v4 ma inny zakres: chronioną rodzinę B1 **38 157 rekordów**, tworzoną w testach przez osobny importer. Walidator wymaga dokładnie tej liczby. Materializer obecnych danych zapisuje JSON `CanonicalProjectDocuments` i nie wywołuje kompilatora artefaktu. Różnica 4126 nie jest liczbą brakujących itemów. Udane testy starego importera nie są dowodem kwalifikacji obecnego grafu do tego artefaktu.

| Zakres pracy | Konkretny brak | Wielkość |
|---|---|---|
| Kontrakt wersji i pomiary | Dyspozycja właściciela dla profilu5/domain7, dokładne wejście nowego artefaktu, metadane schema/package/compiler/canonicalization i własne dowody limitów | Jedna paczka decyzji i pomiarów; starego v4/38 157 nie zmieniać |
| Kodek i zgodność | Minimalna aktualna poprawka enumu, obu projekcji, selektora profilu, encode/decode i niezależnego oracle | Cztery pliki prototypu do ponownego opracowania; testy stanowią większą część pracy |
| Źródła i import danych | Nowe successor receipts/packets dla zmienionych pełnych pinów; zamknięty atomowy import całych8 wektorów i komplet bieżących guardów | Mała paczka danych8/16, większa praca z zależnościami |
| Kwalifikacja końcowa | Niezależny review, testy zgodności i zasobów, pełny diff, testy repozytorium/Clippy/fmt oraz deterministyczne generowanie | Osobny checkpoint po zamrożeniu implementacji |

Najważniejsze testy:

- Stare wartości1..6 i profilev1–v4 mają identyczne bajty/goldeny. V5 przyjmuje dokładnie7; 0/8/255 nadal odrzuca. V4 nie przyjmuje7 przy użyciu rekordu.
- Zachować leniwy kontrakt v4: odczyt nagłówka/indeksu przed dekodowaniem body. Artefakt v4 z poprawnymi skrótami i7 w body może dojść do lookup; wtedy musi zostać odrzucony. Nie zastępować tego skanowaniem wszystkich rekordów podczas load.
- Oba pełne artefakty zachowują wszystkie16 atomów, także dla `materializable=false`, wcześniejsze pola i `UNKNOWN` dla target_domain/evaluation_phase/priority. Brak filtrowania, fallbacku do v4 lub utraty Known.
- Osobne testy starego i nowego wejścia rodzinnego: dokładna liczba rekordów, ±1, duplikaty, pełne pokrycie, profil/version/manifest/pair mixing oraz max/max+1 wszystkich przyjętych wymiarów.
- Fakty źródłowe: all-global ownID, cutoff, pełne identity, exact/reverse binding, membership, nazwy, World exclusions, current known-state i konflikt na końcu paczki. Idempotencja i brak częściowej aplikacji.

Prototyp `item_stats_promotion.rs` jest starszy od bieżącego kodu: jego kopiowanie usunęłoby106 obecnych linii. Trzeba dopisać wyłącznie Physical do aktualnego guardu, zachowując już istniejące Mantra/Bond i ich testy. Prototyp oracle tylko rozszerza parametr `codec(..., artifact_profile=5)`; nie dodaje wywołania v5 do CLI, pomiarów ani testów. Selektor prototypu rozpoznaje tylko Physical Bond; zastosowanie współdzielonego enumu w innych dopuszczonych miejscach musi mieć jawną dyspozycję, bez poszerzania niekwalifikowanych znaczeń.

Aktualny census modifierów to494 wektory/796 atomów. Warunkowy wynik po zamknięciu tej konkretnej paczki wynosiłby502/812; to prognoza, nie wykonane uzupełnienie. Nie zmienia się D278, protokołu ani runtime admission. Rozsądny podział to dwa zakresy review: wersjonowana zgodność/kodek/oracle, potem zamknięty import8 wraz z resealami źródeł.

Research: wyłącznie lokalny odczyt zachowanych plików i kontraktów. Nie wykonano Cargo, testów prototypu, generowania, HTTP ani działań Remote Desktop. Pełne ścieżki, skróty i checklisty: `physical8-implementation-work-inventory-20261002.json`.
