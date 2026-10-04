## NPC — lokalne wyniki i zależności dla architekta/koordynatora (2026-10-04)

Na wyraźne polecenie właściciela przekazuję raport do koordynacji #162. Wpis trafia do oficjalnej kontynuacji #1622, ponieważ #162 osiągnęło limit komentarzy. To handoff wyników i zależności, nie zmiana canonical STATE, przyznanie lease ani uruchomienie konkurencyjnych workerów.

### Zakres i rzeczywiście sprawdzony wynik

Projekt `NPC-COMPLETE-LOCAL-20261004`: sześć torów, osiem zadań, root jako architekt/koordynator lokalnej partii oraz niezależny reviewer. Baza produktu: **`79b79ae99c3b5ca1667c7157b812131ee27c43f9`**. Poniższe braki są ustalone na tej zamrożonej bazie; **nie są twierdzeniem o braku mechanik na aktualnym main**. Przed alokacją/integracją należy uzgodnić je z bieżącym main i aktywnymi pakietami.

- Bazowy kwalifikowany WorldProject: **1282 NPC, 836 Dialogue, 380 Service, 2564 profili**, SHA256 `e749728fc12b0b79a8a296fdb8b8c8393b5dd2c8b4e7ef0115c8836084347ebc`. Są to rekordy danych, nie liczba kompletnych grywalnych NPC.
- W kopii z proponowaną schemą zaimportowano dokładnie **dwa teksty podróży** istniejących tras Hawkhurst/Hawkhurst Ingol. Cztery obserwacje Canary/Crystal, tekst `All Hand Hoy!`. Ceny, destination, premium i pozostałe deklaracje zachowano. Dwie niezależne materializacje identyczne bajtowo, ponowny pinned data-only admission PASS; odmowy driftu SHA, zmiany ceny i zdublowanego celu PASS. Drzewo kopii: `b506d02a8ca6512ae59fc3abce7992352cb49de50a09286a08ff28debf6782f7`. Wymaga przyjęcia proponowanego `departure_text`; nie zmienia bazowego importu produktu.
- **Cztery kandydaty Item: 23374, 236, 238, 7642** przeszły pełne rzeczywiste `CanonicalProjectDocuments::from_v2_draft` i ponowny `ProjectSnapshot::parse`, nie tylko decode DTO. `PENDING`/`LocalNonProduction` zachowane, limit importu **4/24 kandydatów** (26 batchów nie jest tym limitem). Pięć negatywnych przypadków odrzuconych. Opisy pozostają źródłowym Text w imporcie; native Item stats/appearance, EXACT i materializacja nie są promowane.
- Połączona lokalna propozycja kodu: **offline cargo check --lib PASS, 11/11 native admission tests PASS**, niezależny final review PASS. Testy torów: 8 Quest tracks, 6 catalogue/dialogue, 3 candidate placement. Patch SHA256: `32afc94047308fcbdd411bcf28a7a2cccfce25670ef7440cacb38dce03069d0f`.
- Produkt pozostaje czysty. **Zero aktywowanych aktorów/usług, brak gameplay E2E.** Zgodnie z poleceniem właściciela wszystko pozostaje lokalne: bez nowego PR/pusha/wdrożenia. Hashe plikowe powyżej nie są commitowym FREEZE_SHA.

### Gotowe lokalne propozycje do wykorzystania

