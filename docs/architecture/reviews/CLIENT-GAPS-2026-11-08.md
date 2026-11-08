# CGMECH: jedna lista luk klienta (TIBAUD-1 + OTCAUD1108)

Data: 2026-11-08. Jedna lista dla PR #3058, scalona z dwóch audytów: `TIBIA-CLIENT-PARITY-2026-11-08.md` (oficjalny klient, wiersze `T`) i `OTCLIENT-PARITY-2026-11-08.md` (OTClient, wiersze `O`). Ujęto tylko wiersze PARTIAL i MISSING.

Zasady przeciw dublowaniu:

1. Każdy obszar ma dokładnie jedną linię. Wiersz `O` i wiersz `T` z tego samego obszaru należą do tej samej linii; linia realizuje je jedną implementacją i odhacza oba.
2. Przed implementacją linia sprawdza gałąź `cgmech` (zawiera main i wszystkie scalone linie). Jeśli funkcja już tam jest, tylko uzupełnia braki.
3. Funkcji z obszaru innej linii nie implementuje się; najwyżej podpina się istniejący kod.
4. Wiersze `OWNER` (mobilny joystick i pasek skrótów) czekają na decyzję właściciela.

| Linia | TIBAUD-1 | OTCAUD1108 | Razem |
|---|---:|---:|---:|
| cg_account | 1 | 0 | 1 |
| cg_analytics | 18 | 2 | 20 |
| cg_cyclo | 33 | 9 | 42 |
| cg_options | 43 | 12 | 55 |
| cg_ui | 39 | 9 | 48 |
| OWNER | 0 | 2 | 2 |
| razem | 134 | 34 | 168 |

## cg_account

| id | obszar | funkcja | status | brak |
|---|---|---|---|---|
| T165 | Account | Premium status/features i expiry | PARTIAL | Dane account/store są; brak dedykowanego Premium Panel z feature list i expiry; real-money zakup wykluczony. |

## cg_analytics

| id | obszar | funkcja | status | brak |
|---|---|---|---|---|
| T102 | Analytics | XP Analyser | DONE | evidence: crates/client-ui/src/features/0010-analyser/view.rs:48 + analyser_selector_projects_session_facts_without_resetting_other_tools (apps/client/src/features/0010-analyser/tests/play.rs); xp_rates_goal_eta_and_minute_chart_follow_confirmed_experience |
| T103 | Analytics | Loot Analyser | DONE | evidence: apps/client/src/features/0010-analyser/protocol.rs:27 + analyser_item_query_and_reply_use_the_production_command_mailbox (apps/client/src/features/0010-analyser/tests/play.rs); loot_items_reprice_confirmed_counts_rates_and_history |
| T104 | Analytics | Supply Analyser | DONE | evidence: crates/client-ui/src/features/0010-analyser/view.rs:129 + merged_analyser_controls_keep_selectors_prices_resolve_and_reset_distinct (apps/client/src/features/0010-analyser/tests/play.rs); supply_counts_costs_rates_and_charts_revalue_real_consumption |
| T105 | Analytics | Impact Analyser | DONE | evidence: crates/client-ui/src/features/0010-analyser/view.rs:195 + outgoing_and_incoming_metrics_reach_real_selectors_through_wire_facts_and_mailbox (apps/client/src/features/0010-analyser/tests/outgoing.rs); impact_uses_wire_damage_types_healing_rates_gauges_and_charts |
| T106 | Analytics | Input Analyser | DONE | evidence: crates/client-ui/src/features/0010-analyser/view.rs:265 + outgoing_and_incoming_metrics_reach_real_selectors_through_wire_facts_and_mailbox (apps/client/src/features/0010-analyser/tests/outgoing.rs); input_preserves_wire_sources_elements_rates_and_reset |
| T107 | Analytics | Hunting Session Analyser | PARTIAL | Brak zegara, XP/h, profit/balance, kills/loot breakdown i save/export sesji. |
| T108 | Analytics | Party Hunt Analyser | PARTIAL | Brak tabeli członków i rozliczenia loot/supplies/cost split. |
| T109 | Analytics | Loot Tracker | PARTIAL | Brak listy śledzonych itemów i historii dropów/config. |
| T110 | Analytics | Kill / Hunting Task Tracker | PARTIAL | Brak samodzielnego trackera; postęp pozostaje w Task Board. |
| O012 | Analytics | Analizery polowania, XP, łupu, zapasów, obrażeń i drużyny | PARTIAL | Brakuje osobnych analizatorów z wykresami/stawkami w czasie, party hunt i boss cooldown/drop tracker; istnieje jeden panel sum kill/XP/loot/supply/damage/heal. |
| O036 | Analytics | Kalkulator podziału łupu drużyny | DONE | evidence: apps/client/src/features/0010-analyser/input/party_import.rs:11 + party_summary_file_io_is_bounded_advisory_and_keeps_live_members_untouched (apps/client/src/features/0010-analyser/tests/party_import.rs) |
| T022 | Chat | Tabbed console, chat on/off i filtry | PARTIAL | Transport jest; brak pełnej konsoli z niezależnymi filtrami i kontekstami chat-on/off. |
| T023 | Chat | Save / append channel log | MISSING | Brak zapisu i dopisywania treści kanału do pliku. |
| T024 | Chat | Ignore / whitelist / blacklist | MISSING | Brak dedykowanych list komunikacji i wyjątków. |
| T038 | Containers | Managed containers i loot destinations | PARTIAL | Listy/destination/nearby loot są; brak pełnego menu managed containers i przenoszenia całej zawartości. |
| T139 | NPC | Buy/sell/amount/backpack/capacity/filters | PARTIAL | Sklep istnieje; brak pełnej listy opcji ignore capacity/equipped/backpack i prezentacji. |
| T029 | Party | Team Finder assemble/join | PARTIAL | Ogłoszenia i operacje członka są; brak pełnej kompozycji Social, filtrów i listy zespołów. |
| T026 | Social | Friends, main character, zaproszenia | PARTIAL | Są zgody/status VIP; brak pełnego kontowego Friends i main character. |
| T027 | Social | Badges: widoczność i filtry | PARTIAL | Odczyt jest; brak edycji publicznej widoczności i filtrów locked/hidden/public. |
| T140 | Training | Offline / exercise / multi-character dialog | PARTIAL | Komendy offline/exercise są; brak pełnego multi-character wyboru treningu. |

