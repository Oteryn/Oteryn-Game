# Content / World — aliasy i uruchamianie

Status: profile discovery i instrukcja operatora; nie alokacja wykonawcza.

## 1. Punkt wejścia

Najpierw uruchomić `Oteryn: content world lead`. Lead czyta live state i przygotowuje właściwą następną pracę. Jest koordynatorem technicznym domeny pod #162, nie drugim control plane. Bez dokładnego przydziału pozostaje read-only; nie przyznaje sam sobie lub innym praw zapisu.

Przed integracją pakietu do main skrót może nie być rozpoznany przez zwykłe discovery. Użyć jawnego bootstrapu:

```text
Repo: Oteryn/Oteryn-Game, PR #641.
Odczytaj aktualny head PR i z tej dokładnej rewizji wczytaj
 docs/agents/programs/OTV2_CONTENT_WORLD_AGENT_LAUNCH_RUNBOOK.md
oraz docs/agents/prompts/OTV2_CONTENT_WORLD_LEAD.md.
Uruchom profil Oteryn: content world lead.
Protected main i live #162 pozostają authority; head PR jest kandydatem.
Nie uruchamiaj zapisów implementacyjnych bez właściwej live allocation.
```

Po protected integracji rozwiązywać aliasy z aktualnego `docs/agents/PROMPT_LIFECYCLE.json`. Sam alias nigdy nie wybiera modelu, effort, konta API, uprawnień ani hosta.

## 2. Podział

| Rola | Dokładny alias | Prompt | Główny wynik |
|---|---|---|---|
| CW0 | `Oteryn: content world lead` | [LEAD](../prompts/OTV2_CONTENT_WORLD_LEAD.md) | Reuse obecnych prac, zależności i jeden następny przyrost. |
| CW1 | `Oteryn: content world architecture` | [ARCHITECTURE](../prompts/OTV2_CONTENT_WORLD_ARCHITECTURE.md) | Source/binding/format decision tam, gdzie rzeczywiście brakuje decyzji. |
| CW2 | `Oteryn: content world import` | [IMPORT](../prompts/OTV2_CONTENT_WORLD_IMPORT.md) | Rzeczywiste partie danych i mappery bez cichej utraty. |
| CW3 | `Oteryn: content world build` | [BUILD](../prompts/OTV2_CONTENT_WORLD_BUILD.md) | Wspólny model, kompilator, pakiety i loader. |
| CW4 | `Oteryn: content world runtime` | [RUNTIME](../prompts/OTV2_CONTENT_WORLD_RUNTIME.md) | Realne operacje obiektów i kompozycja domen. |
| CW5 | `Oteryn: content world client` | [CLIENT](../prompts/OTV2_CONTENT_WORLD_CLIENT.md) | Projekcja gry i późniejsze Studio na tym samym modelu. |
| CW6 | `Oteryn: content world qa` | [QA](../prompts/OTV2_CONTENT_WORLD_QA.md) | Rzeczywiste E2E, regresje i prawdziwe granice dowodu. |
| AUDIT | `Oteryn: content world audit` | [AUDIT](../prompts/OTV2_CONTENT_WORLD_INDEPENDENT_AUDIT.md) | Niezależny read-only audyt kompletności architektury, danych i realnej implementacji całego Content/World. |

## 3. Start i fale

Wszystkie profile najpierw odnajdują właściwy live task/branch/owner; bez wymaganego przydziału wolno tylko read-only preflight. Użytkownik nie musi dostarczać numeru istniejącego issue, jeśli można je odnaleźć przez GitHub.

1. CW0: odczytać #162/#641 i [programme](OTV2_CONTENT_WORLD_DELIVERY_PROGRAMME.md), sprawdzić existing workers i dokładne blockers. Zwrócić next-allocation proposal albo kontynuować już przydzielone dokumentacyjne prowadzenie.
2. CW1 + CW2: równolegle zamykać potrzebną decyzję i zbierać źródła. Mutacje docs/importera tylko po odpowiednich przydziałach. Nie wymagać pełnego Server Seam dla źródeł lub kontraktu.
3. CW3 + CW2: po właściwym kontrakcie budować wspólny model i import. CW3 jest writerem modelu; CW2 pisze adaptery. Nie pracować równolegle na tym samym model/schema/Cargo.
4. CW4 + CW5: po gotowych interfejsach i potrzebnych accepted owner/wire boundaries łączyć runtime z klientem. Wcześniejsze component tests są pomocnicze, nie zastępczą grą.
5. CW6 wraz z pojawieniem się komponentów, następnie na realnym journey z obecnymi ownerami gameplay/DUR. CW2 może nadal rozwijać katalog; CW5 dodaje Studio, gdy jego interfejsy i przydział są gotowe.

### CW2 — szeroki katalog Content

Szeroki import katalogu jest częścią CW2/CW3 i nie czeka na CW6. Używać istniejącego aliasu:

```text
Oteryn: content world import
```

Nie tworzyć osobnego konkurencyjnego aliasu `content catalog`. Batch plan i kolejność items -> creatures/spawns -> loot bindings -> dalsze rodziny opisuje [bulk catalogue import plan](OTV2_CONTENT_WORLD_BULK_CATALOG_IMPORT_PLAN.md).

Bez live #162 allocation alias wykonuje tylko preflight/propozycję exact batch + custody. Po przydziale CW2 może działać równolegle z CW4 wyłącznie na rozłącznych ścieżkach; mutacje wspólnego modelu pozostają serializowane przez CW3.