1. **Content/rozmowy:** istniejący `NpcDataCatalogue` jako właściciel immutable projektu i exact indeksów. Resolver sprawdza oczekiwany tree/revision i własność NPC→Dialogue, rozwiązuje bounded exact keyword path i faktycznie wywołuje native planner. Zachowuje teksty oraz legacy flags. Topic/focus dostarczy prawdziwy actor owner; nie stworzono zastępczego session store.
2. **Quest authoring:** optional nazwane `tracks` w istniejącym `ProjectV2QuestAuthoring`, canonical sorting i dokładne członkostwo Quest/profile/revision. Brak odczytanego progress pozostaje `Unknown` również przy `initial_value=0`. Brak storage-number aliasów. Runtime gate/outcome pozostaje fail-closed bez właściwego konsumenta. To LOCAL PROPOSED amendment, nie zastępstwo aktualnej przyjętej schemy Questa.
3. **Placement:** prywatny candidate payload kwalifikuje dokładne NPC, istniejący placement, Presentation/Behavior i Service refs. Rich fixture z rzeczywistym admission potwierdza rozwiązywanie niepustych profili/usługi i odmowę placementu innego istniejącego NPC. Zachowuje `CandidateOnly`; nie obniża go do executable `PlacementRef`.
4. **Usługi:** przygotowanie spell acquisition na rzeczywistych `SpellDefinition`/`CasterState` i istniejącym planie physical coin fee. Automatic/free spelle nie pobierają opłaty ani nie dostają fikcyjnego grantu; historyczne ceny są informacyjne. Paid preparation nie jest commitem i nie może użyć charm-only fee writer ani domniemanego bank fallback.

### Do architekta / CP — sprawdzić istniejących ownerów i domknąć konkretne zależności

