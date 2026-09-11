# PR #571 — archiwum rozmowy i audytu produktowego

- Data: **2026-09-11**
- Repozytorium: `Oteryn/Oteryn-Game`
- Audytowany PR: `#571`
- Audytowany head PR: `b2ef322791520130a2683f3004d26abd5c027ac8`
- Odczytany protected `main`: `c6103b325748fd31268c7b332defcddf76c59a66`
- Status: **NON-AUTHORITATIVE CHAT/AUDIT EVIDENCE**
- `IMPLEMENTATION_AUTHORITY: NONE`
- `PRODUCTION_AUTHORITY: NONE`

## Cel archiwum

Ten plik zachowuje kontekst właścicielskiej rozmowy, z której powstał audyt PR #571, oraz wskazuje bajtowo zachowane artefakty audytu. Nie jest ADR-em, kontraktem, task allocation ani decyzją o wdrożeniu. Nie zastępuje live GitHub state, accepted architecture, Reference evidence ani repository governance.

To jest **archiwum treści merytorycznej rozmowy**, a nie eksport wewnętrznych instrukcji, narzędzi ani ukrytego rozumowania modelu. System/developer instructions, tool payloads, tajny chain-of-thought i techniczne dane sesji nie są częścią user-visible project state i nie są tu publikowane.

## Wiadomości właściciela w tym wątku

### 1. Zlecenie audytu

> https://github.com/Oteryn/Oteryn-Game/pull/571 dokonaj audyt tych pomyslow, przeszukaj global tibnie, reeddit, wikipedie i inne zrodla i coen ich sens, pomysl opis i zaproponuj zmiany w oparciu i nasz oteryn server i game client

Interpretacja wykonawcza: audyt wszystkich pomysłów zapisanych w PR #571, porównanie z Global Tibią, materiałami społeczności, wiki i innymi źródłami oraz zaproponowanie zmian dopasowanych do architektury Oteryn Game i native client.

### 2. Polecenie domknięcia

> dokoncz ten audyt

Skutek: zakres został domknięty do pełnego rejestru wszystkich **192 wpisów źródłowych** obecnych w master roadmapie: 128 historycznych, 57 forum/QoL i 7 research-only. Każdy wpis otrzymał rekomendowaną dyspozycję i krótkie uzasadnienie.

### 3. Polecenie zapisania całej pracy

> zapisz wszystko z calego czatu do repo

Skutek: pełny audyt i pełny rejestr decyzji zostały zapisane jako izolowane evidence artifacts na osobnej gałęzi dokumentacyjnej; istniejące pliki PR #571, runtime i accepted contracts nie zostały nadpisane.

### 4. Wymóg rozdzielenia lekkiego QoL od zmian gameplayu

> napewno trzeba ustalic rzeczy ktore sa relatywnie prostymi i drobnymi zmianami ktore nie maja ogromnego wplywu na tibia reference a sa bardziej Quality of life na te ktore juz bardzo wchodza w mechanike gamplay itp

Skutek: zaproponowano drugą, niezależną oś klasyfikacji `Semantic Impact`:

- `Q0` — presentation QoL;
- `Q1` — interaction QoL;
- `G1` — gameplay-adjacent semantic change;
- `G2` — core gameplay/progression/economy change.

Oś ta nie zastępuje `S/A/B/C/R`; ma zapobiegać sytuacji, w której wysoki priorytet produktu jest mylony z niskim ryzykiem semantycznym.

### 5. Wymóg przeprojektowania pomysłów tak, aby gra była lepsza dla graczy

> dodatkowo potrzebuje audyt teyuch rzeczy co mozna relanie lepiej zrobic nizeli byly one opisane aby nasza gra byla dobra dla graczy

Skutek: wykonano dodatkowy audyt `Better-for-Players`, którego wyniki są zapisane w:

`OTERYN_PR571_QOL_VS_GAMEPLAY_AND_PLAYER_BETTER_DESIGN_AUDIT_2026-09-11.md`

Audyt nie tylko klasyfikuje wpływ funkcji, ale proponuje lepsze projektowo rozwiązania dla wszystkich 19 programów, w szczególności Activity Plan, Social Assist Group Finder, Prepare/Finish Hunt, Activity Loadouts, Quest/Access Navigator, Death Report, transparentne boss progression/pity, Activity Contracts oraz ograniczenie liczby obowiązkowych pionowych warstw progresji.

### 6. Polecenie zapisania dalszej rozmowy

