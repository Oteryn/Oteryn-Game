# R52 — realne mapowanie do istniejącego czytnika czarów

Ten pakiet zawiera rzeczywisty `Spell`/`Ability`/`Effect`/`Formula`, a nie program AST do przyszłego wykonania. Canary Nature's Embrace otrzymuje player-only projection: `allowed_targets=not_self`, heal, dispel paralyze i dokładną źródłową formułę `(level / 2.5) + magic_level * 20/28`, poziom 300, mana 400. Dane przechodzą obecną schemę i validator; sprawdzono też dopuszczalne pola rzeczywistych Rust structs z `deny_unknown_fields`. Odsyłacz do Ability i wszystkie local dependency refs są gotowe do czytnika.

Istniejący runtime ma testy `part_b_tests` dla tej samej reguły `not_self` i zwykłej Ability healing. **Tego konkretnego nowego bundle nie wykonano jeszcze przez Rust reader ani na serwerze.** Receipt oddziela schema-valid target data od native execution i aktywacji.

Źródłowe różnice pozostają jawne: dokładna wiadomość odmowy i POFF z Lua nie mają pola w obecnym generic TargetIllegal rejection; domyślny viewport wymaga kwalifikacji operational provider. Zachowano te dane w źródłowym nagłówku, provenance i `remaining_mechanics`. Oryginalny nagłówek zachowuje również donor vocation display flags; nie są one operational requirements i obecny Rust reader odrzuca je jako unknown fields, więc nie wchodzą do wykonawczego bundle.

`ultimate-healing-caster-requirements.json` zawiera rzeczywiste częściowe mapowanie UHR: vocations z Canary poza Exalted Monk. Zwykły Monk pozostaje dozwolony. Źródłowe `none` jest jawnie zaznaczone jako nieobecne w obecnym enum Vocation; nie usunięto go po cichu. Pełny UHR nadal wymaga monster target guard — nie zastąpiono go inną regułą self-only.

`player-control-mapping-proposal.json` klasyfikuje dokładnie wszystkie 21 wariantów r49: co już wspiera istniejący reader, konkretne brakujące zachowanie, istniejący runtime endpoint i wymagane pole danych. Cancel Magic Shield nie jest ogłoszony jako wiernie odwzorowany, ponieważ source removeCondition przed Combat nie wynika z obecnego porządkowania side effects. Find Person 275 nie jest podmieniony na 251: obecny locate runtime ma ten drugi próg na stałe, więc sama zmiana schema nie wystarczy.

Nie zmieniono aktywnej zawartości, runtime, wspólnych schem ani katalogu wyboru. Producent korzysta tylko z pobranych lokalnie pinów i danych r28/r49.

## Dwie rzeczywiste canonical base Ability.chain

Forked Glacier i Forked Thorns mają obecnie pełne target bundles z istniejącą `Ability.chain`: odpowiednio 6 i 5 dalszych celów, fork, jump 4, initial range 7, bez backtracking i target filter. To jawne zastosowanie już przyjętego S23 bindingu z `chain-behaviours.json`; obecny donor Crystal ma jump 5 i jego pełne dane pozostają w r49. S5 korzysta z istniejącego `level_base_damage_healing`. S6 oddziela Wheel additional-target augment od bazowego Spell: reference do `ProjectV2AugmentBinding` jest w proof, kwalifikacja augmentu pozostaje u Wheel owner. Nie usunięto source unlock gate, ponieważ te rejestracje mają `needLearn(false)` i są level-unlocked.

## Cztery propozycje powiązań ze starymi native templates

Istniejący native reader wymaga **całkowitej równości** Spell i dependencies z embedded 67 profiles `spell-p2-r20`, także identity, header, koszty i poziom. Dlatego emitujemy osobno trzy niezmienione kopie reader-qualified templates i cztery source binding proposals (Blood Rage/Protector Canary, Find Person obu źródeł). Nie zmieniamy czterech source receipts na schema-valid przez podmianę identity.

**Równość ze starym template nie dowodzi poprawności jego danych dla obecnego źródła.** Przykładowo Blood Rage source ma poziom 60 i mana 290, template poziom 20 i mana 20. Wszystkie takie różnice są zapisane w proof. Obniżenie poziomu lub kosztu nie jest dopuszczone przez te artefakty. Metadata policy i ewentualne rozdzielenie reader identity od nagłówka należy do istniejącego właściciela #1534. Przyjęta reguła Find Person P7 używa 251 zamiast source 275, ale inne różnice nagłówka również wymagają kwalifikacji.

Łącznie: **3 nowe schema-valid target candidates**, **3 niezmienione stare native templates / 4 propozycje source bindings wymagające review**, 21 szczegółowych mapowań. Żaden z tych artefaktów nie aktywuje runtime ani nie deklaruje pełnej zgodności 1:1.
