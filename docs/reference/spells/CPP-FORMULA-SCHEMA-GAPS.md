# Sześć formuł Monka: źródło Canary i przyjęta normalizacja Oteryn

Wcześniejsza diagnoza traktowała brak `calculateFlatDamageHealing` w docelowym
czytniku jako brak implementacji. Sprawdzenie przyjętego kontraktu skorygowało
wniosek: **S5 jawnie odrzuca ten helper Canary jako wadliwy** i przyjmuje
`level_base_damage_healing`, wspólną krzywą Crystal/wiki. Nie trzeba dodawać
kolejnego helpera runtime, aby przygotować docelowe dane tych sześciu czarów.

Autorytet: `docs/architecture/OTERYN_SPELL_AUTHORING_SCHEMA_V1.md`, decyzje S5
oraz S16, SHA256
`2df19abc242230d9e8bc934c303c8ecccbb948d4106b0475f3bb18692928ba93`.
Dokument odczytano ze zwykłego GitHub API dla main
`eaa401001d5a91cc1cb62385223b6ad4d23111ac`; lokalna kopia jest identyczna.

## Dane docelowe i oryginalne

R53 zawiera sześć rzeczywistych pakietów Spell/Ability/Effect/Formula.
Formuła zachowuje źródłowe base power oraz zależność od skill/attack i granice
±10%, a składnik poziomu korzysta z przyjętej S5. S16 ustawia
`learning_required=false`; Forceful Uppercut i Mystic Repulse zachowują
`wheel_unlock=true`. To jawna normalizacja do Oteryn, a nie numeryczna
równoważność z bieżącym Canary. Nie zmienia wyboru aktywnego manifestu.

| Czar | Skrót katalogu źródłowego | Base power |
| --- | --- | --- |
| Double Jab | `f8352b29de559c08` | 50 |
| Flurry of Blows | `a0f21d6017982b10` | 65 |
| Forceful Uppercut | `6680f2fa7fe5978a` | 130 |
| Greater Flurry of Blows | `29db6bf8310ef9a2` | 100 |
| Mystic Repulse | `ab832ba328abde47` | 72 |
| Swift Jab | `a537acecb42bf775` | 12 |

Oryginalne nagłówki, needLearn, callbacki i dokładny program helpera Canary
pozostają w niezmienionych r28/r50. Ewaluator badawczy r50 przeszedł kontrolę
oryginalnym oracle C++; nie jest runtime Game. Docelowe r53 osobno sprawdza
przyjętą formułę przez oryginalny przypięty moduł Rust z PR #1534.

Źródło danych: lokalne obiekty Canary
`04b83b512114bfd888000d6e1433ed8ecaec7c5b`, odczyt przez `git show`.
Nie pobierano ponownie donorów ani stron wiki. Nie wykonywano działań przez
Remote Desktop. Dokładne dowody polityki, walidacji i sumy kontrolne znajdują
się w `r53-source-closure/projection-qualification.json` i lokalnym imporcie r53.

## Pozostaje kwalifikacja wykonania

Pakiety danych nie dowodzą kwalifikacji providerów rzeczywistej broni,
attack/skill, charges, RNG, konwersji wartości callbacku ani lifecycle Harmony.
Kwalifikacja native, providerów i aktywacja pozostają wyłączone. Ich podłączenie
należy do istniejącej pracy #1534 i koordynacji #1622. Nie zmieniono dzierżawionych
plików Rust ani historycznych receiptów r28/r50.
