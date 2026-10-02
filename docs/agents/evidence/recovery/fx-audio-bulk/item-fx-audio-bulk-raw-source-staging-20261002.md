# Efekty/audio Item — zbiorczy staging źródłowy

Status: OTS_HYPOTHESIS_ONLY, bez kwalifikacji Native, assetów i aktywacji runtime. Aktualny świadek: `1b026969a8b2236737bff537b3eddb3c909781b2`; historyczny audyt4f pozostaje niezmieniony.

Pełne trzy XML: 552 deklaracji z polami effect/shootType/meleeAttackEffect, 295 unikalnych jawnych ID po rozwinięciu zakresów. Pełne zachowane skrypty broni: Crystal10 na pin i Canary4; 12 ID z literalnych wywołań :id, 12 wspólnych z XML. Łączny zamknięty zakres 295 ID. Wersji nie sumujemy jako nowych Itemów. Każda deklaracja ma literalne pełne XML, SHA, zakres bajtów, nazwę i wszystkie własne atrybuty; każdy skrypt i właściwy zachowany plik C++ ma pełny tekst i pin.

Podział aktualnej tożsamości: {'CURRENT_WORLD_OWNER': 141, 'CURRENT_BOUND_ITEM_STATIC_JOIN': 149, 'IDENTITY_OR_MEMBERSHIP_HOLD': 5}. Są to wyniki jawnych bindingów/current membership, nie promocje. 0 ID ma jawny niezgodny token między pinami; 46 ma różnicę obecności. Samej nieobecności nie nazywamy ciszą ani sprzecznością.

Dźwięków nie zadeklarowano w tych XML. Skrypty broni mają 22 wierszy odwołań, a jeden dodatkowy skrypt kontekstowego zaklęcia 2 wiersze odwołań do dźwięków; wybrane pełne pliki C++ mają 315 takich wierszy. To kompletny wykaz zachowanych plików, nie wszystkich źródeł repozytorium. Dźwięk Cast/Impact w skrypcie i zewnętrzny dźwięk ataku/trafienia silnika pozostają oddzielnymi warstwami; literalne ID nie dowodzi wykonania rejestracji. Kierunek, pudło, typ amunicji/broni, żywioł i wynik walki są kontekstem. Nie dopisano wymyślonych eventów, AssetKeys ani numerów sound.

Obecny SourceImport/import-candidate model może zachować źródłowe hipotezy pod właściwym profilem importu i pinami. Ten raport jest dowodem zewnętrznym, nie przyjętym importem. Formalny mapper ma trzy pola FX i generuje zależności asset-shaped; nie ma mapowania dźwięków. Istniejący Presentation authoring oferuje powiązania visual/audio i Item→Presentation, ale wymaga realnych kwalifikowanych assetów/cues/tożsamości. Bez nich tokenów OTS nie można przedstawiać jako dopuszczonych zasobów.

Jeżeli potrzebny jest nowy typowany owner samych literalnych tokenów przed dopuszczeniem assetów, konkretna propozycja to opcjonalne media_source_observations przy istniejącym ItemAuthoring: wyłącznie source field/value/coordinates i OTS hypothesis, bez semantyki eventu/runtime/defaultów. Wymaga jawnego supplementu owning SourceProfileV2/master/formal schema, revision/migration i strict-compatible canonical tests. Kontekstowe C++/Lua pozostają oddzielnym dowodem, nie nowym programem w danych. Nie wprowadzać Native profile/enum/protocol. Candidate SPELL-PRESENT/graphics dokumenty nie nadają tej paczce autorytetu.

Źródła odczytano lokalnie z wcześniej zapisanych zwykłych publicznych HTTP GitHuba. Nie wykonano nowych zapytań web/RDC, zmian repo, Cargo ani Git/ref. Wszystkie wyniki mają pełne listy ID i sourcefile digests w towarzyszącym JSON.
