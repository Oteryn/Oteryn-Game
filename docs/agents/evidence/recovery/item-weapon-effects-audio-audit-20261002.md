# Efekty i dźwięki broni — audyt 2026-10-02

Źródła Canary i Crystal przypisują efekty broniom na kilku poziomach: atrybuty XML konkretnego itemu, skrypt broni, domyślna reguła zależna od typu broni/amunicji oraz wynik walki. Sam XML nie obejmuje całego zachowania.

W aktualnym Oteryn uwzględnienie jest częściowe. Formalny schemat ma `presentation.effects`, `projectile_effect`, `attack_effect` i `sounds`. Konwerter `engine_items.py` mapuje XML `effect`, `shootType` i `meleeAttackEffect` na odnośniki do assetów. Nie mapuje dźwięków silnika. Odnośnik źródłowy nie jest dowodem dopuszczonego assetu, działającej animacji ani odtwarzania audio.

Opublikowany stan `4f690c1e8a8966a4d074dde94d5a884dbb34fb94`:

- 316 wpisów ItemAuthoring; **0 powiązań z Presentation**.
- Native ItemPresentation przechowuje tylko nazwę i opis. Native ItemWeapon nie ma pól efektu pocisku, efektu uderzenia ani dźwięku.
- Importer B1 oznacza `melee_attack_effect` jako `EXPLICIT_UNSUPPORTED_PRESENTATION_BINDING_V1`.
- Istnieje ogólny model Presentation z audio/visual bindings, lecz w 2613 aktualnych profilach Presentation nie ma ani jednego takiego powiązania. Nie jest to licznik samych broni.
- Weapon103 uzupełnia tylko modyfikator ataku i procent trafienia; nie zamyka efektów/audio.

Przykłady z Crystal `ff7ede593c69d4c658b382c97443e8155926924a`: Terra Rod3065 `shootType=smallearth`, Traditional Sai10389 `meleeAttackEffect=monkdaggers`, Drachaku10391 `meleeAttackEffect=monkstaff`. W tym XML 83 definicje broni z jawnym weaponType mają shootType, a22 broni fist ma meleeAttackEffect. Są to liczniki definicji źródłowych w przyjętym zakresie weaponType, nie liczba już zakwalifikowanych brakujących itemów Oteryn. Dodatkowo29 pozycji amunicji i2 pozycje bez jawnego weaponType mają shootType; nie utożsamiamy tych zakresów. Canary47df ma81 broni z jawnym weaponType i shootType,25 pozycji amunicji i2 bez jawnego typu; brak meleeAttackEffect w tym pinie. Osobny snapshot Crystal00ce ma odpowiednio87 broni z shootType i24 fist z meleeAttackEffect; wersji nie sumujemy.

Dźwięki nie występują jako klucze XML w sprawdzonych pinach. C++ wybiera dźwięki ataku/trafienia według broni i amunicji, a skrypty ustawiają własne parametry Cast/Impact. W Canary dźwięk zewnętrznego użycia broni może być wysłany przed callbackiem skryptu; potem Combat emituje własne dźwięki. Trzeba zachować obie warstwy, nie spłaszczać ich do jednej wartości nadpisującej. Literalne rejestracje ID w skrypcie nie dowodzą same w sobie skutecznego załadowania obu wariantów.

Pozostałe prace tego zakresu:

1. Zamknięty wykaz per-item efektów i skryptowych nadpisań, powiązany przez istniejące jawne ID/bindings; nie przez samą nazwę.
2. Osobne reguły dźwięków domyślnych zależnych od broni, amunicji i zdarzenia walki; nie przedstawiać wyniku reguły jako literalnego atrybutu XML.
3. Kwalifikowane powiązania do własnych assetów wizualnych/audio i zachowanie jawnych braków oraz konfliktów.
4. Minimalne dopuszczone przeniesienie do danych Presentation/Item i testy migracji.
5. W osobnym właściwym zakresie integracja ze zdarzeniami walki/protokołem Oteryn i test widocznego/słyszalnego rezultatu. Nie przenosić protokołu Canary do serwera.

Odczyt: lokalne Gitbloby opublikowanego Oteryn oraz zachowane, przypięte pliki XML; równoległe sprawdzenie C++/Lua przez zwykły HTTP publicznego GitHuba. Nie użyto Remote Desktop ani Tavily. Nie sprawdzono najnowszych upstream main; wyniki odnoszą się do podanych commitów. Nie wykonano zmian kodu projektu, Cargo ani nowych importów assetów.

Towarzyszący raport Oteryn: `item-weapon-effects-audio-oteryn-4f-audit-20261002.json`. Raporty źródłowe Canary/Crystal zawierają dokładne linie, commitowe URL i reguły wybierania dźwięków.
