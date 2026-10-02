Zamknięta propozycja Name15 i BR3 — stan opublikowany b0f91e2883007861f5556c44cce13701bb447283

15 korekt nazw i 3 przypisania `quest_item` mają dowody źródłowe. Niczego nie zastosowano. Pozostałe 217 rekordów Family235 zachowuje swoje dotychczasowe blokady. Weryfikację wykonano offline na dokładnych Git blobs, oficjalnym pliku 15.30 i już zachowanych publicznych odpowiedziach; bez nowych zapytań, zmian repo i Cargo.

| Zamknięta kohorta | ID | Zakres | Warunek przed zastosowaniem |
|---|---:|---|---|
| Name15 | 15 | Tylko `ReferenceItemSemantics.presentation.name` | Osobna zatwierdzona kwalifikacja źródła dla dokładnego importowanego literału `weapon of mayhem` |
| Rodziny po Name15 | 15 | Navigation metadata: 9 `weapon_melee`, 6 `weapon_magic` | Faktyczna publikacja korekty i ponowienie accepted name-join wraz z dowodem pełnych stron/gałęzi disambiguation |
| BR3 | 3 | Navigation metadata: `quest_item` | Osobny zamknięty auxiliary raw-BR bridge; dotychczasowy resolver nie obsługuje tej drogi |

Istniejąca authority nazwy i konieczna decyzja

Oryginalny Name139 już zapisuje oficjalne własne pole nazwy rekordu appearance do istniejącego native leaf. Jego reguła jest jednak zamknięta:

> ReferenceItemSemantics.presentation.name; only exact fullTarget IDs in closed139 packet with each proven prior KNOWN imported literal (120 weapon of carving, 19 event item) and absent GameOwned presentation authoring; incoming already equal accepted idempotently. All other Known names and blocked states reject.

Dokładne źródło tej reguły to `docs/agents/evidence/OTV2-20261002-item-name-source-qualification-v1.json`, SHA5650ab936c222a3771dec95c7bd058c3d398883f14b6ecb7e982e10f2592482c; compiler `witness_params` dodatkowo wymaga niepustego własnego singleton WikiID. Rust `item_name_promotion.rs` dopuszcza tylko dwa powyższe wcześniejsze literały. Żaden z nowych 15 targetów nie ma własnego ID w pełnym selected own-ID index. Nie wolno przypisać im ID ze strony podstawowego wariantu ani rozszerzać starego zapieczętowanego pakietu139.

Wniosek: istnieje właściwy typ/owner dla nowej wartości. Nie trzeba nowej grupy native ani zmiany schema. Potrzebna jest osobna, jawna i zamknięta **kwalifikacja authority korekty**: bieżący oficjalny własny numeric object/name + dokładny importowany literal, zamiast nieistniejącego własnego WikiID. Jest to nowa wyjątkiem ograniczona polityka źródła, a nie czynność już dopuszczona przez stary moduł139. Wcześniejszy NAV6 potwierdza użycie A12 numeric official identity/name w nawigacji; nie daje samodzielnie prawa do nadpisywania KNOWN name. Niniejsza propozycja wymaga niezależnej recenzji i zatwierdzenia tej zamkniętej authority przed implementacją.

Własny dowód Name15

ID:23577,23583,23589,23596,23605,23609,23619,23624,23638,23641,23644,23656,23659,23662,23665.

Oficjalny plik SHA2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2 ma 5,017,996 bajtów. Ponownie przeliczono cały manifest:43516 własnych rekordów object,43516 unikalnych numeric IDs, zgodne wszystkie identity-projection i raw-record digests. Pozostałe protobuf families są odrębne:1480 outfits,243 effects,76 missiles i1 version. Dla każdego wybranego ID występuje dokładnie jedno własne ID i jedno własne pole nazwy4, z poprawnym UTF-8. Nazwy wartościowo powtarzają się między wariantami — nie stanowi to konfliktu numeric identity i nie uprawnia do rozszerzania kohorty na pozostałe warianty.

