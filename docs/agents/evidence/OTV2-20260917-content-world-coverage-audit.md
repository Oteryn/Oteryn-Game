# Content / World — zachowany audyt pokrycia PR #641

```yaml
status: RETAINED_ANALYSIS
classification: PROPOSED_NONCANONICAL
date: 2026-09-17
repository: Oteryn/Oteryn-Game
pr: 641
inspected_main: b44fefe08f6aaf1b2c1c23dedd92bab0de87146e
original_audit_head: 0682d66d5f526d8119a8e8d5ab215ff120be0f59
implementation_authority: NONE
production_authority: NONE
worker_release: NONE
```

## 1. Pochodzenie, korekta właściciela i zakres dowodu

Ten dokument przenosi merytoryczne ustalenia, 36 obszarów i ich scenariusze z załączonego do rozmowy raportu `PR641_macierz_pokrycia.md` oraz pakietu `PR641_audyt_pokrycia.zip`. Jest redakcyjną syntezą, nie identyczną kopią wszystkich powtarzających się kart, surowych logów i plików ZIP.

- Oryginalny Markdown SHA-256: `6534c6adcb89fdbf33924c64d1a940fac552c21c74cde25e400feeda289bd6ef`.
- Oryginalny ZIP SHA-256: `50cf58b2c89419dd04449d240d5f2b728cda3009ec079694d2d4ac5a2848806f`.
- Kod podstawowego modelu zachowano już w [local-transition candidate revision 2](OTV2-20260917-content-world-local-transition-contract-candidate.md). Nie tworzymy drugiej implementacji.
- [Ostateczna synteza i plan wdrożenia/importu](OTV2-20260917-content-world-owner-synthesis-and-implementation-plan.md) utrwalają późniejszą dyspozycję właściciela.

**Właściciel nie nakazał redukcji architektury ani zastąpienia gry protezą. Nakazał ograniczenie niepotrzebnych customowych bibliotek, forków i infrastruktury.** Etapowe wdrażanie wykorzystuje tę samą docelową architekturę. Dawna odpowiedź asystenta okrajająca kierunek jest odrzucona i nie stanowi specyfikacji.

36 aspektów to inwentarz coverage, nie procent ukończenia Tibii i nie 36 nowych blockerów przed pierwszą operacją. LOCAL/WIRE/IMPORT/VALUE/FEATURE/REFERENCE porządkują dowody, nie przydzielają nowych uprawnień. Właściwy zakres wynika z wybranej funkcji i obecnych kontraktów/#162. `UNKNOWN` w audycie nie dowodzi nieobecności kodu w całym repozytorium.

Odczyt dokumentu to nie wykonanie silnika. Dane OTS są źródłem hipotez, pól i przypadków testowych, nie dowodem Global parity. Historyczne statusy CI i dostępności stron nie są bieżącą authority.

## 2. Wyniki faktycznie odtworzone przy publikacji

Wyodrębniony z załącznika model ma dokładnie SHA-256 podany w PR:

`e1e6cb4713b84586df2293205daa9b7667b52f26c3f8adfff80c1d93e49aadaa`.

Osobne uruchomienie na Pythonie 3.13.5: **21 testów PASS**, w tym **4096 długości-cztery sekwencji zdarzeń**. Dodatkowo cztery celowo uszkodzone warianty zostały odrzucone przez testy: usunięcie cudzych wkładów kolizji, pominięcie current authority, użycie wyniku komendy jako stanu świata i pominięcie bazowej rewizji delty.

