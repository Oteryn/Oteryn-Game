# Pozostała praca nad Itemami — 2 października 2026 (UTC)

Stan opublikowany: `1b026969a8b2236737bff537b3eddb3c909781b2`. Dane poniżej odczytano z rzeczywistych blobów tego commitu. Przygotowane paczki nie są jeszcze wliczone jako opublikowane.

Obecnie jest **34 031 rekordów Item** w **57 320 rekordach Native**, przechowywanych w **69 wspólnych shardach**. ItemAuthoring ma **411 właścicieli metadanych**, w tym **147 profili Forge**, **139 obserwacji użycia** i **103 pola metadanych broni**. Opublikowane są już Weapon103 i DefaultFalse8. Opisów Native jest **0**, znanych nazw **12 100**; wektorów modyfikatorów **494**, atomów **796**. Te liczby nie oznaczają kompletnych Itemów.

| Przygotowana paczka | Zakres | Co pozostało |
| --- | --- | --- |
| Description1629 | 1629 Itemów / 1629 literalnych opisów; niezależny przegląd implementacji PASS | Wspólna kompozycja, kompilacja, materializacja, kontrola dokładnego delta i publikacja |
| Forge289 | 289 nowych profili źródłowych, 578 pól, 289 powiązań BR | Poprawka izolacji historycznej reprodukcji PASS; świeży przegląd implementacji, wspólne walidacje i publikacja |
| Family10 | 10 szablonów; obecnie13, po publikacji23; 22 profile | Publikacja kontraktu/szablonów. To nie jest10 nowo sklasyfikowanych Itemów |
| Modifier17 | 17 całych wektorów / 50 atomów, źródła zakwalifikowane | Implementacja, niezależny przegląd, kompilacja/materializacja i publikacja |

Trzy paczki zmieniające fakty Itemów obejmują razem **1844 różnych targetów**, z nakładaniem się zbiorów. To zakres przygotowanej pracy, nie liczba wszystkich nieskończonych Itemów. Po pomyślnym zakończeniu projektowane są: **1629 opisów**, **700 właścicieli ItemAuthoring**, **436 profili Forge**, **511 wektorów /846 atomów**, **539 źródeł /752 relacje**. Liczba rekordów Item i ich materializowalność pozostają takie same.

| Blokada | Zakres | Czego brakuje |
| --- | --- | --- |
| Maksymalny stack | 2493 targety:2491 typedUNKNOWN +2 identity-only, aktualnie ponownie sprawdzone | Własny maksymalny stack albo jawna reguła bez wyjątków dla tych wariantów. **0 nowych maksimum**; nie wolno przyjąć100 |
| Forge | 870 w zachowanej kohorcie bez profilu; warunkowo581 po Forge289 | Jawna para klasy/maksimum i zaakceptowane dokładne powiązanie źródła. Strona znaleziona po nazwie nie wystarcza |
| Regeneracja | 14 wektorów, **0 zakwalifikowanych** | 8 braków własnego XML fazy, 4 współdzieloneID/fazy, 1 różny kwant ticka, 1 sprzeczna mana/brak health; nie uśredniać |
| Physical8 | 8 wektorów /16 atomów | Decyzja wersji/profilu codec, pełny dowód kompatybilności starego profilu i pomiar właściwego artefaktu. Prototyp nie jest ukończeniem |
| Imbuement | 656 Itemów ze znaną liczbą slotów; **0 zamkniętych list rodzin/tierów** | Pełna lista dopuszczeń i wykluczeń. Dwa fakty częściowe (28715,29427) wymagają osobnego kontraktu Source; nie zamykają listy |
| Nazwy / BR | 15 propozycji nazw +3 mostyBR, **0 zastosowanych** | Name15: zamknięta kwalifikacja poprawki zaakceptowana, implementacja oczekuje. BR3: naprawa klucza kanonicznego, zachowanie konfliktów i świeży przegląd; family15 NAV nadal wstrzymane |
| Rodziny | **235** otwartych przypisań w zamrożonym overlay12252/12487 | Dokładna tożsamość/kategoria i integracja właściciela. Family10 nie zmniejsza tej liczby |
| FX/audio | Brak nowego pomiaru kompletności po1b | Powiązania assetów/zdarzeń/celu efektu oraz kontrakt i dowód runtime. Istniejące formalne pola nie tworzą automatycznie działania |

Dalsze zachowane blokady obejmują mobilność4091 (TakeABSENT nie oznaczaFalse ani domeny portable), augmenty62/77 klauzul (brak typowanych powiązań czarów), cztery dokładne konflikty wag oraz trzy błędne komórki zasięgu. Są to **ograniczone diagnozy źródłowe z wcześniejszego raportu4f**, nie nowe sumy wszystkich braków. Typowane pola dla światła, użycia, chargów, proficiency, czasu życia i zewnętrznych zależności wymagają osobnej kwalifikacji źródła/owner/runtime; szczegółowe kontrakty są w wcześniejszym raporcie `item-import-gaps-master-20261002.md`.

**Nie ma jeszcze wiarygodnej liczby Itemów kompletnie ukończonych ani wszystkich nieskończonych**: oba wyniki pozostają `null`. Zbiory nakładają się, a UNKNOWN może dotyczyć pola opcjonalnego, braku źródła, domeny albo działania runtime. Historyczne „2521” i ogólne „3k” nie są aktualnym bilansem.

Wszystkie22 rodziny mają profile katalogu. Aktualnie13 szablonów będzie23 dopiero po przygotowanej Family10. Kanoniczne definicje nadal są w69 shardach ID, bez oddzielnych22 katalogów Itemów według rodziny. Zamrożony overlay klasyfikacji nie jest klasyfikacją Native; aktualny source-navigation eksport ma164 wiersze, a typed Nativeclassification ma0 Known. Nie są to zamienne miary.

Źródła tego bilansu: lokalne bloby opublikowanego Git1b oraz zachowane proofy/checkpointy (pełne SHA w JSON). Zachowane wiki czytano przez zwykły HTTP, a po402/403 przez publiczną przeglądarkę Chrome/CDP; publiczne XML pobrano z pinned GitHub przez zwykły HTTP. Ten bilans nie wykonuje nowych odczytów internetu. Cutoff rewizji nie potwierdza aktualności wszystkich stron live. Chroniony artefakt V4 ma38157 rekordów w innej przestrzeni niż34031 aktualnych Itemów; różnica4126 nie jest backlogiem.
