# DRAFT: usprawnienie przepływu PR i kosztu CI

Status: **propozycja do niezależnego przeglądu**, nie przyjęta polityka ani implementacja.
Data pomiarów: 14 października 2026, około 09:54–10:01 Europe/Warsaw.
Właściciel draftu: bieżąca sesja Codex, na bezpośrednie zlecenie właściciela projektu.
Zakres: dokumentacja propozycji i dane pomiarowe. Bez zmiany workflowów, ochrony,
Merge Queue, deploymentu, autoryzacji lub wywołania dodatkowego review.

## 1. Cel i stan wiedzy

Skrócić czas od rzeczywistej gotowości zmiany do bezpiecznego merge oraz ograniczyć
ponawianie tej samej kwalifikacji. Nie premiować liczby PR ani samej szybkości pisania kodu.

PROVEN: audyt objął 63 scalone PR Game i 47 Platform, znalezione przez
`is:pr is:merged merged:>=2026-10-12` do momentu odczytu. Okno zaczyna się
12 października 00:00 UTC (02:00 czasu polskiego). Dane tej kohorty obejmują również
PR otwarte i wcześniej dodawane do kolejki przed początkiem okna.

DERIVED: wielogodzinne przerwy często przypadają przed enqueue. Nie jest dowiedzione,
że każda taka przerwa była zbędna: mogła wynikać z review, zależności lub decyzji operacyjnej.

UNKNOWN: historyczny timestamp autorytatywnego READY oraz komplet powodów wszystkich
blokad. Zielony check, CLEAN i data ostatniego commita nie dowodzą samodzielnie gotowości.

## 2. Cztery proponowane pakiety

### A. Terminowe przekazanie kwalifikowanego kandydata do integratora

Rozbudować istniejący koordynator zamiast tworzyć drugi system sterowania merge.
Zakończenie wymaganych checks lub review powinno wznowić ocenę aktualnego kandydata.
Integrator sprawdza exact head, zakres autoryzacji, komplet dowodów, rozwiązane uwagi,
zależności i wcześniejsze nieudane MQ. Dopiero po spełnieniu wszystkich wymagań
korzysta z obowiązującej natywnej ścieżki enqueue. W przeciwnym razie zapisuje
konkretny `BLOCKED(reason)`, właściciela i zdarzenie usuwające blokadę.

Proponowany cel operacyjny: P90 **potwierdzone READY → przyjęte enqueue ≤5 minut**.
To hipoteza/SLO do zatwierdzenia, nie obecny wynik ani gwarancja całego czasu merge.
Nie zastępuje obowiązującej polityki integracji, nie pozwala omijać kontroli ani
ponawiać nieudanego MQ bez rozpoznanej przyczyny. Nie należy automatycznie uznawać
każdego zielonego PR za uprawniony do enqueue.

### B. Jedna kwalifikacja stabilnego kandydata

Podczas authoring wykonywać szybkie kontrole właściwe dla zmiany. Zebrać znane
poprawki i dokumentację przed zamrożeniem kandydata do wymaganej kwalifikacji/review.
Nie robić statusowych commitów po każdym etapie. Świeżo kontrolować metadane PR,
ale nie powtarzać zgodnej kwalifikacji kodu wyłącznie z powodu edycji opisu.

Reuse musi wiązać head, właściwą bazę/kompozycję, wersję workflowów, środowisko i
pełny wymagany zakres. Sam head SHA nie wystarcza. Materialna zmiana unieważnia
odpowiednie dowody; kosmetyczna zmiana nie jest automatycznym powodem dodatkowego
płatnego review. Istniejąca polityka review pozostaje nadrzędna wobec tej propozycji.

### C. Ograniczenie niepotrzebnie wybranego CI