| Próba diagnostyczna | Zaobserwowany wynik | Dokładne ograniczenie dowodu |
|---|---|---|
| B1: druga sesja wobec tego samego Owner | `FENCED`; drzwi pozostają otwarte | Jeden Context nie reprezentuje dwóch równocześnie dopuszczonych sesji. Nie jest to dowód błędu produkcyjnego multiplayer. |
| B2: command 2 przy pending command 1 | `COMMITTED`, następnie terminalizacja `false` | Oddzielne Owner i IngressWitness nie dowodzą ordered-commit composition. Revision 2 już to ujawnia. |
| B3: zawartość Frame | Brak scope/content/incarnation/owner_generation | Model nie sprawdza tych powiązań. Realny widok może wiązać je poza pojedynczą ramką; nie narzucamy wszystkich pól każdemu pakietowi. |
| B4: pełny receipt store bez eviction | Następna komenda `NOT_ADMITTED` | Konserwatywne bezpieczeństwo fixture nie dowodzi retencji i dalszego postępu rzeczywistego konsumenta. |

Nie uruchomiono w tej publikacji poprzedniego modelu 26-testowego ani historycznego prototypu 63-testowego. Nie sumujemy ich z 21. Nie uruchomiono Oteryn Rust, PostgreSQL, rzeczywistego importera/korpusu, natywnego klienta, współbieżnej produkcji ani obserwacji na Globalu.

## 3. Ustalenia audytu i ich znaczenie

**F1 — kompozycja runtime, EVIDENCE_GAP.** Potrzebny jest prawdziwy consumer istniejącego Foundation ingress i obecnego runtime ownera. Dwie poprawne sesje, cała GameSession i różne cele muszą zostać sprawdzone w realnej kompozycji. To nie nakaz nowego per-door journal ani dużego globalnego subsystemu kolejkującego. C11–C16.

**F2 — obserwacja, EVIDENCE_GAP.** CommandResult nie nadpisuje świata; delta i snapshot wymagają aktualnego widoku oraz rzeczywistego egress. Samo istnienie SnapshotBarrier nie dowodzi użycia. Inspected registry ma puste `command_types` i `state_domains`; realny wire child musi je zarejestrować, nie używać opaque payload lub cudzego ID. C17–C19.

**F3 — końcowa wartość źródłowa, EVIDENCE_GAP.** Historycznie odczytany Crystal `Items::{reload,loadFromProtobuf,loadFromXml}` ładuje appearances przed XML core/data. `canReadText` jest przypisane dwukrotnie. Lista nazw pól nie opisuje finalnego wyniku; trzeba zachować warstwy/nadpisania i jawnie rozstrzygać mapping. To fakt o wskazanym kodzie, nie ocena intencji ani dowód Globala. C01/C03/C32.

**F4 — brak cichej utraty danych, EVIDENCE_GAP.** W historycznie odczytanym Canary `Container::unserializeItemNode` wybrane błędy tworzenia/deserializacji dziecka prowadzą do `continue`. Oteryn musi rozliczyć pominięty rekord, a nie raportować kompletny sukces. Atlas-facing visual export świadomie nie zawiera pełnych nested contents. C04/C06.

**F5 — community evidence, CONFLICT/UNKNOWN.** W poprzednim sprawdzeniu indeksowany fragment Imbuing podawał dla Intricate 60 000, a Intricate Imbuements 25 000, 70% i dodatkowe 30 000. Nie potwierdzono jednoczesnych rewizji stron ani tego, która informacja odpowiada targetowi. Nie są to wartości dopuszczone do importu Reference. Bez oldid i ciągłości targetu pozostają historycznym konfliktem materiałów, nie nową regułą. C02/C25.

**F6 — Weapon Proficiency, ograniczenie dowodu.** Poprzedni raport wskazuje komunikat CipSoft z 13.07.2026 dotyczący zastępowania dwóch slotów drzewa. Taki komunikat nie definiuje pełnego storage, authority i wszystkich efektów. W tej publikacji ponowny odczyt strony zakończył się błędem narzędzia; zachowujemy pierwotny lokator, nie twierdzimy, że świeżo potwierdziliśmy treść. C26 dopiero przy użyciu tej funkcji.