Dla każdego targetu sprawdzono pełne `family/key/revision`, exact Crystal forward/reverse binding (`ots/item_server_id`, pinned ff7ede/00ce02), bieżące cztery nagłówki (`shard.schema`, `shard.family`, `definition.kind`, `definition.identity.family`), KNOWN `weapon of mayhem`, brak GameOwned presentation oraz brak właściciela World zarówno w rzeczywistym b0f, jak i opublikowanym World overlay3ed. `flags.take` musi być jawnie TRUE; brak lub brak dodatkowych flag nie jest promowany do FALSE. Wszelkie dodatnie World/unmove flags blokują.

Chroniony Crystal XML SHA c847293e980b40ec146e2b7f68a62366513a1c0566d16b7c3a011136087021eb zawiera dokładnie jeden źródłowy element `fromid=23577 toid=23667 name="weapon of mayhem"`. Zachowano jego **dosłowne** bajty w source proof, zamiast przedstawiać serializację ElementTree jako oryginał. Każdy z15 ma dokładnie zgodny wpis historycznej promocji `presentation.name` i obecny alias do pełnego targetu. XML obejmuje91 IDs; nowa korekta dotyczy wyłącznie15, bez rozszerzania na cały zakres.

Proponowane wpięcie Name15

Osobne `lower_item_official_name15_packet.py`, source proof i promotion packet; minimalny osobny Rust `item_official_name15_promotion.rs` korzystający z istniejącego modelu name. Zachować wszystkie bajty Name139 proof/packet/compiler/Rust. Compiler może bezpośrednio używać istniejących wąskich helperów `base.checked`, `base.exact_bindings`, `movable.leaf`, `movable.own_index`; nie wywoływać `Name139.load_inputs`, bo ta funkcja uczciwie wymaga własnych WikiID. Nie dodawać konfigurowalnego uniwersalnego nadpisywania nazw.

Pakiet ma dokładnie15 pełnych targetów i wcześniejszy literal `weapon of mayhem`. Wprowadza TEXT official name. Przed zmianą całej kohorty ponownie zweryfikować source pins/current parent, numeric object tag1, complete membership, protected XML/imported proof/alias, full target i reverse binding, World/Take oraz GameOwned presentation. Rust najpierw waliduje **cały** pakiet: dokładny zestaw targetów i unikalność, identity/r1, wcześniejszy literal, owner/fulltarget i name-state. Tylko KNOWN oldliteral lub identyczny KNOWN incoming są dopuszczone. UNKNOWN/CONFLICT/NOT_APPLICABLE, inne KNOWN oraz pojedynczy późny brak targetu odrzucają cały pakiet przed pierwszą mutacją. Zachować description i wszystkie pozostałe grupy, stack/class/admission, owners, relations i protected importer literals. Idempotence nie wymaga historycznego full-shard digest po wprowadzeniu poprawnej nazwy; rozdzielić dowód baseline od bieżących relewantnych guardów.

Znaczące testy: dwa numeric IDs z identyczną nową nazwą; niewłaściwy source ID lub namespace; błędny fullTarget revision; inny prior imported literal/alias/XML range; zduplikowane własne pole nazwy w raw object; GameOwned presentation; BLOCKED/Unknown albo unrelated KNOWN name; duplicate/late missing target i atomowość; powtórzenie pakietu=0 zmian. Offline wykonano już8 negatywnych source/state guardów i1 pozytywny idempotent case; **nie są to wykonane testy przyszłego Rust modułu**.

Nawigacja po Name15

Zachowane13 stron bazowych mają dokładne `actualname` i jawne admitted primarytype, lecz inne własne item IDs. Proof zachowuje ich rewizje, raw boxes, duplicate/empty params i accepted resolver wyniki. To dowód nazw/rodzin metadata, **nie authority numeric identity ani scalar values wariantów**. Potrzeba prawdziwego replay dokładnej kolejności accepted capture helper: own-ID priority przed name fallback; unresolved own-ID blokuje fallback; title→appearance-title→actualname, all-present agreement wewnątrz wybranej próby. `resolve_name` sprawdza `DISAMBIG_RE` w pełnym article. Compact raw box nie dowodzi braku dodatkowej gałęzi disambiguation w artykule. W tym proposal `whole_article_disambiguation_branch_proven=false`; przed finalnym snapshot/carrier wymagany pełny artykuł właściwej rewizji oraz komplet candidate-title/redirect odpowiedzi, albo już zatwierdzony retained helper output o równoważnej kompletności. Nie tworzyć fake observation ze starego stats snapshot. Zachować availability `unavailable` z bazowych stron i nie przenosić duration/charges/stats do wariantów.

