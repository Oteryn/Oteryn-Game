# Tibia → Oteryn: 720 skontrolowanych wątków — audyt nieukończony

Stan 12.09.2026: **720 unikalnych ID** w zachowanych rejestrach. Ostatnia partia wnosi 12 wątków i 23 posty ponad [punkt kontrolny 708](./synteza-708.md). Łącznie nowsze 120 rekordów ma metadane 241 postów. To wpisy otwierające i wybrane początkowe odpowiedzi, nie pełne dyskusje; rekord usuniętej treści pozostaje jawnie oznaczony.

**Last Year: spis 103/103 stron, 3061 ID; 628 z zachowanym odczytem/oceną, 2433 do opracowania.** Ogólnych 720 nie należy odejmować od 3061: 92 rekordy nie należą do obecnego spisu. Lista jest ruchomym snapshotem, nie atomowym eksportem. [Postęp i kolejka](./last-year-progress.json) oraz [indeks źródeł](./source-index.json) są podstawą wznowienia.

Przeczytaj główną tabelę powtarzających się potrzeb w [syntezie 708](./synteza-708.md), a poniżej jej uzupełnienie. Porównanie [#571](https://github.com/Oteryn/Oteryn-Game/pull/571) potwierdzono ponownie na rewizji `b2ef322791520130a2683f3004d26abd5c027ac8`. Jest to propozycja dokumentacyjna, nie wdrożenie.

| Problem gracza | Powtarzalność | Global / status dowodu | Odpowiednik #571 | Lepszy Oteryn, klasa i ryzyka |
|---|---|---|---|---|
| Trudno porównać rzeczywisty wpływ buildu | [4998105](https://www.tibia.com/forum/?action=thread&threadid=4998105): 1 OP z jawnymi przykładowymi liczbami, nie pomiarem. | Brak niezależnego testu dostępnych narzędzi. | Dokładne §4.6.4–5 Build Impact & Comparison / Training Arena & Combat Simulation. | Q0 zakresy/założenia porównania dostępnych danych; parametry ukryte i nowy dostęp do testów wymagają osobnej polityki. R: zgodność wersji/formuł; G: bez zmiany mocy; E: lepsza decyzja inwestycyjna, nie gwarancja wyniku hunta. |
| Trening przerywa się bez obecności gracza | [4998104](https://www.tibia.com/forum/?action=thread&threadid=4998104) jest wcześniejszym OP; 4998106 to kopia tego samego autora. **2 ID, 1 tekst**. | Czas charge, logout i przewidywane przychody są relacją/założeniem OP. | §4.8.9 Offline Exercise Sessions: ten sam rate, koszt, skuteczność i eligibility, jawna współbieżność finansowana zasobami każdej postaci. | G2 automatyczne przedłużanie dostępnego treningu; kandydat offline zamiast wymogu stale otwartego klienta. R: jedna sesja/postać; G: eventy, skill i czas; E: zużycie zasobów. Płatne ładunki nie dowodzą neutralności względem obecnej progresji. |
| Zewnętrzne rezerwacje utrudniają spontaniczny hunt | Nowe [4997963](https://www.tibia.com/forum/?action=thread&threadid=4997963) i [4998089](https://www.tibia.com/forum/?action=thread&threadid=4998089) wzmacniają potrzebę dostępu, ale proponują różne rozwiązania. | Zarzuty blokad i tezy o retencji niezweryfikowane. | §4.2.2–3 availability, §4.2.7 bounded dynamic spawn, P3 Party Finder. | Q0/Q1 dostępny Party Finder bez bonusów. G1/G2 nowe dane o pozycji i reguły dostępu; G2 mnożniki respawnu, loot i damage. R: zgody/granice wiedzy; G: konkurencja i boosting; E: globalny budżet nagród. „0,5%” nie jest placebo. |

Nowa [tabela 12 źródeł](./year-seventh12-analysis.md) obejmuje też touchscreen, szybsze usuwanie paralysis, punkty Wheel za skille, proficiency boosts, role Monka i Etcher. Deklaracja autora „bez wpływu na balans” nie zastępuje oceny kosztu akcji, zasięgu, czasu, mocy i podaży.

**Korekta wcześniejszego snapshotu:** po znalezieniu starszego OP `4998104` późniejszy `4998106` wyłączono z recurrence. Liczba ID nie spadła. [Indeks jakości](./year-quality-index.json) obejmuje teraz 84 nowe rekordy; wcześniejsze liczniki w syntezie 708 opisują moment przed tą korektą. Pełne posty i profile nie są publikowane.

**Pozostaje:** 2433 OP z bieżącego spisu, dalsze strony odpowiedzi, globalna deduplikacja oraz weryfikacja istotnych twierdzeń o Global. Nie działa harmonogram ani kolektor w tle.