**F7 — ponowne użycie istniejących fundamentów, PROVEN źródłowo.** GAME-ITEM-01 już obejmuje equip patterns, containment, temporal modes, deterministic modifiers i kompatybilność. Dossier obejmuje rodziny Content. Wiele luk dotyczy konkretnych bindings i wykonania, a nie potrzeby projektowania od nowa item model, timer subsystem lub transaction framework.

Powyższe są lukami/konfliktami przy dokładnie nazwanych następnych funkcjach. Nie są nowymi produkcyjnymi podatnościami tego dokumentacyjnego PR ani podstawą do obciążenia wszystkich etapów pełnym inwentarzem.

## 4. Macierz C01–C36: zakres, źródła, brak i test

Wszystkie T01–T36 poniżej są scenariuszami kwalifikacji dla wskazanych triggerów. Nie oznaczają wykonanych testów Rust lub ukończonych funkcji. D/L/E/I/F/P/R/M/A/T/C/K/W/O rozwinięto w sekcji 5.

| ID / obszar | Obecna podstawa i luka | Kiedy / źródła | Test i oczekiwany wynik |
|---|---|---|---|
| C01 — cały input i kolejność nakładania | Projekt źródeł istnieje; Crystal ma appearances/core/data. Nie uruchomiono gameplay importera; potrzebny kompletny lock i disposition nadpisań. | IMPORT wybranego korpusu; D/E/C | T01: to samo ID ma różne wartości w appearances/core/data; wynik jest ustalony albo jawny konflikt, z pełnym provenance. |
| C02 — wartość i data targetu | #483 określa target, brak nowych obserwacji; community konflikt nie został rozstrzygnięty. Potrzebne value/unit/revision/locator/continuity. | REFERENCE konkretnego pola; D/T/W2/W3/O | T02: późniejsza lub odmienna strona nie nadpisuje zaakceptowanej wartości; konflikt blokuje ćwiczony zakres, nie cały katalog. |
| C03 — parser, typy i liczby | Wymagania są w dossier, permanentny codec nie jest wybrany. Brak testu realnego parsera w tym audycie. | IMPORT rzeczywistego source codec; D/E | T03: duplicate key, unknown critical field, nadmierna głębokość, max/max+1, overflow i trailing bytes są wykrywane. |
| C04 — nested records | Canary może pomijać błędne dziecko; visual export nie jest pełnym gameplay IR. Potrzebne rozliczenie records/fields. | IMPORT kontenerów; D/E/K/A | T04: poprawne dzieci plus jedno błędne kończą się jawnym raportem/odrzuceniem zależnego podzbioru, nigdy fałszywym pełnym sukcesem. |
| C05 — placement i reimport | Model three-way jest opisany; realne legacy identity mapping nie zostało wykonane. | IMPORT/reimport; D/E | T05: rename/move pliku zachowuje ID, copy ma nowe, upstream delete przeciw local edit daje konflikt. |
| C06 — selected closure i unresolved | A zachowuje unresolved appearance 2141 dla przypiętej pary; to wcześniejszy wynik, nie nowy full-world run. | IMPORT/promotion; D/E/A | T06: missing appearance/cell potrzebne w wydaniu blokuje dopuszczenie zależnej funkcji; niezwiązany future quest nie blokuje wydania. |
| C07 — stack i Browse Field | D opisuje rodziny, K/W1 osobny widok i wybór. Brak realnego testu klienta/importera. | FEATURE wyboru wielu elementów/Browse Field; D/K/W1/I | T07: obiekt pod innym obiektem jest wybierany właściwą regułą; zmiana stosu nie używa starego indeksu; browse nie nadaje custody. |
| C08 — współrzędne i piętra | A ma `floor=-z` tylko dla przypiętego inputu; konkretny transform/geometry muszą być kwalifikowane. | IMPORT współrzędnych/przejść; D/M/A | T08: negative coordinates, half-open bounds i cross-shard footprint są poprawne; scope handoff nie udaje local teleport. |
| C09 — wkłady kolizji | L/M rozdzielają współczynniki przestrzenne, model ma test; realnego runtime nie wykonano. | LOCAL spatial composition; L/M/C | T09: otwarcie A nie usuwa blokady B; rebuild i aktualizacja dają zgodny wynik. |
| C10 — definicja i binding | L opisuje finite states/edges i linker; fixture nie jest produkcyjnym source parserem. | LOCAL/source-to-owner; L/D | T10: wrong reference family, missing state, duplicate placement, ambiguous edge i unsupported capability są odrzucane. |
| C11 — scope i inkarnacja | Obowiązuje Channel/Instance ownership; jeden Context nie dowodzi izolacji całego świata. | LOCAL composition; L/M/F | T11: zmiana w kanale A nie zmienia B; stary handle po recycle nie trafia w nowy obiekt. |
| C12 — dwie poprawne sesje | B1 wykazał jeden Context; multi-session proof nie istnieje w tym modelu. | LOCAL multiplayer, bez nowego systemu sesji; L/F | T12: A/1 open, B/1 close, replay A/1 nie otwiera; reconnect A nie zmienia fence B. |
| C13 — cała GameSession | F/P wymagają commit order; Owner może wykonać efekt przed sprawdzeniem retirement. | LOCAL przed authoritative effects; L/F/P | T13: pending komenda do A zatrzymuje późniejszy efekt/result tej samej sesji do B; inna sesja może postępować. |
| C14 — retencja i dalszy postęp | F ma high-water mark/duplicate dispositions, model no-eviction nie dowodzi kompozycji. | LOCAL realny consumer wyników; L/F/P | T14: wygasły payload nie reaktywuje starego CommandRef; nowa komenda przy dostępnej pojemności może być dopuszczona. |
| C15 — stan/przestrzeń/outcome | Model zakłada indivisible publication; nie jest proof prawdziwej atomowości. | LOCAL owner commit; L | T15: injected failure przed publikacją nie zostawia połowy stanu; lost response nie powoduje drugiego commitu. |
| C16 — close i ruch | Model sprawdza dwa serialized orders przy synthetic reject-occupied-close. Global reguła pozostaje nieudowodniona. | LOCAL Movement integration; L/M | T16: ruch pierwszy odrzuca close, close pierwszy odrzuca ruch, bez half-footprint i TOCTOU dla tej fixture policy. |
| C17 — wynik kontra stan | L/P rozdzielają result/delta; nie wykonano rzeczywistego wire klienta. | WIRE; L/P | T17: historyczny sukces A po nowszym close B nie przywraca starej prezentacji. |
| C18 — registry i semantic equality | R ma puste gameplay registries na inspected main; typed equality nie jest raw protobuf equality. | WIRE przed gameplay exchange; L/R/P | T18: nieznany typ odrzucony; różne legalne bytes tego samego typed intent nie tworzą konfliktu; zmieniona semantyka nie staje się nową operacją. |
| C19 — snapshot i związanie widoku | Barrier istnieje, ale model nie pokazuje egress; Frame nie ma wszystkich bindings. | WIRE real producer/client; L/P | T19: result+delta ponad target czekają na SnapshotCommit; stary scope/view nie jest akceptowany nawet przy takiej samej liczbowej connection generation. Nie wymuszać wszystkich pól w każdej ramce. |
| C20 — lifetime/activation | Kandydat odrzuca live content switch, model nie kwalifikuje recovery. | LOCAL stateful activation; L/E | T20: eviction/reimport/disconnect nie resetują stanu; replacement scope wymaga poprawnego fence i dozwolonego resetu. Nie wymaga ogólnego hot reload. |
| C21 — zasoby i tuning | Granice są wymagane, model ma wyłącznie fixture limits. Nie są to production maxima. | Używana amplification/resource boundary; L/E | T21: max/max+1/exhaustion bez częściowej mutacji i nieograniczonego fan-out; tuning na wybranej skali, nie dziedziczenie historycznych liczb. |
| C22 — wyposażenie i containment | I obejmuje equip patterns i graph legality; brak nowego runtime/PG testu. | VALUE equip/move/containment; I/D | T22: pełne two-hand occupancy atomowe; pełna torba/cycle odrzucone; stale session nie przenosi przedmiotu. |
| C23 — stack/charge/transform | I/DUR rozdzielają ilość, ładunki, tożsamość i custody; brak PG qualification tutaj. | VALUE użytych operacji; I/E/D | T23: split/merge/transform i replay odpowiedzi nie duplikują wartości ani nie zerują stanu. Nie wymagać nieużytej rodziny operacji przy pojedynczym transferze. |
| C24 — czas/decay | I już odróżnia deadline/active budget; C pokazuje wearOut/clockExpire/expire/expireStop. Target mapping nie zamknięty. | FEATURE przedmiotu z czasem; I/C/D | T24: equip/deequip, przechowywanie, disconnect/restart i expiry zachowują semantykę przyjętego trybu. |
| C25 — imbuement | D/I opisują slots/modifiers, W podaje niezatwierdzone wyjątki/kombinacje i konflikt kosztów. | FEATURE imbuement; D/I/W2/W3 | T25: niedozwolona kombinacja individually allowed slotów nie mutuje itemu; timer działa według typu; niepewne koszty nie są defaultem. Nie wymyślać asymetrii slotów. |
| C26 — proficiency/perki | D wspomina rodzinę; historyczny O potwierdza kierunek, nie storage/całość reguł. Brak runtime proof. | FEATURE użytej proficiency; D/I/O/T | T26: config/progress postaci nie zmienia wspólnej definicji; efekt po equip/zmianie broni odpowiada zaakceptowanemu przypadkowi. |
| C27 — creatures/AI/spawn | D/E/#504 wskazują stat/combat/reward gap; wybrane wartości i runtime niekwalifikowane tutaj. | FEATURE wybranego spawnu/creature; D/E/T | T27: missing required ref odrzucony; reload nie tworzy dodatkowego live spawn/reward occurrence. |
| C28 — Damage/Heal | D/E wskazują semantic successor; Ability/SIM pozostają ownerami. Nie wykonano tej ścieżki. | FEATURE wybranej zdolności; D/E/T | T28: attack+heal współistnieją, unsupported family jest odrzucona, nowe Content nie reinterpretuje starej operacji. |
| C29 — death/corpse/loot/XP | Selection i settlement są rozdzielone; brak prawdziwego restart/PG evidence w audycie. | VALUE kill/reward; E/D/I | T29: lost reply po commit nie tworzy drugiego mint/XP; jedna śmierć ma stabilne potomne operacje; nie zakładać distributed atomicity. |
| C30 — NPC i usługi | D/E wykorzystują chroniony #500; dialog/widget nie jest realnym settlement ani podróżą. | FEATURE wybranej usługi; D/E/I | T30: stary catalogue price nie autoryzuje trade; otwarcie okna nie przenosi value; travel nie udaje local relocation. |
| C31 — special doors/keys/ACL | D/L odróżniają te klasy; plain synthetic door ich nie kwalifikuje. | FEATURE selected door family; D/L/I | T31: brak prawa/klucza nie daje skutku; coupled value wymaga owning commit; brak fallback do zwykłych drzwi. |
| C32 — read/write/fluid/field/rotate/wrap/hang | D wymienia families, C flagi/nadpisania; brak ich pełnych source-to-owner adapters w PR. | FEATURE użytej capability; D/C/I | T32: use-with wrong target, unknown capability i source overrides są rozliczone; unsupported to explicit exclusion, nie silent no-op. |
| C33 — quest/event/world change | D/E opisują kierunek i lifetime; storage integer nie wystarcza do odtworzenia znaczenia. | FEATURE konkretnego questa/eventu; D/E | T33: reload/channel hop nie resetuje once claim; possess nie znaczy consume; event ma jawny scope/timer/eligibility. |
| C34 — houses/depot/bank/mail/market | I/D odróżniają surfaces/authority; nie wykonano tych usług. | FEATURE dopuszczonej powierzchni; I/D | T34: reference nie nadaje ACL; widok kontenera nie przenosi custody; brak duplikacji wartości między kanałami. |
| C35 — player death/re-entry | T wskazuje wymaganą pętlę; #641 nie jest jej implementacją. Target i wykonanie wymagają osobnego dowodu. | FEATURE physical death/re-entry; T/E/I | T35: terminal session nie wraca przez reconnect; respawn i skutki śmierci według przyjętego przypadku, bez przenoszenia zasad ephemeral drzwi na postać. |
| C36 — projection/assets/rights | D/E/A rozdzielają projekcję i prawa; nie eksportowano realnych assets tutaj. | IMPORT/release klienta; D/E/A | T36: server-only nie trafia do klienta; nierozstrzygnięte prawa/source nie są automatycznie dopuszczone; mixed generations odrzucone, working set ograniczony. |