Wprowadzać osobno dla każdego repozytorium; szczegóły poniżej. Tanie kontrole
spójności i polityk powinny poprzedzać wielominutowe testy, jeśli zależności na to
pozwalają. Zachować kompletną kwalifikację złożenia i stale raportowany wymagany agregat.
Nie używać liczby linii ani globalnego `paths-ignore: '**/*.md'` jako dowodu małego ryzyka.

### D. Rozsądne grupowanie i domykanie pracy

Łączyć powiązane drobne zmiany jednego modułu razem z dokumentacją. Domykać jego
kwalifikowane lub naprawialne PR przed rozpoczęciem kolejnych zależnych zmian.
Niezależne moduły mogą powstawać równolegle. Nie łączyć wszystkich funkcji w duże PR.
Nie zwiększać bez pomiaru limitu MQ i nie narzucać arbitralnego globalnego limitu dwóch PR.

## 3. Dane tego repozytorium i lokalny zakres pakietu C

Snapshot kodu: `1d8714599c2a9baddc48293f61f85dd9ddd86f75`.

| Miara | Wynik |
|---|---:|
| PR scalone w kohorcie | 63 |
| Mediana finalny commit/otwarcie → merge | 42,0 min |
| P90 tego odcinka | 548,8 min |
| Mediana ostatnie enqueue → merge | 18,4 min (n=62) |
| P90 ostatnie enqueue → merge | 19,9 min |
| PR z wielokrotnym enqueue w ich cyklu życia | 7 |

