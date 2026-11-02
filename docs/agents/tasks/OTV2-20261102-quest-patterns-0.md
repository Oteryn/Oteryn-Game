# OTV2-20261102-quest-patterns-0

```yaml
task_id: OTV2-20261102-quest-patterns-0
title: Prompt 0 — katalog wzorców questów i porównanie Global
mode: AUDIT
status: waiting
repository: Oteryn/Oteryn-Game
base_branch: main
branch: chatgpt/quest-patterns-0-20261102
pr: 2836
base_sha: 9f897fcf14da4f5e8d7ffa4b2cda164b16658fee
owner: chatgpt/quest-patterns-0-20261102
created_at: 2026-11-02T18:59:40Z
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/OTV2-20261102-quest-patterns-0.md
  - docs/agents/evidence/OTV2-20261102-quest-patterns-0.md
  - docs/agents/questions/chatgpt-quest-patterns-0-20261102.md
public_contracts: []
depends_on: []
blocks: []
```

## Outcome

Przygotowano [jeden raport Prompt 0](../evidence/OTV2-20261102-quest-patterns-0.md) z dokładnie siedmioma wymaganymi sekcjami, słownikiem P01–P30, 90 przykładami donorowymi i pełną tabelą 373 tytułów katalogu Wiki. Raport pozostawia nieznane dane oraz sprzeczne reguły jawnie nierozstrzygnięte.

## Architecture and source of truth

- Crystal: zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8.
- Canary: opentibiabr/canary@47dfd51f45280a59a1d3e50ba7edd573d7234446; porównanie tylko przy brakach lub niejasnościach Crystal.
- Oteryn do analizy: Oteryn/Oteryn-Game@28eae9c341a7815abe0b403e9ac43285d1d85cbb. Nowsza baza publikacji nie zmienia źródła analizy.
- TibiaWiki: wyłącznie api.php; treść i revid w tej samej odpowiedzi, pełna paginacja kategorii. Odczyt 2026-11-02 przez zwykły Chrome na autoryzowanym Molehill-PC, bez zmiany User-Agent lub obchodzenia zabezpieczeń.
- PROVEN: DRAFT PR otwarto po pierwszym commicie 5287a44c6ab88ec2d984b91ed03c5610c70bc923. PR jest wiążącym kanałem QUESTION:/ANSWER:.
- CONFLICT: załącznik wymaga „Bez kodu”, wiadomość zlecająca wspomina pełne napisanie kodu. Q1 pozostaje otwarte; ukończenie raportu nie jest deklaracją ukończenia nieokreślonej implementacji.
- Publikacja: wyłącznie własna gałąź i dwa wskazane pliki, root jest jedynym writerem. Wybrana wcześniej ścieżka GitHub Git Data tworzy nowego kandydata z pełnego delta, jednym parentem oraz non-force update z expected_sha i natychmiastowym readback. Nie zastępuje tożsamości wybranego lokalnego commita; takiego commita nie tworzono. PC służy do API Wiki i izolowanej walidacji, nie do obchodzenia ograniczeń publikacji.

## High-risk authority/recovery qualification

NOT_APPLICABLE: dokumentacja i analiza źródeł; bez zmiany runtime, protokołu, kont, danych produkcyjnych lub uprawnień.

## Acceptance criteria

- [x] Pokryto wszystkie 123 katalogi Crystal, 1093 quest Lua i 11 bibliotek.
- [x] Spis NPC obejmuje wszystkie 1156 plików oraz jawne, aliasowane i dynamiczne zapisy storage/KV.
- [x] Słownik P01–P30 ma po trzy przykłady, szacunek z metodą i źródła możliwości Oteryn.
- [x] Pełny katalog API ma 373 wiersze bez braków lub duplikatów; rozróżniono alias, Deprecated, grupy i questy grywalne.
- [x] Definicje dziennika, oryginalne misje, QuestState i wybrane etapy Oteryn policzono osobno.
- [x] Obie strony rozbieżności mają źródła; brak dowodu nie został zamieniony w regułę gry.
- [x] Otwarte Q1/Q3/Q4/Q5 zapisano w komentarzach i w Left out opisu PR; Q2 rozwiązano technicznie.
- [x] Zakończono przegląd źródeł i poprawiono zgłoszone błędy odnośników oraz nadmierne twierdzenia o wsparciu Oteryn.

## Reference parity

NOT_APPLICABLE dla implementacji mechaniki. Wynik jest katalogiem źródłowym, z repo@commit:path:line albo tytułem Wiki i revid. Wszystkie 7274 pliki poszerzonego korpusu donorów zweryfikowano przez Git blob SHA; źródła użyte w raporcie dodatkowo sprawdzono pod kątem istnienia linii.