## 5. Źródła i dokładne kotwice

Kotwice z inspected main i oryginalnego head są niezmienne. Aktualny stan alokacji/CI należy odczytać ponownie; poniższe linki nie są zgodą na zmianę kodu.

| Ref | Źródło | Co może potwierdzić |
|---|---|---|
| D | [Dossier](OTV2-20260917-content-world-design-dossier.md) | Zakres propozycji i wcześniejsze korekty, nie runtime proof. |
| L | [Local-transition candidate](OTV2-20260917-content-world-local-transition-contract-candidate.md) | Revision 2, model i jego jawne założenia. |
| E | [Execution design](OTV2-20260917-content-world-execution-design.md) | Powiązania source/owner, ograniczenia composition i dependency triggers. |
| I | [GAME-ITEM-01](../../architecture/GAME-ITEM-01_ITEM_MODEL_AND_EQUIPMENT_CONTRACT.md) | Właściciele legalności, typy, containment, modifier/temporal/compatibility boundaries. |
| F | [foundation/mod.rs](../../../apps/game-server/src/foundation/mod.rs) | CommandIngress primitives; odczyt źródła nie dowodzi złożenia z obiektem. |
| P | [FND-02](../../architecture/FND-02_PROTOCOL_OTERYN_V1_CONTRACT.md) | Tożsamość, kolejność, semantyczna równość, result/delta/snapshot/barrier. |
| R | [Protocol registry](../../contracts/PROTOCOL_OTERYN_V1_REGISTRY.json) | Puste command_types/state_domains na inspected main. |
| M | [VSL-MOVE](../../architecture/VSL-MOVE-01_MINIMAL_MOVEMENT_VISIBILITY_CONTRACT_CANDIDATE.md) | Position owner, static/dynamic legality, visibility; status acceptance w osobnym chronionym owner acceptance. |
| A | [Game fullworld source README](../../../tools/game-atlas-fullworld-source/README.md) | Exact source scope, floor transform, brak nested children w visual projection i historyczny unresolved 2141. |
| T | [Issue #483](https://github.com/Oteryn/Oteryn-Game/issues/483) | Target `global-tibia-observable-2026-07-28-post-server-save`, official-first, OTS hypothesis only. |
| C | [Crystal items.cpp](https://github.com/zimbadev/crystalserver/blob/ff7ede593c69d4c658b382c97443e8155926924a/src/items/items.cpp) | Historyczny odczyt load order, canReadText i flags. C++ nie był wykonywany. |
| K | [Canary container.cpp](https://github.com/blakinio/canary/blob/12df285bd181ad72eaa23901b0e5b19f37352634/src/items/containers/container.cpp) | Historyczny odczyt Browse Field i unserializeItemNode. C++ nie był wykonywany. |
| W1 | [Browse Field](https://tibia.fandom.com/wiki/Browse_Field) | Community discovery; nie przypięto oldid ani target continuity. |
| W2 | [Imbuing](https://tibia.fandom.com/wiki/Imbuing) | Historyczny indeksowany fragment; oryginalny raport odnotował błąd bezpośredniego odczytu 402. Nie current target truth. |
| W3 | [Intricate Imbuements](https://tibia.fandom.com/wiki/Intricate_Imbuements) | Historyczny fragment sprzeczny z W2; nie ustalono właściwej reguły dla targetu. |
| O | [CipSoft Summer Update 2026](https://www.cipsoft.com/en/440-tibia-summer-update-2026-now-available) | Lokator datowanego komunikatu zachowany w poprzednim raporcie; ponowny odczyt w tej publikacji nie powiódł się. |

Nie zastępujemy kontroli daty targetu aktualną stroną. Nie dopisujemy brakujących stawek, formuł, limitów i polityk na podstawie intuicji. Rozbieżność źródeł nie jest pozwoleniem na arbitrarny wybór wariantu.

## 6. Odtwarzalna diagnostyka modelu

Wyodrębnić blok `LOCAL_TRANSITION_WITNESS_BEGIN` z L do pliku `local_transition_witness.py`; zweryfikować SHA-256 z sekcji 2 i uruchomić oryginalną suite osobno. Następnie poniższy blok można zapisać jako `check_model_boundaries.py` w tym samym izolowanym katalogu. To zachowana diagnostyka synthetic evidence, nie nowy framework testów ani production implementation.

```python
"""Reproduce known limitations of the exact PR #641 design witness.

These are diagnostic assertions about a synthetic model, NOT production tests
or an implementation of missing Oteryn behavior. A passing diagnostic confirms
that the model omits a behavior, not that the corresponding contract is met.
"""
from dataclasses import fields, replace
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile

from local_transition_witness import Owner, Command, IngressWitness, Frame

root = Path(__file__).resolve().parent
source = (root / "local_transition_witness.py").read_text(encoding="utf-8")
expected_sha256 = "e1e6cb4713b84586df2293205daa9b7667b52f26c3f8adfff80c1d93e49aadaa"
assert hashlib.sha256(source.encode()).hexdigest() == expected_sha256

# B1: the Owner fixture has a single Context, not a session-authority registry.
owner = Owner()
assert owner.apply(owner.current, Command()) == "COMMITTED"
context_b = replace(owner.current, session="session-b")
result_b = owner.apply(context_b, Command(session="session-b", desired="closed", expected_revision=1))
assert result_b == "FENCED"
assert owner.state.opened

# B2: calling the isolated owner is not coupled to pending session commit order.
ingress = IngressWitness()
assert ingress.reserve(1) == "RESERVED"
assert ingress.reserve(2) == "RESERVED"
owner = Owner()
result_2 = owner.apply(owner.current, Command(command=2))
retired_2 = ingress.terminal(2)
assert result_2 == "COMMITTED" and owner.state.opened and not retired_2

# B3: no scope/content/incarnation is represented on a model frame.
frame_fields = [f.name for f in fields(Frame)]
unrepresented = [s for s in ("scope", "content", "incarnation", "owner_generation") if s not in frame_fields]
assert len(unrepresented) == 4

# B4: this model remains permanently full; it cannot demonstrate cache eviction
# plus preserved ingress high-water and subsequent acceptance in one composition.
owner = Owner(limit=1)
assert owner.apply(owner.current, Command()) == "COMMITTED"
capacity_result = owner.apply(owner.current, Command(command=2, desired="closed", expected_revision=1))
assert capacity_result == "NOT_ADMITTED"

mutations = {
    "drop_other_spatial_contributions": (
        "return self.static_blockers | own", "return own"),
    "skip_current_authority": (
        "if context != self.current or command.session != context.session:", "if False:"),
    "apply_result_as_state": (
        'if frame.kind == "result":\n            self.sequence = frame.sequence',
        'if frame.kind == "result":\n            self.opened = frame.opened\n            self.sequence = frame.sequence'),
    "skip_delta_base_revision": (
        'frame.kind != "delta" or frame.base != self.revision or frame.revision <= frame.base',
        'frame.kind != "delta" or frame.revision <= frame.base'),
}
mutation_results = []
with tempfile.TemporaryDirectory(prefix="pr641-model-mutations-") as temp:
    for name, (old, new) in mutations.items():
        assert source.count(old) == 1, (name, source.count(old))
        path = Path(temp) / f"{name}.py"
        path.write_text(source.replace(old, new), encoding="utf-8")
        run = subprocess.run([sys.executable, str(path)], text=True, capture_output=True, timeout=10)
        log = run.stdout + run.stderr
        assert run.returncode != 0 and "FAILED (failures=" in log, (name, log)
        (root / f"mutation_{name}.log").write_text(log, encoding="utf-8")
        summary = next(line for line in log.splitlines() if line.startswith("FAILED ("))
        mutation_results.append({"variant": name, "exit_code": run.returncode, "summary": summary})

report = {
    "classification": "SYNTHETIC_MODEL_DIAGNOSTICS_NOT_PRODUCTION_QUALIFICATION",
    "pr_head": "0682d66d5f526d8119a8e8d5ab215ff120be0f59",
    "source_sha256": expected_sha256,
    "python_version": sys.version.split()[0],
    "original_suite": {"tests": 21, "bounded_serialized_traces": 4096, "passed": True},
    "boundary_probes": [
        {"id": "B1", "meaning": "Single Context cannot represent two simultaneously admitted sessions", "second_session_result": result_b, "door_still_open": True},
        {"id": "B2", "meaning": "Owner and IngressWitness are not ordered-commit composition", "later_command_result": result_2, "later_command_retirement": retired_2},
        {"id": "B3", "meaning": "Frame lacks scope/content/incarnation/owner-generation fields", "frame_fields": frame_fields, "unrepresented": unrepresented},
        {"id": "B4", "meaning": "No-eviction receipt fixture cannot qualify bounded eviction and continued progress", "new_command_when_full": capacity_result}
    ],
    "deliberate_model_mutants": mutation_results,
    "not_run": ["Oteryn Rust", "production concurrency", "PostgreSQL", "real importer corpus", "real client/server", "Global observations"],
}
(root / "model_diagnostics.json").write_text(json.dumps(report, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
print(json.dumps(report, indent=2, ensure_ascii=False))
```

Pole `original_suite.passed` w zachowanym skrypcie jest stałą opisową. Samo uruchomienie tego skryptu nie dowodzi przejścia oryginalnej suite: podczas tej publikacji uruchomiono ją oddzielnie i dopiero jej wynik uzasadnia PASS w sekcji 2. Cztery mutanty mają oczekiwany niezerowy exit i odpowiednio 1, 2, 1, 1 failures; to wykrycie celowo popsutego modelu, nie niepowodzenie oryginału.

## 7. Wniosek i następne użycie

Kierunek projektu jest szeroki, ale obecność rodziny w dokumencie nie jest pełną kwalifikacją jej zachowania. Największe konkretne luki dotyczą source-to-capability mapping, rozliczenia strat importu, realnej ordered-commit composition, wielu sesji, registered wire, trwałych transakcji oraz datowanego Reference evidence.

Zachować pełną architekturę. Użyć istniejących Game-owned narzędzi i upstream bibliotek, a własny kod ograniczyć do rzeczywistych reguł i brakujących integracji Oteryn. Następny przyrost ma działać na rzeczywistej ścieżce produktu; szeroki candidate catalogue może powstawać równolegle. Nie uruchamiać wszystkich T01–T36 przed niepowiązaną funkcją i nie usuwać tych zagadnień z planu docelowego.

Ten zapis nie zmienia accepted architecture, production resource registry, protokołu, kodu, branch protection ani allocation. Nie jest independent owner acceptance. Exact-head CI i stan integracji należy odczytać z żywego PR, nie z historycznych wyników załącznika.