> zapisz toi wszystko o czym piszemy tutaj do repo

Skutek: dodatkowa warstwa `QoL vs Gameplay Impact + Better-for-Players Design Audit` została zapisana na tej samej izolowanej gałęzi archiwalnej.

## Zachowane wyniki rozmowy

Pełna treść merytoryczna obu rozbudowanych odpowiedzi audytowych została skonsolidowana w `OTERYN_PR571_PRODUCT_AUDIT_2026-09-11.*`. Dokument obejmuje:

- podstawę dowodową i ograniczenia audytu;
- końcowy werdykt wobec PR #571;
- relację z pierwszą granicą Reference po server-save 2026-07-28;
- korektę `Personal Hideouts` względem accepted `EXP-HOUSES-01` Residence/physical-house model;
- relację Rested / no-delevel / `DeathDebt` / exhaustion / corpse recovery z draftem #295;
- rozdzielenie SHARED-UX od zmian informacji, timingu i gospodarki;
- konsekwencje World/Channel dla spawnów, party i boss rewards;
- rozdzielenie priority-of-architecture od feature-delivery order;
- odczytany stan native client/server bootstrap;
- badanie CipSoft, TibiaWiki, Reddit, TibiaPal i Wikipedii z klasyfikacją siły dowodowej;
- proponowane docelowe opisy Equipment Loadouts, Build Comparison, Offline Exercise, reservations, Hunt Accounting, Huntfinder, quests/collections, boss rewards, Prey/Imbuements, Residence, Market/Economy i Security/Premium;
- dyspozycję wszystkich 19 programów;
- proponowaną kolejność F0–F3 oraz minimalne scenariusze odbioru;
- koncepcję integrującą `Plan aktywności`;
- listę brakujących danych i konsekwencji tych braków;
- pełną tabelę 128 historycznych propozycji;
- pełną tabelę A01–A18, B01–B19, C01–C02 i D01–D18;
- pełną tabelę siedmiu research-only candidates.

Pełny rejestr maszynowy został zachowany bez zmian jako gzip podzielony na pięć części. Po złączeniu i rozpakowaniu odtwarza `oteryn_pr571_rejestr_decyzji.json`, zawierający komplet 192 wpisów wraz z `source_group`, `programmes`, `disposition` i `recommendation`.

Dalsza warstwa audytu po właścicielskim doprecyzowaniu QoL/gameplay i potrzeby lepszego designu została zapisana jako:

- `OTERYN_PR571_QOL_VS_GAMEPLAY_AND_PLAYER_BETTER_DESIGN_AUDIT_2026-09-11.md`.

Ten dokument dodaje:

- klasy `Q0/Q1/G1/G2`;
- trzy pytania do klasyfikacji semantic impact;
- shortlistę Safe QoL Wave;
- listę funkcji, które wyglądają jak QoL, ale zmieniają timing/economy/gameplay;
- ulepszony design wszystkich 19 programów;
- Activity Plan jako UX spine;
- lepszy Group Finder;
- zasadę, że core social QoL nie powinien być sztucznym premium wall;
- Activity Loadouts z rozdzieleniem `Plan` i `Active`;
- zakaz redukowania buildów do jednego Gear Score;
- `Prepare Hunt / Finish Hunt` flows;
- explainable Huntfinder z confidence/freshness;
- Quest Navigator z poziomami spoilerów;
- Account-World progression zachowującą tożsamość postaci;
- Practice/Normal/Challenge encounter model;
- Death Report;
- Activity Contracts zamiast czystego weekly FOMO;
- Residence jako lifestyle/social identity hub;
- source/sink accounting;
- progression-layer budget;
- zasadę alt-friendly, not alt-mandatory;
- balance przez asymetrię klas bez wykluczania klas z całych kategorii contentu;
- nowe wymagane pola roadmapy: `PLAYER_PROBLEM`, `SEMANTIC_IMPACT`, `BETTER_DESIGN`, `DO_NOT_CROSS`, `SUCCESS_METRIC`, `REFERENCE_BASELINE_OR_DECLARED_DIFFERENCE`, `OWNING_SYSTEM/CONTRACT`.

## Najważniejsze wnioski z rozmowy

