# Odblokowanie danych: wynik partii r55–r58

Ten dokument opisuje historyczny etap r55–r58. Bieżący wynik danych r59–r62 jest
w `FINAL-129-DATA-COMPLETION.md`; nie przepisuje pierwotnego audytu 33/129.

Audyt objął dokładnie wszystkie 162 warianty z kolejki po r54. Cztery nowe
importy dodają **33 pełne kandydaty danych**. Indeks w tym historycznym etapie: **354 kandydatów
strukturalnych / 129 BLOCKED / 483 warianty źródłowe**. W 129 blokadach jest
4 wyłączonych przykładów i 2 wycofane Expose Weakness; **123 warianty gameplay
nadal wymagają uzupełnienia danych**.

| Import | Pełne nowe pakiety danych | Pozostałe rekordy audytu |
| --- | --- | --- |
| r55 | 21: 15 Crystal stance attacks i 6 bazowych Wheel | 43 konkretne blokady |
| r56 | 2: Canary Focus Harmony / Focus Serenity | 14 częściowych modeli i 9 bez pełnego kontraktu |
| r57 | 2: Summon Creature z obu donorów | 4 częściowe acquire-summon, 10 częściowych party i 12 bez kompletnej projekcji |
| r58 | 8: IHR, UHR, Cancel Magic Shield, Paralyze z obu donorów | 30 częściowych/proponowanych modeli, 6 referencji i 1 nierozstrzygnięty Mentor Other |

Dane są zmaterializowane w `imports/spells/r55/` do `imports/spells/r58/`.
Wszystkie 162 nagłówki źródłowe zachowano bajtowo. Normalizacje mają dowody
przyjętych decyzji; nie deklarujemy zgodności numerycznej lub semantycznej 1:1
z błędnym albo celowo zastąpionym zachowaniem donora.

## Co zostało naprawione

- S5/S6/S21 pozwalają zachować bazowe mechaniki Wheel oraz wybrać przyjęte
  mechaniki odpowiednika Canary dla 15 wariantów Crystal. Augments i różnice
  stance pozostają odrębnymi, jawnymi zależnościami.
- Focus ma pełny aktualny model danych native i sprawdzoną zgodność dziewięciu
  konkretnych helperów z zaakceptowanym źródłem. Koszty i tożsamości pochodzą
  z bieżącego źródła, a nie z podmienionego starego profilu.
- Summon Creature ma prawdziwy descriptor source variant i sprawdzone korzenie
  helperów. Wywołanie źródłowe `getManaCost` nie ma bindingu; przyjęta C.2
  ustanawia model kosztu `summoning.mana_cost`. Oryginalny błędny getter pozostaje
  dowodem, a równoważność surowego bindingu jest false.
- Pakiety leczenia i kontroli stosują przyjęte B.5/D.3/C.5/D.5. Review skorygował
  IHR: zachowano wszystkie 11 źródłowych vocations, również `none`, ponieważ
  ograniczenie UHR nie mogło być automatycznie przeniesione na IHR.

## Gotowość danych i wykonanie

33 nowe kandydaty są pełnymi pakietami **docelowych danych**, nie 33 czarami
uruchomionymi na serwerze. Dwa Focus i dwa Summon mają osobną blokadę równości
całego profilu w czytniku native; dwa IHR mają blokadę obsługi vocation `none`.
Pozostałe wymagają kwalifikacji wejść, providerów i właściwych mechanik silnika.
Wszystkie flagi runtime, native execution i aktywacji pozostają false.

Nie podniesiono statusów samych kopii canonical profiles ani aliases.
Przykładowo Animate Dead i Convince pozostają częściowe, ponieważ źródłowy
`setSummon` kopiuje również atakowany cel, a descriptor nie ma tej operacji.
Party pozostaje częściowe: potrzebne jest pełne odwzorowanie selekcji członków,
subID, mana/commit order i różnic geometrycznych. Dla pozostałych stance, Wheel,
Monka i world spells zapisano konkretne brakujące parametry i gałęzie.
Ich integracja z silnikiem pozostaje w istniejącej pracy #1534 / #1622.

Maszynowe zestawienie wszystkich 162: `unblocking-162-review-index.json`.
Ówczesna kolejka obejmowała 129; jej późniejsze domknięcie danych pokazuje
`final-129-completion-review-index.json`. Bieżący worklist runtime zachowuje
`source-private-consumer-worklist.json`.
Ówczesny `source-closure-review-index.json` obejmował 31 importów r28–r58.
Bieżący indeks obejmuje 35 importów r28–r62. Historycznych zestawów nie przepisano. Status potworów z r54 pozostaje
10 pełnych projekcji, 62 częściowe i 103 bez projekcji; 175 runtime-unqualified.

## Źródła i sprawdzenie

Dane donorów, helpery oraz cache wiki były dostępne lokalnie. Nie pobierano ich
ponownie ani nie używano Remote Desktop. Wykorzystano przypięte Canary/Crystal,
wcześniejsze importy i przyjęte kontrakty z SHA wskazanymi w projection proofs.
Nie deklarujemy świeżego odczytu wszystkich stron wiki.

24 testy producentów oraz 4 testy buildera postępu przeszły razem (28 PASS,
6,792 s). Importer przeszedł 7 testów (PASS, 4,321 s). Niezależny przegląd
zweryfikował wszystkie cztery źródłowe pakiety i importer przed materializacją.
Dokładne dowody i sumy kontrolne są w nowych manifestach. Brak PR, commitów,
pushów i zmian w `apps/`, `crates/`, `content/`, `server/`.

Końcowy audyt rzeczywistych kopii: PASS. Sprawdzono 654 artefakty danych,
kompletność membership, sourcePath bytes, SHA256SUMS, schemaRefs i wszystkie162
nagłówki. Indeks i wszystkie widoki odtworzono identycznie. `git diff --check`
PASS; brak zmian aktywnego contentu lub ścieżek aplikacji i silnika.

Pozostałe123 warianty gameplay według historycznych lanes: Wheel30,
Monk/equipment23, summons15, stance12, party11, world_target11, P4custom10,
other_cast10 i chain1. Dokładne bieżące przeszkody wskazuje audyt per registration,
ponieważ historyczna etykieta lane sama nie określa brakującej implementacji.
