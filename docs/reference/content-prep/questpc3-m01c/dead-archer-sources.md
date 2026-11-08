# Dead Archer: checked source facts for M01c

- TibiaWiki BR page 4042, revision 329894, redirects Dead Archer Quest to Thais Quest. URL: https://www.tibiawiki.com.br/index.php?curid=4042&oldid=329894 ; body SHA256 `e7895865f80b3c29f2a38869ce5a1b5bc91ea3f19942b440e170149425b9d129`.
- Thais Quest page 53187, revision 433964, section “Missão 04: Dead Archer”, lines 124–128: use a shovel to open the entrance at Tibia coordinates `(32493,32259,7)`, then descend east through the caves and cross the poison-field passage. URL: https://www.tibiawiki.com.br/index.php?curid=53187&oldid=433964 ; body SHA256 `baa2bccd9eb87e55245e538350a84a44db3c2696bb567ff5117d3d8dd25a5b06`.
- The accepted donor reward is independently mapped at `(32514,32303,10)` in both servers (UID 6169): `tools/content-schema/reward-claim-authoring/samples/container-source/input.json:16945` at commit `c6fca36fc0d932f47b0fcae896da0838da1214c7`; file SHA256 `ba71bc2e619f46e2b45d25643c2f8380d39f925e3574411b4d3a3769ccac1776`.

Decision: the wiki supplies the entrance coordinate only. Preserve the accepted donor reward payload (including five Poison Arrows), rather than replacing it with the current combined wiki page's seven arrows or its broader Thais Quest requirements. No production route is qualified by these facts alone. Bind the actual native placement and successful tool receipt; arbitrary digging must not qualify.
