# Przygotowane dialogi partii 5

Te pliki **nie są ładowane przez obecny runtime** ani zarejestrowane jako druga tożsamość questa. Zawierają pełne literalne odpowiedzi i wybrane własne reguły donora dla osobnego PR koordynatora. Ich status to `DATA_PREPARED_RUNTIME_LEFT_OUT`.

Wiążące rozstrzygnięcie: [ANSWER6589906637](https://github.com/Oteryn/Oteryn-Game/pull/2846#issuecomment-6589906637); pytanie: [F5-COMMON-ADAPTER](https://github.com/Oteryn/Oteryn-Game/pull/2846#issuecomment-6589549529). Zmiany wspólnego adaptera, routingu i NpcTalkBook opt-in nie należą do tej partii.

Pole `source_dispatch_constraints` jest częścią wymaganej semantyki źródła, nie komentarzem pozwalającym pominąć wcześniejszy KeywordHandler. Statyczne `KeywordNode` korzystają z Lua string.find oraz pairs; dynamiczne MsgContains mają inną regułę. Nie wolno aktywować samych match_mode/unless bez kwalifikacji całej kolejności. Niektóre przygotowane podzbiory używają lower_exact i celowo pozostawiają złożone wiadomości poza zakresem. Dane zawierają descriptor keyword_handler_substring dla statycznych gałęzi, a nie wymyśloną kolejność pairs.

Źródła tej różnicy: `zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data/npclib/npc_system/npc_handler.lua:446-455`, `data/npclib/npc_system/keyword_handler.lua:42-73,178-189` oraz `data/npclib/npc.lua:13-20` na tym samym pinie. Każda wypowiedź i zmiana stanu ma osobne źródła w JSON.

Żaden plik nie autoryzuje podziału atomowej wymiany, zmiany obcego questa, pominięcia powitania ani zmiany zapisanych zer. Inventory/gold/reward/appearance/time/map-mark i inne niewspierane pełne gałęzie są rozliczone w evidence per quest.