Przykłady do weryfikacji: [#2023](https://github.com/Oteryn/Oteryn-Game/pull/2023)
czekał po zielonym game-gate około 13h02m przed enqueue, a potem 6,1 min do merge;
[#2024](https://github.com/Oteryn/Oteryn-Game/pull/2024) odpowiednio 12h10m i 4,7 min.
To obserwacja czasów, nie automatyczny dowód braku uzasadnionej blokady.

W oknie utworzenia runów MQ od 12 października odczytano 67 runów: 62 success i
5 failure. Nie wyłączać testów złożenia: błędy obejmowały biblioteki, integrację,
PostgreSQL i lint. Ta populacja runów jest inna niż cykle życia scalonych PR.

Już wdrożone: odroczenie Rust do MQ dla kwalifikujących się zmian samych danych
Content, pomocnicze deploy/ przy braku konsumentów produktu, normalizacja mtimes
cache na PR i cache w canonical World package. Nie proponować ich ponownie jako nowych.

Do sprawdzenia w pakiecie C:
- rzeczywiście szybki zakres PR kodu dużego serwera, pełna biblioteka w MQ;
- dokumentacja narzędzia Quest bez pełnego run_checks.py;
- zawężenie historycznego materializera World według udowodnionych wejść, z zachowaniem pin-check;
- wcześniejsza tania walidacja spójności dowodów Quest/governance;
- deduplikacja kwalifikacji metadanych; dopiero potem profilowanie PG i cache.

Świeże przykłady: [Rust PR 16,4 min](https://github.com/Oteryn/Oteryn-Game/actions/runs/38528002471),
[World 17,2 min](https://github.com/Oteryn/Oteryn-Game/actions/runs/38528002650),
[Quest failed po 17,8 min](https://github.com/Oteryn/Oteryn-Game/actions/runs/38528002646),
[pełna MQ 18m34s](https://github.com/Oteryn/Oteryn-Game/actions/runs/38519194501).
Różne zakresy i SHA: nie jest to porównanie A/B efektu cache.

Ochrona odczytana: ruleset 20991995, wymagany game-gate, MQ ALLGREEN, max 5 wpisów budowania.
Nie zmieniać tych ustawień na podstawie samego draftu.

## 4. Metodologia i ograniczenia pomiarów

Załącznik [metryki PR](CI_FLOW_AUDIT_2026-10-14.csv) zawiera tylko kohortę tego repo.
Każdy wiersz ma URL PR, finalny head, merge SHA i znaczniki czasu do niezależnej weryfikacji.
- `open_to_merge`: od otwarcia do merge; nie jest czasem aktywnego pisania kodu.
- `final_commit_or_open_to_merge`: od późniejszej z dat otwarcia i committedDate
  ostatniego commita finalnego head; nie jest autorytatywnym FREEZE ani READY.
- `first_enqueue_to_merge`: również usunięcia, naprawy i ponowienia.
- `last_enqueue_to_merge`: końcowa próba z kwalifikacją MQ.
- `before_first_enqueue`: tylko przypadki, gdy finalny commit/otwarcie nie wypada
  po pierwszym enqueue; brak wartości nie oznacza zera.
- Jednostką czasów numerycznych jest minuta. P90: nearest-rank. Nie sumować median
  niezależnych odcinków. `RemovedFromMergeQueueEvent` występuje też przy poprawnym merge.
- Pobierano filtrowane zdarzenia ready/draft/queue/merge; sprawdzono brak poprzedniej
  strony przez pageInfo. `timeline_total` jest metadanymi API i nie dowodzi liczby
  filtrowanych zdarzeń ani obcięcia historii.
- Ta kohorta nie obejmuje porzuconych/otwartych PR w medianach dostarczenia.
  Aktualne repozytoria i kolejki mogą już różnić się od snapshotu.

Reprodukcja: pobrać wskazany PR, jego finalny commit i filtrowane timelineItems;
porównać znaczniki z CSV. Dla przerw po zielonym agregacie odczytać
`commits/<head>/check-runs?filter=all` i odszukać właściwy sukces przed enqueue.
Przed przypisaniem przerwy do bezczynności sprawdzić review tego samego SHA,
zależności, autoryzację i ewentualne okno operacyjne. Nie wykonywać rerunów,
review, enqueue ani zmian ustawień tylko w celu odtworzenia pomiaru.

## 5. Plan oceny i wdrożenia po przeglądzie

1. Potwierdzić dane i oddzielić gotowość od zielonego checku.
2. Wybrać najmniejszą zmianę istniejącego integratora/state; zdefiniować źródło READY.
3. Wdrażać A/B, potem repozytoryjny C, w oddzielnych ograniczonych paczkach.
4. Przez tydzień mierzyć READY→enqueue, ostatnie enqueue→merge (lekki/runtime osobno),
   kwalifikacje i próby MQ na merge, job-minuty oraz czas w każdym BLOCKED(reason).
5. Potwierdzić niezmienione pokrycie obowiązkowych testów i ochronę; przy brakujących
   dowodach wrócić do pełnej kwalifikacji. Nie obiecywać procentu oszczędności przed pomiarem.

## 6. Pytania dla niezależnego agenta

- [ ] Czy znaczniki i statystyki z CSV zgadzają się z GitHub, a okno i braki są opisane poprawnie?
- [ ] Które wielogodzinne przerwy mają uzasadnioną przyczynę poza CI? Nie utożsamiać ich ze stratą.
- [ ] Jaki istniejący komponent posiada wyłączne uprawnienie do enqueue i poprawny mechanizm wznowienia?
- [ ] Czy proponowane READY i klucz reuse obejmują wszystkie wymagane dowody, review i zmiany ryzyka?
- [ ] Czy zmiany od snapshotu już realizują lub unieważniają którąś rekomendację?
- [ ] Czy routing zachowuje przypadki mixed diff, rename/delete, unknown, workflow, build i zależności?
- [ ] Czy dla każdej proponowanej redukcji jest równoważne wymagane pokrycie w odpowiedniej fazie?
- [ ] Czy SLO 5 minut dotyczy wyłącznie enqueue po rzeczywistej gotowości i jest operacyjnie osiągalne?
- [ ] Które punkty przyjąć, odrzucić lub odłożyć? Podać dowód i minimalny zakres przyszłej implementacji.

Reviewer ma ocenić propozycję; ten dokument nie upoważnia do wdrożenia, merge,
zmiany ochrony, wydatków na nowe review ani operacji produkcyjnych.