| Zakres | Luka na naszej bazie / czego wymaga konsument | Aktualne uzgodnienie zamiast nowej konkurencyjnej pracy |
| --- | --- | --- |
| Native NPC / mapa / klient | Native family/payload i executable lowering/activation, actor carrier, visibility oraz CHAT dispatch. Authoring placement już umie wskazać NPC — nie potrzeba nowej mapy ani Creature udającego NPC. | Uzyskać zgodność z NPC-BEHAVIOUR-0 z #1733, aktywnym NPC-VIS-1 oraz NPC-REBUILD-1; existing map/bundle owner. STATE D499 wskazuje rozpoczęte NPC-VIS-1. |
| Quest progress / effects | Fenced trwały reader/writer i role-bound efekt wykonywany razem z innymi skutkami; raw source storage nie staje się native progress, World Transition nie jest Quest transition. | Najpierw przyjęte QUEST-PRED-1/QUEST-LOWER-1/QUEST-XP-1 oraz obecny owner Questów. NPC-QUEST-1 zachowuje własne zależności. Nasz lokalny tracks amendment trzeba dopasować, nie budować drugiego Quest store. |
| Usługi / wallet / inventory | Owning atomic transaction i rewalidacja warunków, single application/retry, odmowa bez utraty zasobów; handel/podróże/blessing/promocja/bank oraz acquisition tylko zgodny z modern policy. | Wykorzystać aktualne BANK/GOLD-FEE oraz pozostałych domain ownerów. Zachować warunki acceptance/held z STATE D498; ten wpis nie daje lease migracji, protokołu ani zgody na start held tasków. |
| Item / Spell / appearance | Exact wiązania natywnych tożsamości i epok, pełne payloady oraz klientowe appearance; name match i known-field projection to osobne dowody. | Wykorzystać gotowe pakiety Item/Spell. Właściciel Spell właśnie przekazał lokalne r59–r62: [475 kandydatów + 8 referencji](https://github.com/Oteryn/Oteryn-Game/issues/1622#issuecomment-5978988593). Odebrać je przed ponownym zbieraniem tych samych danych; prywatny Spell DATA model nie jest automatycznie native binding. |
| Integracja / kwalifikacja | Zastosowanie spójnej dopasowanej partii i sprawdzenie na istniejącej mapie: greeting, reply, success/refusal, wallet/inventory/progress, reconnect/restart. | CP ustala istniejącego writera/owned paths i sposób odbioru lokalnego packetu. Test importu nie jest gameplay E2E; bez fake servera, receiptów lub platformowej topology. |

**Oczekiwany wynik triage CP:** przy każdej zależności zapisać istniejący task/owner i konkretny consumer seam; odróżnić „mechanika już jest, brakuje adaptera” od „nieprzyjęty kontrakt”, „brak runtime” oraz „celowo zachowany source conflict”. Nasze wcześniejsze ogólne listy braków nie powinny automatycznie tworzyć nowych tasków przeciw obecnemu main.

### Rozliczone dane i nadal jawne holdy

- 4379 wystąpień + wcześniejsze 31 supplements: linker daje **4346 kwalifikowanych powiązań źródłowych i 33 context/epoch holds**, nie native progress bindings. Holds: **32** wystąpienia dwóch odrębnych kontekstów Captain Haba Open Sea oraz **1** Summer Shirtalis old/current storage epoch. Nie zrównywać ich z bazowym aktorem/trackiem po nazwie.
- **18 callbacks / 55 węzłów** to source-only kandydaci; **zero automatycznych nowych przypięć Dialogue** przy konfliktach tekstu/gałęzi/stanu.
- **4067** wierszy usług źródłowych rozliczonych. 342 name/epoch candidates odnoszą się do siedmiu istniejących instant Spell payloads w naszym cache; nie oznacza to admitted bindingów. Pozostałe 3354 wpisy nie miały pełnego payloadu w tym cache — teraz najpierw uzgodnić z gotowym packetem workera Spell, nie uznawać ich za brak aktualnego serwera.
- 416 obserwacji authored prices → 223 grupy; 58 donor conflicts → 16 grup. Istniejące ceny zachowane. 361 historycznych count/subtype observations → **188 grup już dokładnie zaimportowanych**: to stare holdy, nie brakujące oferty.
- **356** obserwacji Item → 105 ID/pin rows, 102 IDs; known projection: 269 match / 48 differ / 39 unproven. Cztery description-only Item candidates obejmują 110 obserwacji, ale ich local admission nie promuje bieżących donor bindings ani appearance.
- Zachowane 8 source blessing conflicts u 4 NPC. Static Lua→C++ proof obu pins: 6=Embrace, 1=Twist; bez automatycznego 6→1 i bez deklaracji live game bug.
- Source `postman` discount używa pack-qualified `Storage.Quest.ExampleQuest`, nie native PostmanQuest; global loader pozostaje nieudowodniony. Brak `new frontier` w konkretnym source discount table daje source amount 0, bez wymyślonego gate.

### Źródła i odbiór artefaktów

Praca cache-first: Canary `04b83b512114bfd888000d6e1433ed8ecaec7c5b`, Crystal summer `00ce02a57ca5a12e48f32a3476e37471167e4c3f`, istniejące snapshoty Quest/Item oraz rzeczywisty kod i schemy serwera. Ta partia nie wymagała ponownego pobierania źródeł, nowego wiki researchu ani Remote Desktop.

Lokalny katalog: `/workspace/npc-upstream-first-audit/npc-completion-project/`.
- `PLAN.md`, `ARCHITECTURE.md`, `tasks.json`, `STATUS.md`;
- `integration/second-batch/combined-proposed.patch`, `manifest.json`, `verification.json`;
- `integration/travel-data/{materialize.rs,reproduce.py,patch.json,verification.json}`;
- `items/second-batch/{receipt.json,validate.rs,reproduce.py,RESULT.md}`;
- per-lane patches/results w `quest-linking/`, `dialogue/`, `runtime/`, `special-services/`, `commerce/`, `items/`;
- `review/{final-repair-review.json,travel-data-review.json}`.

Te ścieżki są lokalne i nie muszą być dostępne innym workerom. Patche/manifesty/skompresowane kandydaty są zapisane w projekcie; duże odtwarzalne światy i logi w /tmp są nietrwałe. CP powinien wskazać sposób odbioru packetu przed uzależnieniem integracji od plików; ten handoff nie jest zgodą na PR/push.

<!-- OTERYN-NPC-LOCAL-HANDOFF-20261004-32afc940 -->