1. **PR #571 ma sens jako katalog produktu, ale nie jako zbiorcze uprawnienie do wdrożenia 19 programów.**
2. **Najwyższą wartość mają spójne przepływy przygotowania i gry:** klient, input, loadouty, inventory/stash, party/lobby, rozliczenia, czytelna progresja i Offline Exercise.
3. **Offline Exercise zachowuje właścicielską decyzję o równoległym treningu wielu postaci konta**, gdy każda legalnie zużywa własne zasoby; nie wprowadzono limitu jednej postaci na konto.
4. **Equipment Loadouts są jednym systemem** dla presets/protection sets/full-set hotkeys/ring A/B, ale atomowość serwerowa nie może sama nadawać nieograniczonego natychmiastowego swapu bojowego.
5. **Slot imbuementu jest częścią budżetu wartości przedmiotu.** Nowe Attunement/Forge/+N/proficiency nie powinny bezwarunkowo nakładać się jako osobne obowiązkowe warstwy mocy.
6. **Huntfinder i zajętość mają koordynować, nie tworzyć własności respawnu ani publicznego GPS.** Dynamic Spawn pozostał badawczy.
7. **Personal loot, pity i Boss Essence muszą dzielić jeden budżet podaży nagród.** Wielokanałowość encounteru nie oznacza automatycznie wielokrotnej eligibility.
8. **Quest access, quest completion, choices i reward eligibility są różnymi stanami.** Shared legacy access nie oznacza kopiowania wszystkich flag.
9. **Collection Framework współdzieli UI/primitives, nie musi scalać wszystkich kolekcji w jeden autorytatywny licznik.**
10. **Housing ma zużywać accepted `EXP-HOUSES-01`.** Hideout nie powinien tworzyć trzeciej konkurencyjnej osobistej nieruchomości obok Residence i physical house.
11. **Death/Recovery musi zostać uzgodniony z #295 i FND-04**, zamiast tworzyć drugi niezależny system długu/odzysku/reconnectu.
12. **AI w anti-bot/economy jest pomocnicze.** Deterministyczna authority, evidence i bounded policy pozostają po stronie systemu.
13. **Pierwszy Reference target jest już wybrany:** Global Tibia production-observable behavior po granicy 2026-07-28. Późniejsze aktualizacje Global nie przesuwają automatycznie tej wersji.
14. **Najlepszy wyróżnik integrujący roadmapę:** `Plan aktywności` — wybór celu → wymagania → loadout/zapasy → grupa → gra → settlement/restock — bez tworzenia dwudziestego systemu progresji.
15. **Roadmapa musi mieć dwie osie:** `Priority` i `Semantic Impact`. `S` nie oznacza automatycznie „bezpieczne do Reference”.
16. **Safe QoL Wave powinna wyprzedzać ciężkie G2 redesigny**, jeśli live dependencies/authority na to pozwalają.
17. **Nie każdy pomysł literalnie opisany przez graczy jest najlepszym rozwiązaniem problemu gracza.** Należy zachować problem, ale przeprojektować rozwiązanie, gdy można uzyskać lepszy UX, mniejszą złożoność i mniej niechcianych skutków ubocznych.
18. **Mniej obowiązkowych warstw pionowej mocy, więcej jakościowych wyborów i explainable systems.**

## Wynik klasyfikacji 192 wpisów źródłowych

| Dyspozycja audytu | Liczba |
|---|---:|
| `ZACHOWAĆ` | 54 |
| `ZMIENIĆ` | 69 |
| `PÓŹNIEJ` | 25 |
| `BADANIE` | 31 |
| `UZGODNIĆ` | 10 |
| `ODRZUCIĆ` | 3 |
| **Razem** | **192** |

Te liczby dotyczą wpisów źródłowych, w których część propozycji powtarza ten sam system. Nie są liczbą 192 unikatowych funkcji i nie stanowią nowych owner decisions.

## Granice tego zapisu

- Nie zmieniono runtime, protokołu, bazy, produkcji, ruleset ani accepted architecture.
- Nie zmieniono plików head PR #571; archiwum jest path-disjoint evidence.
- Nie wykonano testów przyszłych mechanik ani nie zatwierdzono ich parametrów.
- Nie odtworzono pełnego pochodzenia każdego pierwotnego forum request A–D, ponieważ URL-e źródłowe nie były zapisane w audytowanym inwentarzu.
- Nie ujawniono system/developer instructions, tool internals ani hidden chain-of-thought.
- Rejestr i pełny audyt pozostają rekomendacją audytową do późniejszej właścicielskiej/architektonicznej selekcji.

Zobacz manifest obok tego pliku, aby odtworzyć bajtowo oryginalne artefakty oraz zweryfikować SHA-256.