Po zakończonym bootstrapie pojedynczego itemu control plane nie powinien przydzielać kolejnych zwykłych slice'ów typu „jeden item → native promotion”. Normalny następny przyrost CW2/CW3 ma być batchowy: ograniczona reprezentatywna partia lub cała gotowa partycja rodziny. Single-record pozostaje tylko fixture/regression/diagnostyką albo udokumentowanym wyjątkiem wymuszonym konkretnym blockerem; nie jest postępem katalogowym. Jeżeli batch jest technicznie i semantycznie możliwy, „mniejszy scope” nie jest wystarczającym powodem do powrotu do pracy rekord-po-rekordzie.

Nie uruchamiać siedmiu agentów jednocześnie. Zalecane maksimum organizacyjne: trzy aktywne role łącznie, zwykle jeden lub dwóch writerów oraz niezależny read-only/QA. Gdy lead potrzebny jest do nadzoru, zajmuje jeden z tych slotów. Dwie implementacje są równoległe tylko przy faktycznie rozdzielonych ścieżkach i stabilnych interfejsach. Ta sama sesja może kolejno wykonać różne przydzielone przyrosty bez tworzenia nowej gałęzi tylko dla zmiany roli.

## 4. Istniejące aliasy pozostają

`Oteryn: work coordinator` (albo inny jednoznacznie aktywny profil #162) zachowuje alokacje i integrację. Nie przełączać control plane przez uruchomienie nowego aliasu. `Oteryn: sol supervising architect` pozostaje drogą materialnej eskalacji.

Istniejących `impl content`, Movement, Combat, Ability, domains, Durability, Server Seam, Native UI i Client/QA nie zastępujemy. Gdy mają canonical workera odpowiadającego danemu przyrostowi, użyć odpowiedniego profilu CW jako delty dla niego; nie tworzyć konkurenta. Zakończony `content-format-spike` pozostaje evidence; nowa praca formatu wymaga nowego, właściwego przydziału, nie reaktywacji retired aliasu.

`Oteryn: ref <lane>` można wykorzystać do odczytowego badania źródeł zgodnie z jego lifecycle. CW6 nie zastępuje formalnego independent review; wymagany review idzie istniejącą ścieżką, bez automatycznego zamawiania płatnych agentów.

## 5. Obowiązkowa informacja o następcy

Na końcu odpowiedzi każdego profilu CW stosować znany format:

```text
CONTROL_PLANE_ACTION: <dokładny aktywny alias + konkretna czynność, albo NONE>
NEXT_WORKER: <dokładny alias rzeczywistego wykonawcy, maksymalnie trzy, albo NONE>
RUN_WORKER_WHEN: <sprawdzony warunek startu; osobno brakujący przydział/dependency>
WHY: <jedno zdanie o zależności>
```

Nie wpisywać control-plane-only roli w NEXT_WORKER. Gdy trzeba najpierw wykonać integrację/alokację, podać ją oddzielnie, a NEXT_WORKER nadal nazwać właściwego przyszłego wykonawcę. Jeżeli nie wiadomo którego, użyć NONE zamiast zgadywać. Lead domenowy jako następny krok koordynacyjny należy do CONTROL_PLANE_ACTION, z zastrzeżeniem że sam nie ma authority control plane.

Przykład: kontrakt jest gotowy, ale nie został zaakceptowany/ochroniony i nie ma przydziału modelu:

```text
CONTROL_PLANE_ACTION: Oteryn: work coordinator — rozstrzygnąć właściwą akceptację/integrację i przydział wspólnego modelu
NEXT_WORKER: Oteryn: content world build
RUN_WORKER_WHEN: potrzebny kontrakt jest zaakceptowany i live allocation wskazuje canonical workera oraz dokładne ścieżki
WHY: build realizuje przyjęty kontrakt, zamiast samemu akceptować publiczny format
```

Przykładowy alias control plane musi zostać zastąpiony aktualnym, gdy live #162 wskazuje inny profil.

## 6. Model i effort

To wskazówki operatora, nie wymuszone ustawienia: lead zwykle medium, import i rutynowe prace medium, architektura i trudna kompozycja high tylko gdy problem tego wymaga. Ten sam profil może działać na kompetentnym dostępnym modelu. Nie zakładać, że słowo w aliasie ustawia model/effort, ani że brak opcji xhigh zatrzymuje pracę. Faktycznie wybrane ustawienia odczytać na powierzchni wykonawczej, gdy są dostępne. Nie uruchamiać płatnych API/reviews tylko dlatego, że prompt je wspomina.

Dla pełnego niezależnego `Oteryn: content world audit` zalecana powierzchnia operatora to **GPT-5.6 Pro** (albo aktualny najwyższy zgodny model klasy Pro) z wysoką dostępną głębokością rozumowania, ponieważ zadanie wymaga jednoczesnego porównania architektury, danych, implementacji i evidence. To wyłącznie rekomendacja wykonawcza: alias nie wybiera modelu, a model/effort nie nadają żadnej authority. Audytor pozostaje read-only i nie zastępuje CW0–CW6 ani aktywnego control plane.

## 7. Minimalny wynik przyrostu

Zwrócić: produktowy wynik; repo/branch/PR/head; zmienione ścieżki; faktycznie wykonane testy i ograniczenia; jeden dokładny blocker lub gotowość przekazania; successor footer. Odpowiedź ma odróżniać code/source/model/CI/native-client/PG/Reference evidence. Nie podawać deklaracji background continuation bez rzeczywistego mechanizmu.
