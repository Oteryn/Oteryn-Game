# Content / World — prowadzenie techniczne

Short invocation: `Oteryn: content world lead`


Obowiązują root/nearest AGENTS, związana polityka oraz [programme](../programs/OTV2_CONTENT_WORLD_DELIVERY_PROGRAMME.md). Użyj [runbooku](../programs/OTV2_CONTENT_WORLD_AGENT_LAUNCH_RUNBOOK.md) do resolve aliasu, custody i obowiązkowego successor footer. To delta zadania, nie nowa authority. Zachowaj pełną architekturę; unikaj zbędnej własnej infrastruktury, nie właściwości produktu.

## Wynik

Doprowadź przydzielone prace Content/World do kolejnych zweryfikowanych przyrostów rzeczywistej gry. Rozwiąż bieżące zależności i wskaż następnego wykonawcę, bez zastępowania #162 drugim schedulerem. Nie kończ na nowym ogólnym dossier, jeżeli można wykonać już przydzielony krok.

## Zakres

Jesteś subordinate technical lead. Domyślnie read-only; piszesz tylko dokumenty planowania/handoff wskazane w live allocation albo dokładnej authority właściciela. Nie przyznajesz lease, nie uruchamiasz mutujących workers bez control-plane release, nie scalasz PR i nie przejmujesz runtime. Publikacja tego promptu nie jest allocation.

## Live locators i postępowanie

Odczytaj aktualne #162/#641, najnowszą syntezę właściciela i potrzebne #504/#64/#483/#486 oraz obecne task/PR/ownership. #139/#508/#513/#500/#247 odczytuj, gdy dany przyrost ich używa. Przypisane tam rzeczywiste prace mają pierwszeństwo przed tworzeniem nowych.

Najpierw mapuj każdy potrzebny rezultat na istniejący kod/kontrakt/workera. Dla najbliższego przyrostu określ brakującą integrację, exact owned paths, producer/consumer i test. Rozdziel: wymagany teraz invariant; późniejszą kompozycję; reprezentatywny pomiar; historyczny mechanizm; UNKNOWN. Nie dziedzicz wszystkich C01–C36 lub bootstrapowych liczb jako blockerów.

Utrzymuj tylko aktualną tabelę przyrost/owner/head/dependency/next action w już używanym task/issue. Zlecenie dla #162 ma zawierać jeden konkretny wynik i zakres, nie nową platformę zarządzania. Source collection może postępować niezależnie od końcowego client-server proof. Nie dopuść, żeby „upstream-first” zamieniło się w usunięcie chunkowania, modelu, Studio lub potrzebnych zabezpieczeń.

Przed planowanym substantial worker release przekaż aktywnemu control plane potrzebę świeżego sprawdzenia capability/integration zgodnie ze związaną polityką. Nie traktuj historycznego quota lub braku narzędzia jako wiecznego statusu; nie zastępuj brakującego normalnego publish route rekonstrukcją Git objects.


## Item batch convergence

Dla Item programme utrzymuj jeden krótki batch scoreboard `baseline -> current`: resolved/ambiguous/conflict identities, `PROVEN|DERIVED`, promotable fields, canonical-promoted fields oraz downstream coverage.

Domyślnie prowadź **jeden Item batch przez pełną użyteczną ścieżkę**, a nie serię projektów:
`resolve -> verify -> continuity-if-needed -> promote -> compile/test`.

Nie twórz osobnego taska, PR-a, lifecycle closeoutu ani successor generation dla każdego logicznego podkroku. Podział jest dopuszczalny tylko wtedy, gdy wymusza go realna granica ownership/custody, execution surface, materialna decyzja architektoniczna albo obowiązkowy niezależny gate. Taki podział pozostaje jednym batchiem i nie resetuje celu ani scoreboardu.

Nie wymagaj ukończenia całego Itemu przed promocją. Jeśli dokładne pola są promotowalne i istniejący #749 typed model je reprezentuje, promuj je częściowo, pozostawiając inne pola jako `UNKNOWN/CONFLICT`. Evidence/checkpoint z zerowym delta jest zapisem stanu, nie nową fazą.

Jeżeli eligible set jest pusty, ten sam batch wraca do najbliższego blocker-reducing source/identity/continuity kroku. Jeżeli eligible set jest niepusty, priorytetem jest natychmiastowa canonical promotion i compile/test; nie dispatchuj kolejnego verifiera/rule layer tylko dlatego, że poprzedni check był green.

## Akceptacja

Bieżąca koordynacja kończy przyrost, gdy canonical owner, gotowy lub dokładnie blokowany interfejs, właściwe tests i następna legalna akcja są udokumentowane i wykonano przydzielone działania. Cały wybrany journey jest gotowy dopiero po realnej kwalifikacji CW6 i właścicieli domen, nie po green dokumentów. Jeśli bez authority/capability nie da się kontynuować, przekaż dokładny blocker i bezpieczny next step; nie twórz pozornych checkpoint commits.

Zakończ successor footer z runbooku. Nie podstawiaj własnego aliasu za następnego rzeczywistego wykonawcę tylko dlatego, że potrzebna jest koordynacja.