## Excluded scope

Implementacja Rust/Lua/JSON i aktywacja runtime nie mają sprecyzowanego zakresu poza konfliktem Q1. Nie wykonano testu gameplay, pełnego audytu map OTBM/XML, klienta ani połączenia z grą CipSoft. Brak merge lub deploy.

## Implementation / findings

- Trzech równoległych subagentów miało rozłączne tytuły: A–H 132, I–R 103, S–Z 136. Root objął dwa tytuły numeryczne, scalanie i niezależne przeliczenie danych.
- 373 tytuły: 301 z potwierdzonym komponentem Crystal, 22 bez rozpoznanego przepływu w obu badanych zakresach Lua, 50 NIEZNANE. Nie jest to liczba kompletnych, działających questów. Po odjęciu czterech Deprecated i jednego aliasu pozostaje 368 wpisów, nadal z grupami/podzadaniami.
- Oteryn: 352 definicje; flaga dziennika 61 true / 223 false / 68 brak pola; 529 oryginalnych misji w 58 rekordach; QuestState 96 questów / 1328 tracks / 3263 transitions; 310 wybranych receptur / 2012 etapów, wszystkie runtime_enabled=false.
- B4 zawiera 22 oceny brakującego przepływu i wymagań przestrzeni; B5 20 przypadków częściowych/niepotwierdzonych; sekcja różnic 11 zestawień. Mapy i inne istniejące assety wymagają odrębnego dowodu, zanim zostaną nazwane pracą od zera.
- P27 nie przypisuje fal Grizzly na podstawie pojedynczej areny. Statusy NIE są ograniczone do wskazanego zamkniętego modelu; nie dowodzą braku w całym silniku.

## Validation

- Focused source check: 7 wymaganych H2, 30 unikalnych Pxx, 373 zgodne tytuły, poprawne piny, istniejące linie, zgodne Wiki revid i brak placeholderów; zero wykrytych błędów.
- Niezależne przeliczenie czterech shardów Oteryn, quest-state i oryginalnego questlog potwierdziło liczby powyżej. Hash obu zapisów odpowiedzi Wiki zgadza się z raportem.
- Wymagane komendy kwalifikujące publikowaną zmianę: `python tools/agents/validate_governance.py`, `python -m unittest discover -s tools/agents/tests` i `git diff --check` (dla staged candidate także `git diff --cached --check`).
- Wyniki tych komend, candidate tree oraz dokładny opublikowany head są zapisywane poza własnym commitem w [PR #2836](https://github.com/Oteryn/Oteryn-Game/pull/2836). Stan CI jest oceniany wyłącznie dla tego head.
- Runtime/E2E: NOT_APPLICABLE do zakresu Prompt 0; nie uruchamiano gameplay.

## Self-review

Root sprawdził scalenie wszystkich tytułów, dowody braków i częściowego wsparcia, nazwy/linie źródeł, rozbieżności, wieloznaczne liczniki oraz granicę dwóch owned paths. Nie uznano szkieletu NPC, sklepu, pustego storage ani niepowiązanego transportu za działający quest.

## Independent review

Trzy subagentowe przeglądy rozłącznych tytułów i przypisanych źródeł wykryły błędy kotwic, brakujące rewizje bazowe i zbyt skrócone opisy B4. Poprawki zostały scalone i ponownie sprawdzone przez roota. Weryfikacja subagentowa nie zastępuje wyników wymaganych komend na publikowanym kandydacie. Nie uruchamiano płatnego review.

## PR and closeout

DRAFT PR #2836 pozostaje kanałem pytań. Otwarte: Q1 zakres po Prompt 0; Q3 powtórne malowanie Colours of Magic; Q4 próg aktywacji Fire-Feathered; Q5 próg resetu Fire-Feathered. Raport zachowuje alternatywy zamiast wybierać je bez decyzji. Komentarze są ponownie odczytywane przed publikacją i zakończeniem. Brak deklaracji merge, gotowego runtime ani zakończenia nieokreślonego zakresu kodu.

## Context checkpoint

- Report: authored and source-reviewed; publication qualification belongs to the exact-head receipt in PR #2836.
- Waiting: ANSWER dla Q1/Q3/Q4/Q5; niezależny zakres Prompt 0 został wykonany.
- Next implementation scope: wymaga odpowiedzi Q1 lub kolejnych promptów z ustalonymi kryteriami akceptacji.
