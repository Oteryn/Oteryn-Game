# Stan projektu — 2026-10-04

Root koordynuje sześć torów, osiem zadań i niezależny przegląd. Pierwsze lokalne rezultaty wszystkich torów są gotowe. Status LOCAL_DELIVERABLE_READY oznacza dostarczony wynik danej partii, nie ukończoną funkcję runtime. Właściciele, zależności, dowody i dalsze prace są w tasks.json.

## Wyniki pierwszej partii

| Tor | Wykonano | Pozostaje |
|---|---|---|
| Questy i konteksty | Linker, 352 pary deklaracji/profili jako nieprzyjęci kandydaci, 4346 powiązań źródłowych; rozliczone 33 zatrzymane konteksty/epoki | Nazwane ścieżki postępu, trwały właściciel zapisu, faktyczne wiązania |
| Dialogi | Proponowana schema i planner uporządkowanych kroków; 18 callbacków/55 węzłów kandydatów | Prawdziwy dispatcher i właściciele efektów; zero nowych przypięć Dialogue |
| Runtime | Prześledzona istniejąca mapa i rzeczywiste granice typów, kandydat placement MUD | Aktor NPC, lowering, CHAT, widoczność, transport; zero uruchomionych aktorów |
| Usługi specjalne | Rozliczone 4067 wierszy; proponowane przygotowanie nauki spell i kosztu | Wiązanie Spell, transakcyjny zapis nauki i wykonawcy innych usług |
| Handel i podróże | Rozliczone ceny i stare count/subtype; 4 źródłowe obserwacje dają 2 aktualizacje tekstu podróży | Przyjęcie schemy/danych, wykonawcy i wiązania rabatów; brak bezpiecznych zmian cen |
| Przedmioty | 356 obserwacji/105 wierszy ID+pin rozliczonych; 4 kandydatów opisów i rzeczywiste DTO importu | Pełne przyjęcie, bieżące bindingi źródłowe, appearance, materializacja |

## Scalenie i weryfikacja

Trzy propozycje kodu połączone w integration/combined-proposed.patch: schema dialogów, departure_text podróży oraz przygotowanie nauki spell. Patch zawiera dwa dodatkowe testy rzeczywistego importu napisane przez root. git apply --check względem zamrożonego produktu PASS. W izolowanej kopii rzeczywistego game-server: cargo check --lib PASS; 10/10 native admission tests PASS. Dowody i hashe: integration/manifest.json i verification.json. Niezależny przegląd usunął dwa znalezione problemy dialogów; dodatkowo zaostrzono test odmowy efektu bez właściciela.

To są propozycje poza produktem: nie zmieniono repo Oteryn-Game, nie zastosowano nowego importu ani nie aktywowano runtime. Bazowy kwalifikowany WorldProject pozostaje e749728fc12b0b79a8a296fdb8b8c8393b5dd2c8b4e7ef0115c8836084347ebc: 1282 NPC, 836 Dialogue, 380 Service, 2564 profili. Liczby nie określają kompletnych NPC w grze.

## Następna kolejność architekta

1. Kontrakt aktora i dispatchera NPC w istniejącym runtime; równolegle uzgodnić z workerem Questy nazwane ścieżki i właściciela trwałych zapisów.
2. Przypinać reprezentowalne programy rozmów dopiero po rozstrzygnięciu tożsamości, konfliktów tekstu i właścicieli efektów.
3. Podłączyć transakcyjne usługi i zakwalifikować Item/Spell; zachować istniejące ceny oraz rozdzielenie automatic/free od płatnej nauki.
4. Przyjąć konkretną spójną partię danych w izolacji, odtworzyć ją i sprawdzić na istniejącej mapie. Dopiero po parytecie Canary/Crystal uzupełniać wiki.

Źródła tej partii: istniejący lokalny cache Canary/Crystal oraz rzeczywisty kod i schema Oteryn. Bez ponownego pobierania źródeł, bez Remote Desktop, PR, pushu i wdrożenia. Duże odtwarzalne dowody w /tmp są nietrwałe; trwałe patche, manifesty, skompresowane kandydaty i wyniki znajdują się w tym projekcie.

## Druga partia — ukończona lokalnie

- Dwa istniejące rekordy podróży Hawkhurst/Hawkhurst Ingol mają `departure_text` w kopii importowanej przez rzeczywisty serwer z proponowaną schemą. Drzewo b506d02a8ca6512ae59fc3abce7992352cb49de50a09286a08ff28debf6782f7. Dwie materializacje mają identyczne bajty; ponowny pinned admission PASS. Odmowy driftu SHA, zmiany ceny i zdublowanego celu przechodzą bez zapisu wyniku. Pozostałe deklaracje niezmienione. Zmieniły się wyłącznie declarations oraz trzy kanoniczne dokumenty rewizji/hashów. Bazowy import produktu e749… pozostaje niezmieniony. Dowód: integration/travel-data/verification.json; pełne świeże odtworzenie: reproduce.py --scratch /tmp/npc-project-travel-NOWA-NAZWA.
- Cztery PENDING/LocalNonProduction kandydaty Item przechodzą pełne rzeczywiste `from_v2_draft` oraz ponowny snapshot parse. Użycie limitu 4/24 kandydatów, pięć odmów negatywnych. Wynik jest walidacją in-memory batcha, nie edycją natywnych statystyk przedmiotów ani aktywacją. Dowody items/second-batch.
- Propozycja nazwanych etapów Quest dodaje tracks do istniejącego authoringu i sprawdza exact membership. Brak zaobserwowanego postępu pozostaje Unknown nawet przy initial_value=0; gate runtime nadal odmawia bez trwałego właściciela.
- Resolver katalogu NPC sprawdza tree/revision oraz własność NPC→Dialogue i faktycznie wywołuje natywny planner. Stan rozmowy dostarczy przyszły actor owner; żadnego zastępczego session store.
- Kwalifikacja placementu sprawdza istniejące dokładne NPC, profile i usługi; bogaty natywny fixture odmawia placementu przypisanego innemu NPC. Wszystko nadal CandidateOnly.

Finalny wspólny patch i manifest: integration/second-batch. Offline cargo check --lib PASS; 11/11 rzeczywistych testów admission PASS na finalnej kopii. Testy torów: 8 Quest, 6 Dialogue, 3 placement; niezależny final-repair review PASS. W pierwszym testowym fixture odkryto niedozwolony marker fixture w ProductionKey; naprawiono nazwy zgodnie z istniejącą konwencją courier, bez osłabienia walidatora.

Nie są to działające aktory/usługi NPC. Najbliższa rzeczywista implementacja: przyjęte native family/lowering/activation oraz actor/CHAT/visibility owner; równolegle trwały Quest reader/writer i transakcje usług. Kolejne decyzje są zapisane w tasks.json. Produkt nadal czysty; brak PR/pushu/wdrożenia, pobierania źródeł i Remote Desktop.
