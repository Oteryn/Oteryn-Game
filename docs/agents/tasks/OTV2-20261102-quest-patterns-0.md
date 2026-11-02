# OTV2-20261102-quest-patterns-0

```yaml
task_id: OTV2-20261102-quest-patterns-0
title: Prompt 0 — katalog wzorców questów i porównanie Global
mode: AUDIT
status: investigating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: chatgpt/quest-patterns-0-20261102
pr: null
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

Jeden raport Markdown z siedmioma sekcjami wskazanymi w dostarczonym pliku PROMPT_QUESTY_0_wzorce.md: wzorce, lista Global/donor/Oteryn, liczby, brak we wszystkich donorach, niepełne u donora, rozbieżności, NIEZNANE.

## Architecture and source of truth

- Crystal: zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8.
- Canary: opentibiabr/canary@47dfd51f45280a59a1d3e50ba7edd573d7234446, tylko porównanie braków lub niejasności.
- Oteryn do porównania: Oteryn/Oteryn-Game@28eae9c341a7815abe0b403e9ac43285d1d85cbb. Baza publikacji jest nowsza; przypięty materiał porównawczy pozostaje bez zmiany.
- TibiaWiki: wyłącznie api.php, identyfikatory rewizji i pełna paginacja.
- PROVEN: użytkownik wymaga DRAFT PR bezpośrednio po pierwszym commicie oraz kanału QUESTION:/ANSWER: na tym PR.
- CONFLICT: wiadomość zlecająca wspomina pełne napisanie kodu, ale załącznik (Twarde zasady, punkt 5) wymaga „Bez kodu”. Pytanie zakresowe zostanie opublikowane po utworzeniu PR.
- Publikacja: wyłącznie ten branch; pojedynczy writer (lead). Lokalny transport Git zgłasza niedostępny browser-proxy; lead wybiera dozwoloną ścieżkę nowego kandydata przez GitHub Git Data, pełny delta, jeden parent, jeden non-force update i natychmiastowy readback. Brak wybranego lokalnego commita do zastąpienia.

## High-risk authority/recovery qualification

NOT_APPLICABLE: dokumentacja i analiza źródeł, bez zmiany runtime, protokołu, kont, danych produkcyjnych lub uprawnień.

## Acceptance criteria

- [ ] Wszystkie 123 katalogi Crystal przejrzane, rozłączne zakresy subagentów.
- [ ] Zmiany storage questów w NPC objęte spisem.
- [ ] Słownik P01, P02, ... i przykłady z numerami linii.
- [ ] Możliwości Oteryn potwierdzone w przypiętym kodzie, oddzielone od danych dziennika.
- [ ] Pełna lista Global przez API albo jawnie udokumentowany brak dostępu, bez zgadywania.
- [ ] Rozbieżności mają oba źródła; nieweryfikowalne pola oznaczone NIEZNANE.
- [ ] Otwarte pytania przeniesione do Left out w PR.
- [ ] Raport, źródła i wymagane kontrole zweryfikowane przed końcowym raportowaniem.

## Reference parity

NOT_APPLICABLE: katalog badawczy, nie implementacja mechaniki. Raport podaje fakty wraz z repo@commit:path:line albo tytułem Wiki i revid.

## Excluded scope

Implementacja Rust/Lua/JSON i aktywacja runtime oczekują rozstrzygnięcia konfliktu zakresu. Brak logowania do Tibii, połączeń z grą CipSoft, scrapowania HTML Wiki i obchodzenia zabezpieczeń. Bez merge i deploy.

## Implementation / findings

- Trzech subagentów: katalogi A–H, I–R, S–Z oraz niealfabetyczne; root scala wynik i bada Oteryn/Global.
- Przed każdym nowym etapem oraz zakończeniem lead ponownie czyta komentarze PR.
- Pytania o reguły, wartości i zakres: jeden komentarz QUESTION: na pytanie; odpowiedzi rozpoznawane po ANSWER:.

## Validation

- Focused: pending — kompletność katalogów, brak duplikatów, odnośniki i numery linii.
- Governance: pending — python tools/agents/validate_governance.py.
- Governance tests: pending — python -m unittest discover -s tools/agents/tests.
- Whitespace: pending — git diff --check.
- Runtime/E2E: NOT_APPLICABLE do zakresu analitycznego.
- Exact-head CI: pending.

## Self-review

Pending on final candidate.

## Independent review

Pending risk classification; no paid review triggered.

## PR and closeout

DRAFT PR immediately after this first commit; final head is recorded outside its own commit. No completion or merge claim.

## Context checkpoint

- Phase: AUTHORING.
- Next action: open DRAFT PR and post scope QUESTION, then complete independent research.