Zamknięty auxiliary BR3 bridge

ID52745/52785/52789 mają własne singleton Fandom itemid, primary Others, pickupable=yes, blank objectclass i brak jawnego konkurencyjnego secondary/status. Cały selected own-ID set został porównany z rzeczywistym pełnym index. Bieżąca native KNOWN name dokładnie odpowiada oficjalnej nazwie; każda z3 nazw jest też globalnie unikalna w43516 current object records. Take/EXACT/fulltarget/native/World/GameOwned guards spełnione. Stary flat stats snapshot nie ma obserwacji dla tych3 — zachować ten fakt, nie dopisywać fałszywej historycznej observation.

Fandom pełne raw odpowiedzi i BR pełne raw odpowiedzi istnieją. Zapisano rzeczywiste full articles, body hashes, page/rev/cutoff, nie zrekonstruowaną treść. BR strony66272/rev432005,66276/rev431353,66274/rev433335 mają dokładnie nazwę oficjalnego obiektu i jawne `primarytype=Itens de Quest`. Ta wartość jest istniejącą kategorią `quest_item` w bieżącym profile-catalog. BR **nie ma numeric itemid**; jest tylko auxiliary name/category source. Fandom Others nie daje konkurencyjnej admitted rodziny, lecz nadal musi być zachowane jako unresolved źródło. Wzmiankowany4h buff jest efektem aktora i nie jest Item duration.

Wpiąć osobny zamknięty source frame/helper obok istniejącego `item_external_family_refinement.py`, bez zmiany jego18 records, parsera Tibiopedia DOM, globalnego resolvera engine28644 ani sourcecatalog825. Raw BR `Infobox_Item` nie jest Fandom `Infobox Object`: potrzebuje właściwego bounded parsera z balance/comments/duplicate i presence guardami, pełnymi coord/hash/cutoff i zamkniętym zestawem3 source pages. Profile fallback jest jawnie DERIVED/NAVIGATION_ONLY. Przed finalną integracją musi przejść obecny build_taxonomy/exact binding/World/source-priority/name/profile guard na rzeczywistym composed predecessor. Nie zmienia native classification, runtime admission, schemas, actor effects, stacks ani Item facts.

Testy BR3: borrowed ownFandomID lub drugi/opposed własny frame; duplicated/blank/wrong BR primarytype; niezgodna caption/name albo nieunikalna official name; zła rewizja/hash/cutoff; explicit admitted competing family; dodatni World owner/flag; rollback/atomic frozen-cohort preservation i idempotence. Zachować opublikowane12252 family rows w odrębnej taxonomy overlay oraz217 holds. To liczba overlay, nie bieżącej composed Native gałęzi, która jeszcze nie zawiera pełnej integracji taxonomy. Dalszy wzrost family assignment należy policzyć dopiero po rzeczywistym importerze, a nie deklarować z liczby kandydatów.

Pliki z propozycją

- `family-name15-br3-closed-source-proposal-20261002.json`: pełne target/current guards, raw źródła, własne/obce ID, legacy literal,197 wejściowych pinów,217 zachowanych held IDs.
- `family-name15-br3-proposed-packet-20261002.json`: jawnie niezaakceptowany i niewykonywalny projekt pakietów15+3.
- `prepare-family-name15-br3-closed-proposal.py`: deterministyczny offline generator proof, bez repo writes.

`FullyVerified=NOT_ESTABLISHED`; opublikowanych/applied zmian z tego proposal:0. Uzupełnienie18 metadata przypadków nie kończy całej puli Item ani nie usuwa217 nadal otwartych source/domain/identity blokad.