## cg_cyclo

| id | obszar | funkcja | status | brak |
|---|---|---|---|---|
| T148 | Achievements | Account list/points/notices | PARTIAL | Odczyt/notices są; brak pełnego filter/sort regular/secret i wizualnej listy. |
| T163 | Boss | Difficulty selection / preview/start/cancel | PARTIAL | Komendy są; wybór wymaga context UUID i numeru 0–25 zamiast pełnego kontekstowego selektora. |
| T164 | Boss | Hazard level i encounter entry | PARTIAL | Query/select/entry są; brak pełnego wizualnego panelu zagrożenia i prowadzenia wejścia. |
| O074 | Boss | Cyklopedia: bosstiariusz | PARTIAL | Brakuje przeglądarki bossów z wyszukiwaniem, kategoriami i kartami szczegółów; istnieje lista kluczy bossów i liczników postępu. |
| T077 | Cyclopedia | Wspólne okno i nawigacja | PARTIAL | Fasada i osobne panele nie tworzą zintegrowanego CyclopediaDialog. |
| T078 | Cyclopedia | Items: kategorie/search/statystyki/źródła | PARTIAL | Inspect/catalogue/market są; brak kompletnej encyklopedii Items i jej filtrów. |
| T079 | Cyclopedia | Map: areas/subareas/passages/PoI/NPC | PARTIAL | Query/start/shuffle/find/donate przez formularz; brak dużej mapy z regionami i NPC. |
| T080 | Cyclopedia | Bestiary: katalog/filter/combat/loot | PARTIAL | Są kills/stage/thresholds; brak pełnego katalogu, ilustracji i danych resistance/loot. |
| T081 | Cyclopedia | Charms: points/unlock/assign/remove/reset | PARTIAL | Operacje są; wybór wymaga indeksów, brak pełnego selektora i opisów. |
| T082 | Cyclopedia | Bosstiary: kategorie/points/progress | PARTIAL | Jest postęp i boss key; brak galerii, filtrów kategorii i pełnej prezentacji etapów. |
| T083 | Cyclopedia | Boss Slots: equip/remove/bonus | PARTIAL | Komendy są; brak pełnego selektora bossów i prezentacji bonusów. |
| T084 | Cyclopedia | Houses: browse/details/auction/map | PARTIAL | Są wire/state i osobny transfer; house/controller.rs jest montowany tylko w teście, bez play.json i produkcyjnego okna Houses/mapy. |
| T085 | Cyclopedia | Analytics selector | PARTIAL | Jeden panel zbiorczy zamiast selektora i osobnych narzędzi. |
| O022 | Cyclopedia | Cyklopedia: nawigacja między działami | PARTIAL | Brakuje wspólnego okna Cyklopedii łączącego działy; Oteryn otwiera oddzielne formularze postaci, bestiariusza, mapy i inspekcji. |
| O072 | Cyclopedia | Cyklopedia: bestiariusz i charmy | PARTIAL | Brakuje przeglądarki kategorii i kart potworów ze statystykami, odpornościami i łupem; formularz pokazuje indeksy ras, zabicia i progi. |
| O077 | Cyclopedia | Cyklopedia: katalog przedmiotów | PARTIAL | Brakuje katalogu przedmiotów z kategoriami i wyszukiwaniem oraz powiązanych ofert/dropów; istnieje inspekcja wybranego przedmiotu i osobny rynek. |
| O078 | Cyclopedia | Cyklopedia: mapa i odkrywanie | PARTIAL | Brakuje graficznej mapy Cyklopedii z warstwami, regionami i filtrami; formularz pokazuje tekstowy stan odkrywania oraz POI/donacji. |
| T086 | Cyclopedia / Character | Overview / General Stats | PARTIAL | Brak pełnego wizualnego overview i układu statystyk. |
| T087 | Cyclopedia / Character | Offence / źródła i spell augments | PARTIAL | Tekstowy odczyt; brak pełnego układu i interaktywnych opisów źródeł. |
| T088 | Cyclopedia / Character | Defence / mitigation/resistance | PARTIAL | Brak pełnej tabeli/układu i interaktywnych szczegółów. |
| T089 | Cyclopedia / Character | Misc Stats / skills/stamina | PARTIAL | Dane tekstowe; brak pełnej prezentacji i pasków. |
| T090 | Cyclopedia / Character | Recent Deaths | PARTIAL | Stronicowanie jest; brak pełnej tabeli szczegółów. |
| T091 | Cyclopedia / Character | Recent PvP Kills / Battle Results | PARTIAL | Brak pełnej tabeli i filtrów rodzajów zabójstw. |
| T092 | Cyclopedia / Character | Achievements | PARTIAL | Brak pełnego sort/filter regular/secret. |
| T093 | Cyclopedia / Character | Appearances / outfits/mounts/familiars | PARTIAL | Tekstowy odczyt; brak galerii wyglądów/mountów/familiarów. |
| T094 | Cyclopedia / Character | Store Summary / services/hirelings | PARTIAL | Brak pełnego wizualnego zestawienia usług. |
| T095 | Cyclopedia / Character | Item Summary / inventory/depot/stash/inboxes | PARTIAL | Jest Inspection/item summary; brak grid/list/filter wszystkich lokacji. |
| T096 | Cyclopedia / Character | Titles: unlock/select/filter | PARTIAL | Operacje istnieją; brak pełnego selektora i filtrów tytułów. |
| T097 | Cyclopedia / Character | Badges / cudza postać / inspection | PARTIAL | Odczyt/zgody są; brak spójnego pełnego profilu i zakładki badges. |
| O015 | Cyclopedia / Character | Błogosławieństwa i ochrona po śmierci | PARTIAL | Brakuje panelu prognozy kary XP/PvP i utraty ekwipunku przed śmiercią; dostępne są listy błogosławieństw i ich zużycia po śmierci. |
| T143 | Highscores | Kategorie/world/vocation, paging/show-me | PARTIAL | Są category/vocation/refresh/previous/next; brak world selector i show-me; BattlEye wykluczone. |
| T113 | Hunting tasks | Bounty, rewards, shop i talisman | PARTIAL | Główne operacje są; brak pełnej prezentacji sklepu/drzewa ulepszeń i odbioru wszystkich nagród jak w referencji. |
| T114 | Hunting tasks | Soul Seals fight dialog | PARTIAL | Licznik soulseals jest; brak dedykowanego fight dialog i wyboru starcia. |
| T076 | Minimap | Exploration/markers/labels, floor/zoom/pan | PARTIAL | Bieżący server window zamiast pełnej trwałej mapy i edytora markerów. |
| O039 | Minimap | Minimapa i nawigacja | PARTIAL | Brakuje trwałej mapy odkrytych terenów, edytowanych znaczników, przewijania i zmiany pięter/zoomu minimapy; nakładka pokazuje bieżące okno świata. |
| T142 | Quest | Tracker auto-add/remove i filters | PARTIAL | Track quest jest; brak pełnego osobnego trackera, auto-reguł i hide/completed filters. |
| O050 | Quest | Dziennik zadań i śledzenie | PARTIAL | Brakuje wyszukiwania i sortowania dziennika; istnieją wybór zadania/misji, opis i przełączanie śledzenia. |
| T098 | Trackers | Bestiary tracker | PARTIAL | Add/remove/read po creature key; brak pełnego widżetu sort/config. |
| T099 | Trackers | Bosstiary tracker | PARTIAL | Wspólny edytor z checkbox Bosstiary; brak osobnego widżetu z konfiguracją. |
| T100 | Trackers | Boss cooldowns | PARTIAL | Odczyt cooldowns w zbiorczym formularzu; brak samodzielnego widget. |
| T101 | Trackers | Imbuement remaining time | PARTIAL | Stan imbuement/timed items jest; brak dedykowanego trackera wyposażenia i alertów. |
| O030 | Trackers | Tracker czasu imbuementów wyposażenia | PARTIAL | Brakuje stale widocznego trackera wszystkich wyposażonych imbuementów z filtrami czasu; czas jest pokazany tylko dla przedmiotu w oknie imbuingu. |

