# R47 — czternaście wariantów guardów i akcji ze źródeł

Pakiet przechowuje dane z lokalnych, niezmiennych pinów Canary/Crystal. Nie pobierano danych z sieci ani wiki. Każdy rekord zawiera SHA pliku i Git blob, SHA historycznej referencji oraz receipt r28, ścisłe fakty źródłowe, kolejność zdarzeń leksykalnych i brakujące możliwości obecnego modelu.

**14 rekordów, 7 rodzin, 0 pełnych kandydatów Spell. Wszystkie nadal BLOCKED.** Nie zmienia to wyboru katalogu, runtime ani dopuszczenia czaru. Kolejność leksykalna całego pliku zachowuje markery branchów i nierozwiązane wyrażenia; nie jest wykonywalnym CFG. Dodatkowe kotwice przypisań zachowują m.in. progi odległości i współczynniki poza zdarzeniami call/guard. Pełna składnia Lua jest już w r38.

Najważniejsze różnice wymagające odrębnego modelu danych:

- Canary IHR/UHR mają odmowę dla potwora, a UHR dodatkowo sprawdza konwersję Player i vocation `exalted monk`; argument `1073762188` w `getNumber` nie jest kwalifikowany jako domyślny numer wariantu.
- Nature's Embrace odmawia self-target tylko dla caster Player z dokładną wiadomością i POFF. Crystal dodatkowo wywołuje drugi heal dla Shared Conservation po pierwszym Combat także gdy pierwszy zwrócił false. Selektor nie sprawdza viewport, a remis zachowuje pierwszego członka party.
- Blood Rage/Protector usuwają condition z dokładnym ID/subID przed Combat.
- Cancel Magic Shield bezwarunkowo usuwa condition z castera **przed** Combat; zwykły Effect dispel po walidacji nie zachowuje tej kolejności.
- Expose Weakness ma nadpisanie Variant bieżącym targetem oraz callback z wyjątkami Player/summon i grade Wheel.
- Find Person używa progów **5/101/275**, a istniejący descriptor przyjmuje **5/101/251**. Źródłowe wartości są zachowane oddzielnie.
- Crystal IHR/UHR mają osobną gałąź `leiden` z bezpośrednim addHealth, późniejsze odrzucenie innych potworów i ograniczenie self-only. IHR przekazuje do Combat nowy numeric Variant, UHR zachowuje incoming Variant.

Producent: `tools/content-schema/spell-authoring/source_simple_guard_candidates.py`. Schema: `source-simple-guard-evidence.schema.json` w tym samym katalogu narzędzi. Receipt opisuje hashe i klucze do nieaktywnego importu.
