# Import danych Itemów i surowych obserwacji FX/audio

**Dane są w canonical content repozytorium serwera.** Paczka Itemów:commit `4151e3005daa41ed848533e45b63fadc5ed78fa3`, draft PR1562. Surowe dane FX/audio:commit `d8d449751b77dcdabdae3a44938472f058fcb14f`, [draft PR1599](https://github.com/Oteryn/Oteryn-Game/pull/1599); Root `VALIDATE`.

| Zaimportowane dane | Stan |
| --- | ---: |
| Wszystkie rekordy Native / Itemy | 57320 /34031 |
| Znane opisy / nazwy | 1629 /12100; 15 nazw poprawionych |
| Source ItemAuthoring / Forge / BRbindings | 700 /436 /454 |
| Relacje:źródła / referencje | 539 /752 |
| Profile / szablony rodzin | 22 /23 |
| Import batches / stany reimportu | **14 /357** |
| Nowe surowe obserwacje | **295 ID +1 kontekst =296 Text stanów** |

Poprzednia paczka zmieniła Native na1655 Itemach i dotknęła1859 targetów wraz z Source. Nowy import przechowuje surowe tekstowe dowody źródłowe; nie tworzy Native efektów, assetów ani runtime bindings. Zachowano wszystkie13 poprzednich paczek/61stanów, rekordy Native, właścicieli Source,69 shardów Itemów i relacje. Pełne bajty 19 plików nowego commitu odpowiadają manifestowi i Root remote readback.

Źródła FX/audio odczytano wcześniej zwykłym publicznym GitHubHTTP z przypiętych Canary/Crystal rewizji. Ten suplement korzysta wyłącznie z zachowanych danych i Gitblobów. Metody BR/Fandom i ograniczenia dostępności pozostają w raporcie poprzedniej paczki.

**Gotowość runtime, odczyt danych przez działający proces serwera i globalna liczba ukończonych Itemów pozostają `null`.** Pozostałe kohorty581Forge/2493stack/656imbuement/14regen/8Physical/235family pozostają osobnymi, nakładającymi się zakresami. Nie sumujemy ich. Dane Itemów mają69 wspólnych liczbowych shardów; profile rodzin nie są osobnymi katalogami Itemów.

Recovery:pełny Qualified archive `combined-item-published-server-data-4151e300-exact-checkpoint.tar.gz` (SHA d7b1cbf1…), finalworker, Source64v6, niezależne peers i opublikowany raport są wymienione z pełnymi hashami w JSON. Status osobnego archiwum Raw jest wskazany jawnie, bez deklarowania nieistniejącego backupu.
