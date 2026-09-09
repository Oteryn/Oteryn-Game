# Oteryn real-content visual slice — continuation prompt

Short invocation:

```text
Oteryn: real-content visual slice
```

Full continuation directive:

```text
Oteryn: real-content visual slice.

Kontynuuj autonomicznie TEN SAM kanoniczny worker Issue #509 na branchu `agent/world-vfx-real-content-509` w `Oteryn/Oteryn-Game`. GitHub LIVE state jest jedynym źródłem prawdy. Najpierw odczytaj #509, protected #480/#489, #502 oraz `experiments/world-vfx-real-content/README.md`; nie twórz replacement Issue/branch/PR, jeśli istniejący worker nadal jest dostępny.

Cel: zastąpić syntetyczne kolorowe placeholdery z `OTERYN WORLD + VFX PROTOTYPE` prawdziwym, lokalnym real-content visual slice opartym o dokładny Tibia 15.32 source i Game-owned normalized appearance/world semantics, bez commitowania proprietary pixels i bez mutacji produkcyjnego klienta/renderera/servera.

Zachowaj custom Rust + `wgpu`, server/gameplay authority, explicit floor/tile/stack ordering, visual coverage/displacement niezależne od gameplay footprint, time-based animations, screen-space overlays oraz bounded visible working-set/cache direction z #480.

Użyj istniejącego Game-owned Thais Z7 fixture producer i normalized 15.32 appearance programs. Lokalny Thais fixture jest już zweryfikowany: 24,311 tiles, 39,282 presentation records, 862 appearances, 990 unique sprite IDs; artifact `sha256:4b340053f72b3522a9fe644c9afdf08d9c7b9b686aec0b57cef764a7cb7dc468`. Exact `15.32.zip` SHA-256: `1a6bad8b7598cd874f534cd4aae2d249fb3d9b4458b3ccfa75754f91bb27870f`. Candidate dense viewport: x=32400..32439, y=32239..32268.

Następnie wykonaj bounded experiment-local decoder `catalog-content.json` + `*.bmp.lzma`, zasil nim real GPU resources i pokaż rzeczywistą scenę Thais zamiast kolorowych prostokątów. Muszą zostać realnie exercised: ground/world objects, animated outfit/creature z direction/timing, co najmniej jeden effect, co najmniej jeden missile, movement interpolation, multi-floor/roof/occlusion, large-sprite overhang, name/HP overlay, day/night/light oraz critical VFX/telegraph. Dodaj camera scroll/zoom/fractional-zoom smoke i bounded decode/cache churn.

Nie buduj drugiego gameplay/appearance schema z `appearances.dat`; Game-owned normalized contracts/programs są semantic authority. Raw 15.32 bytes są tylko physical pixel inputem. Nie zapisuj decoded sprite sheets, proprietary pixels ani screenshots redistributing asset packs do Git.

Pracuj wyłącznie pod `experiments/world-vfx-real-content/**` plus public-safe evidence dla #509. Nie dotykaj `apps/client/**`, `crates/renderer/**`, `apps/game-server/**`, protocol/persistence/CONTENT, root Cargo/workspace, workflows, META, registry, Platform ani Atlas runtime. #502 pozostaje osobnym production resource-bound architecture gate.

Nie kończ na samym działającym oknie. Zakończenie #509 wymaga deterministic bounded sprite decode, object/outfit/effect/missile real-content paths, Game-owned semantic timing/composition, poprawnego ordering/coverage/displacement, zero proprietary bytes w diffie, Molehill-PC/RX 9070 XT smoke bez unexplained crash/device loss, public-safe evidence, exact-head CI/review, META 3.1 native exact-head Merge Queue oraz protected-main readback.
```