## cg_options

| id | obszar | funkcja | status | brak |
|---|---|---|---|---|
| T042 | Action bars | Spell slots i cooldown | PARTIAL | Cztery sloty w formularzu; brak pełnych rzędów i wskaźników cooldown/amount. |
| T043 | Action bars | Object/text/passive ability assignments | PARTIAL | Brak spójnego przypisywania wszystkich typów akcji w barze. |
| T044 | Action bars | Bottom/left/right, lock, drag/drop, scroll | PARTIAL | Brak dokowanych wielorzędowych barów, przenoszenia slotów i blokady układu. |
| O011 | Action bars | Konfigurowalne paski akcji | PARTIAL | Brakuje wielu konfigurowalnych pasków z akcjami przedmiotu/tekstu i układem slotów; natywny pasek obsługuje cztery przypisania czarów. |
| T015 | Battle list | Wiele list, sort/filter, rename | PARTIAL | Jedna lista; brak zarządzania wieloma listami, filtrowania i ich nazw. |
| O014 | Battle list | Battle list i wybór celu | PARTIAL | Brakuje filtrów typu/profesji/gildii i wyboru sortowania listy; obecny panel sortuje encje według identyfikatora. |
| T010 | Game window | HUD i sidebar: dock/show/hide/order/layout | PARTIAL | Osobne okna Win32 zamiast pełnego dokowania i zapisu rozmiarów/kolejności paneli. |
| T011 | Game window | HP/mana, soul/stamina, conditions/rest | PARTIAL | Dane i teksty są; brak pełnej konfigurowalnej prezentacji pasków/łuków/ikon. |
| T012 | Game window | Światło i efekty wizualne | PARTIAL | Światło i zdarzenia są; brak całego zestawu widocznych efektów i konfiguracji referencji. |
| T162 | Game window | Monk harmony/serene/combo i HUD controls | PARTIAL | Harmony/serene tekstowo są; brak pełnych pasków/combo i niezależnych ustawień HUD mnicha. |
| O010 | Game window | Górne menu i przyciski okien | PARTIAL | Brakuje wspólnego paska przycisków otwierających okna i zarządzającego ich stanem; Oteryn udostępnia okna przez skróty. |
| O021 | Game window | Nazwy, zdrowie i ikony nad stworzeniami | PARTIAL | Brakuje pełnego zestawu pasków zdrowia i ikon party/skull nad postacią w widoku świata; potwierdzone tarcze/czaszki są prezentowane w osobnym panelu. |
| O025 | Game window | Okręgi HP/MP i status mnicha | PARTIAL | Brakuje okręgów HP/MP wokół postaci; wartości zasobów i stany mnicha są dostępne w tekstowym HUD. |
| O033 | Game window | Układ interfejsu i okna boczne | PARTIAL | Brakuje dokowanych, przeciąganych i zapamiętywanych miniokien w kolumnach interfejsu; istnieją osobne okna Win32 i przełączniki widoczności. |
| O037 | Game window | Główny panel skrótów funkcji | PARTIAL | Brakuje zwijanego głównego panelu ikon funkcji gry z utrwalonym układem; funkcje są otwierane skrótami. |
| T040 | Hotkeys | Profile, chat on/off, primary/secondary | PARTIAL | Jedna mapa bindingów; brak nazwanych presetów i dwóch kontekstów czatu. |
| T041 | Hotkeys | Object equip / smart equip | PARTIAL | Komenda equip jest; brak pełnego selektora obiektów i opcji smart UI. |
| O028 | Hotkeys | Skróty klawiszowe i akcje przedmiotów | PARTIAL | Brakuje menedżera dowolnych skrótów tekstowych i użycia przedmiotów na self/target/cursor; istnieją przypisania akcji klienta, czarów i formularz equip. |
| T045 | Options | Basic Options | PARTIAL | Jeden mały Settings zamiast zbiorczej strony Gameplay/Interface/Graphics/Sound. |
| T046 | Options | Controls / General: mouse preset, movement, rotation, keyboard delay | PARTIAL | Rebinding jest; brak presetów myszy, rotation modifier i keyboard delay. |
| T047 | Options | Controls / General Hotkeys | PARTIAL | Brak równoważnej strony i presetów chat-on/chat-off. |
| T048 | Options | Controls / Action Bar Hotkeys | PARTIAL | Brak kompletnej konfiguracji wszystkich rzędów. |
| T049 | Options | Controls / Custom Hotkeys | PARTIAL | Brak pełnego edytora własnych object/text/passive actions i profili. |
| T050 | Options | Interface / General: language/scale, cursor i tooltips | PARTIAL | Skala mapy i pole języka są; brak pełnej skali UI, kursora i tooltip controls. |
| T051 | Options | Interface / HUD: player/creature names, HP/mana, marks, NPC icons | MISSING | Brak osobnych przełączników dla owner i pozostałych aktorów. |
| T052 | Options | Interface / HUD: arcs/distance/size/opacity | MISSING | Brak konfigurowalnych łuków HP/mana. |
| T053 | Options | Interface / HUD: status bars/panel, style, skills/condition order | PARTIAL | Dane statusu są; brak edytora stylu, pozycji i kolejności. |
| T054 | Options | Interface / Console: font/time/seconds/level | MISSING | Brak kontrolek fontów, timestamps i poziomów. |
| T055 | Options | Interface / Console: info/event/status/private/store filters | MISSING | Brak niezależnych filtrów i opcji private tab. |
| T056 | Options | Interface / Game Window: own/other spells, text, loot messages | PARTIAL | Zdarzenia są; brak przełączników według źródła i klasy tekstu. |
| T057 | Options | Interface / Game Window: target/PvP frame, banner, loot highlight | PARTIAL | Cel jest wybierany; brak konfiguracji ramek/banerów/kolorowania loot. |
| T058 | Options | Interface / Action Bars: rows/amounts/labels/tooltips/cooldowns/auto-insert spells | PARTIAL | Brak pełnej strony ustawień barów. |
| T059 | Options | Interface / Shortcuts: available/displayed, add/remove/order | PARTIAL | Rejestr akcji i visible_panels są; brak selektora i ustalania kolejności przycisków. |
| T060 | Options | Graphics: engine, AA, fullscreen, VSync, integral scaling | MISSING | Brak strony konfiguracji obsługiwanych parametrów renderera/okna. |
| T061 | Options | Graphics: foreground/background FPS i FPS/latency indicator | DONE | Evidence: apps/client/src/features/local-reference-preferences/performance_controls.rs:8 + apps/client/src/features/local-reference-preferences/frame_shell.rs:4 + apps/client/src/play/session_loop.rs:127; t061_graphics_performance_preferences_survive_real_settings_apply_and_reload; t061_scheduler_tracks_focus_occlusion_successful_frames_and_stale_samples; t061_latency_comes_from_real_session_command_caller_and_clears_on_logout. Foreground/background limits pace native redraws; optional measured FPS excludes skipped frames; command RTT uses real session replies and clears on logout. |
| T062 | Options | Graphics / Effects: ambient/separator/indoor/cloud attenuation | PARTIAL | Światło jest; brak grupy ambient/separator/attenuation. |
| T063 | Options | Graphics / Effects: own/other/creature/boss-area opacity | MISSING | Brak ustawień opacity według źródła efektu. |
| T064 | Options | Sound: master/mute/device/music/ambient/items/events/UI | PARTIAL | Tylko master/mute; brak wyboru urządzenia i niezależnych kanałów miksera. |
| T065 | Options | Sound / Battle Sounds: own/others/creatures, attack/heal/support | PARTIAL | Playback istnieje; brak suwaków i filtrów walki. |
| T066 | Options | Sound / UI Sounds: interactions/party/VIP/chat channels | MISSING | Brak filtrów dźwięków UI, party, VIP i kanałów. |
| T067 | Options | Miscellaneous: confirmations, stay logged in, quick login | PARTIAL | Pojedyncze confirmation istnieją; brak wspólnej strony sesji/potwierdzeń. |
| T068 | Options | Miscellaneous: optimise connection stability | MISSING | Brak ustawienia optymalizacji połączenia i jego konsumenta w panelu Settings. |
| T069 | Options | Miscellaneous / Gameplay: inspect/chase cancel/nearby loot scheme | PARTIAL | Inspect/loot/walk są osobno; brak pełnej strony i chase cancel/mouse loot controls. |
| T070 | Options | Miscellaneous / Screenshots: folder, game-only i manual capture | MISSING | Brak systemu zrzutów i strony wyboru katalogu/zakresu. |
| T071 | Options | Miscellaneous / Screenshots: backlog i event triggers | MISSING | Brak bufora wstecznego i automatycznych screenshotów zdarzeń. |
| T072 | Options | Miscellaneous / Help: FAQ/manual/client help/reports | PARTIAL | Raportowanie jest osobno; brak kompletnej strony pomocy/nawigacji. |
| T073 | Options | Help: settings import/export/backup/reset/defaults | PARTIAL | Jest zapis i walidacja preferencji; brak UI import/export, backup i pełnego reset/defaults. |
| T074 | Options | Help: minimap import/export | MISSING | Brak narzędzia wymiany trwałych map/markerów. |
| O005 | Options | Lokalizacja interfejsu | PARTIAL | Brakuje wyboru i kompletnego tłumaczenia interfejsu dla katalogu języków OTClient; ustawienia Oteryn przewidują przełącznik PL/EN. |
| O006 | Options | Opcje i ustawienia | PARTIAL | Brakuje pełnych stron ustawień grafiki/efektów, konsoli i układu pasków akcji z OTClient; obecny formularz zapisuje ograniczony zestaw preferencji. |
| T031 | Skills | Umiejętności, progress, loyalty | PARTIAL | Dane/progress są; brak pełnego Skills widget z belkami i konfiguracją statystyk. |
| T032 | Skills | Spell list / spellbook | PARTIAL | Jest katalog po name/words; brak wszystkich filtrów vocation/level/premium/group i pełnej książki czarów. |
| T033 | Skills | Magical Archive i przypisanie czaru | PARTIAL | Katalog i sloty są; brak osobnego pełnego archiwum i graficznego przypisywania. |
| O059 | Skills | Lista czarów i wyszukiwanie | PARTIAL | Brakuje filtrów profesji/grupy i szczegółowego widoku wybranego czaru z czasem odnowienia; istnieje stronicowana lista z wyszukiwaniem nazwy/formuły. |
| O080 | Skills | Cyklopedia: Magical Archive | PARTIAL | Brakuje archiwum czarów z filtrami profesji/grupy i kartą szczegółów; dostępny jest podstawowy katalog czarów postaci. |

## cg_ui

| id | obszar | funkcja | status | brak |
|---|---|---|---|---|
| T149 | Appearance | Outfits/mounts/addons/colors | PARTIAL | Apply przez klucze/mask/colors/revision; brak galerii i preview. |
| T150 | Appearance | Outfit Memorial golden/token outfits | MISSING | Brak zakładek token exchange i preview memorial outfits. |
| O043 | Appearance | Wybór stroju, dodatków i kolorów | PARTIAL | Brakuje wizualnego podglądu stroju i selektorów palety/addonów; obecny formularz wymaga klucza stroju, maski dodatków i czterech indeksów kolorów. |
| T127 | Depot | Search/details po item/tier, retrieve | PARTIAL | Retrieve one/all są już w cgmech; dodano indeksy, sumy, tier i zakres retrieve-all. Brak pełnego natywnego widżetu wyboru wyników. |
| T118 | Exaltation | Tier effects i rezultat exaltation | PARTIAL | Są wyniki forge i item effects; brak pełnej prezentacji tier bonuses i wyniku. |
| T116 | Forge | Fusion/convergence i transfer/convergence | PARTIAL | Numeric operation/item handles/occurrence zamiast selektora i podglądu przedmiotów. |
| T117 | Forge | Dust/slivers/cores/limit i historia | PARTIAL | Odczyt/mutacje/historia są; brak pełnego confirmation/result dialog. |
| O041 | Game window | Banery i powiadomienia gry | PARTIAL | Brakuje ogólnego systemu konfigurowalnych banerów/okienek zdarzeń; dostępne są komunikaty samouczka i HUD. |
| T121 | Gems | Reveal/socket/unsocket/dismantle/enhance | PARTIAL | Komendy/selektory są; brak kompletnej wizualnej prezentacji gem/mod i atelier. |
| T134 | House | Guest/subowner/door ACL, bid/leave/notice | DONE | Montaż play.json, natywne okno i submit/HouseResult są już w cgmech; dodano informacje o braku środków, limicie i konieczności odświeżenia. |
| T135 | House | Transfer/move-out/furniture wrapping | PARTIAL | Dodano potwierdzenie dokładnego itemu i celu wrap/place; pozostaje pełny wybór transfer/move-out i pozostałe potwierdzenia. |
| T136 | House | Podium: outfit/mount/creature/item | PARTIAL | Dodano tekstowy preview outfit/mount/item z kierunkiem; brak graficznego preview i przełączników creature podium. |
| T137 | House | Hireling jobs/name/sex/outfit config | PARTIAL | Podgląd nazwy/płci, jobów i oczekującego outfitu jest; brak wspólnego pełnego edytora name/sex/jobs/outfit. |
| O079 | House | Cyklopedia: katalog domów i aukcje | DONE | Sesja i natywne okno są już zamontowane w cgmech; uzupełniono własność, typ, termin aukcji, własny max bid i paginację. |
| O083 | House | Edycja list gości, współwłaścicieli i drzwi | DONE | Natywny edytor i HouseResult są zamontowane; door ACL korzysta z rewizji wybranego domu bez ręcznego wpisywania rewizji. |
| T115 | Imbuing | Shrine: apply/clear, sloty i materiały/szansa | PARTIAL | Komendy/lista są; brak pełnego interaktywnego widoku slotów i szczegółów koszt/szansa. |
| T002 | Login | Połączenie, re-entry i logout | DONE | Widoczna ponowna próba, odliczanie admission i anulowanie pojedynczej próby bez zatrzymania runtime (`68169ce25`). |
| T003 | Login | Pozycja kolejki i odliczanie | PARTIAL | Odliczanie, automatyczne ponowienie i anulowanie są podłączone do istniejącej odmowy capacity; brak autorytatywnej pozycji kolejki oraz jej producenta/protokołu na serwerze (`68169ce25`). |
| T004 | Login | News i kalendarz zdarzeń | PARTIAL | Jest ograniczony rozmiarem feed pakietu `bulletin.json`, news i kalendarz z opisami; brak żywego feedu i publikacji przyszłych wydarzeń przez usługę operatora (`8ba0c9983`, `f86998757`). |
| T005 | Login | Launcher, wersja, kontrola/aktualizacja pakietów | PARTIAL | Jest ekran `--check-install` z aktywnym wydaniem i instrukcją naprawy; brak kontroli integralności pakietów oraz wykonania aktualizacji/naprawy (`5a41232a7`). |
| O002 | Login | Kalendarz wydarzeń i ekran nowości | PARTIAL | Jest news feed z pakietu oraz kalendarz z opisami i nawigacją miesiąc/rok; brak żywego producenta harmonogramu wydarzeń (`f86998757`). |
| T124 | Market | Browse/buy/sell/create/cancel/history | PARTIAL | Dodano statystyki loaded item page oddzielnie buy/sell (min/max/weighted average); brak pełnych filtrów i globalnych statystyk/historii cen. |
| T125 | Market | Coin market / transaction details | PARTIAL | Dodano order id/revision/debit w historii zakupów za coins; brak dedykowanego coin market i wire tej funkcji. |
| T158 | Modal dialogs | Amount/text/edit/confirm/dont-show-again | PARTIAL | Są count confirmation/formularze; brak pełnego wspólnego zestawu text editor i trwałego dont-show-again. |
| T159 | Modal dialogs | Server choices/buttons i show-image | MISSING | Brak ogólnego modal UI sterowanego serwerem; zwykłe okna mechanik go nie zastępują. |
| T160 | Modal dialogs | Blessings / death protection overview | PARTIAL | Stan i death notices są; brak pełnego dedykowanego Blessings dialog. |
| T166 | Modal dialogs | Nagging / balancing / generic notices | PARTIAL | Wiadomości serwera są; brak dedykowanych interaktywnych dialogów i dont-show-again. |
| O040 | Modal dialogs | Ogólne dialogi modalne serwera | MISSING | Brakuje ogólnego handlera dialogu z identyfikatorem, listą wyborów i przyciskami oraz odpowiedzią wyboru do serwera. |
| T123 | Proficiency | Weapon perks/rank i shape/reshape | PARTIAL | Operacje/katalog są; ręczne id/perk index/slot/revision zamiast pełnego wyboru wizualnego. |
| T017 | PvP | Unjustified points i skull timer | PARTIAL | Są stany PvP/warunki; brak pełnego Unjustified Points widget. |
| O066 | PvP | Unjustified points i limity fragów | PARTIAL | Brakuje liczników dziennych/tygodniowych/miesięcznych fragów, czasu czaszki i otwartych sytuacji PvP; obecny panel pokazuje tylko tarcze i czaszki. |
| T144 | Rewards | Daily wall/streak/jokers/claim/pick items | PARTIAL | 30-day wall/claim są; brak pełnego kalendarza, historii i pick-items dialog. |
| T145 | Rewards | Returner reward | MISSING | Brak dedykowanego przepływu i notyfikacji nagrody powrotnej. |
| T147 | Rewards | Boosted creature/boss i daily boost | PARTIAL | Status/komendy są; brak pełnego daily spotlight z prezentacją boosted creature/boss. |
| T128 | Stash | Stow/withdraw/paging i search/filter/sort | PARTIAL | Search strony i sort name/stock działają w oknie z zachowaniem handle withdrawal; brak global search i filtrów trader/category (brak metadanych w wire). |
| T130 | Store | Katalog/search/buy/confirmation/result | PARTIAL | Natywne search/sort strony i preview oferty zachowują server quote; brak pełnej nawigacji kategorii i graficznego preview. |
| T131 | Store | Purchase history i coins history | PARTIAL | Filtry offer/time strony, szczegóły i sumy per currency są; brak pełnej historii ledger monet i typów top-up/transfer (Platform). |
| T132 | Store | Transfer/gift monet w grze | PARTIAL | Dodano natywne recipient/amount/review/confirm w trybie disabled, bez komend i debit; transfer wymaga kontraktu i endpointu Platform, którego STORE_V1 nie definiuje. |
| T151 | Support | Bug report map/technical/typo/other | DONE | Istniejący formularz kategorii/preview/zgody na build rozszerzono o wskazówki map/typo/technical/other; używa istniejącej komendy raportu (`a8040346e`). |
| T152 | Support | Violation name/statement/bot/reasons | DONE | Są wybór name/statement/bot, nazwany selektor 21 powodów ze wskazówkami dowodowymi, opcjonalna oznaczona translacja i confirmation wiązane z dokładnym payloadem (`55eddb04c`, `6433921fb`). |
| T153 | Support | Crash report i zgoda | DONE | Lokalny hook zachowuje ograniczone dane build/OS/architektury i nazwę pliku źródła; następny start pokazuje preview i wymaga zgody na zapis bez uploadu (`fae511e6e`). |
| T154 | Support | About/version i survey invitation | DONE | Okno logowania udostępnia About (wydanie/build/platforma/licencja) i własną opcjonalną ankietę z preview oraz zgodą na lokalny zapis (`1697f40d5`). |
| O071 | Support | Zrzuty ekranu i automatyczne wyzwalacze | PARTIAL | Jest zapis BMP z walidacją klatki/wycinka i logika opt-in/deduplikacji; brak przechwytywania klatki renderera, UI zakresu oraz podłączenia zdarzeń runtime (`fb89d1fa9`). |
| T155 | Tutorial | Hints/markers/mouse trail i server-save notice | DONE | Ctrl+Alt+Shift+N włącza renderowane wskazówki kontekstowe, marker kursora i ograniczony czasem/rozmiarem mouse trail; server-save notice jest dostępny (`d2241ea6c`). |
| T156 | Tutorial | Player Guide / compendium / recommendations | DONE | Zakładki Player Guide i Recommendations są w oknie Ctrl+Shift+N, obok komunikatów samouczka i server-save (`0ef49c5a8`). |
| O017 | Tutorial | Kompendium: poradnik, aktualizacje i pomoc | DONE | Kompendium ma kategorie Guide, Recommendations, Updates (feed pakietu), Features i Help z informacjami o raportach i zgodzie (`dc3a78120`). |
| T119 | Wheel | Punkty, slots/perks i reset | PARTIAL | Lista slotów/save/reset są; brak diagramu koła i równoważnych podglądów perków. |
| T120 | Wheel | Presets i copy/import code/URL | PARTIAL | Save allocation jest; brak nazwanych presetów i wymiany buildów code/URL. |

## OWNER

| id | obszar | funkcja | status | brak |
|---|---|---|---|---|
| O035 | Mobile | Mobilny joystick ruchu | MISSING | Brakuje ekranowego joysticka dotykowego do sterowania ruchem. |
| O056 | Mobile | Mobilny pasek skrótów | MISSING | Brakuje ekranowego paska mobilnych skrótów dotykowych wybierających tryb attack/look/follow/use. |
