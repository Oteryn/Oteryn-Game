# Source: Oteryn/Oteryn-Platform deploy/oracle/game.Dockerfile @ 5bd0d08221ea76f245d60719111e983417487784
# Build with an immutable Oteryn-Game source archive as the context.
FROM rust:1.94.0-bookworm@sha256:365468470075493dc4583f47387001854321c5a8583ea9604b297e67f01c5a4f AS build
RUN apt-get update \
    && apt-get install -y --no-install-recommends cmake pkg-config \
    && rm -rf /var/lib/apt/lists/*
ARG GAME_SOURCE_SHA
RUN test "${#GAME_SOURCE_SHA}" -eq 40
ENV OTERYN_BUILD_SHA=${GAME_SOURCE_SHA}
WORKDIR /src
COPY . .
RUN cargo build --locked --release --jobs 2 -p oteryn-game-server --bins

FROM build AS world-bundle-build
# Archive inputs have no Git metadata. Index only the compiler's documented
# inputs; pin-check verifies their blob identities against the committed pin.
# No synthetic commit or history is created.
RUN git init \
    && git add --force -- content/world content/houses content/creatures/definitions \
        content/items/definitions content/npcs/definitions content/dialogues/definitions \
        content/services tools/world-bundle-compiler crates/world-bundle \
        Cargo.toml Cargo.lock rust-toolchain.toml \
    && if [ -d .cargo ]; then git add --force -- .cargo; fi \
    && cargo build --locked --release --jobs 2 -p oteryn-world-bundle-compiler \
    && target/release/oteryn-world-bundle-compiler pin-check . /world-bundle

FROM scratch AS world-bundle
COPY --from=world-bundle-build /world-bundle/ /

# The node's pinned inputs (deploy/oracle/README.md, "Baked inputs"): the World bundle
# pin-check just proved, the gameplay manifest closure (stage_gameplay.py) and the NPC
# catalogue (stage_npc_data.py), each checked against its pin. INPUTS.sha256 lists every file.
FROM build AS node-inputs
COPY --from=world-bundle-build /world-bundle/ /world-bundle/
RUN apt-get update \
    && apt-get install -y --no-install-recommends python3 \
    && rm -rf /var/lib/apt/lists/*
RUN set -eu \
    && set -- /world-bundle/*.otwb \
    && [ "$#" -eq 1 ] && [ -f "$1" ] \
    && mkdir -p /node-inputs \
    && cp "$1" /node-inputs/world.otbundle \
    && python3 deploy/synology-game/stage_gameplay.py content/spells.manifest.json . /node-inputs/gameplay \
    && python3 deploy/synology-game/stage_npc_data.py deploy/synology-game/npc-data.pin.json . /node-inputs/npc-data \
    && cd /node-inputs \
    && find . -type f -print0 > /inputs.list \
    && LC_ALL=C sort -z /inputs.list > /inputs.sorted \
    && xargs -0 sha256sum < /inputs.sorted > /INPUTS.sha256 \
    && rm /inputs.list /inputs.sorted \
    && mv /INPUTS.sha256 INPUTS.sha256

FROM debian:bookworm-slim@sha256:7c7b2c966bc9ee8cedfeef67e0e279108992c77681fa595db4a9d65c06ccc587 AS runtime
ARG GAME_SOURCE_SHA
LABEL org.opencontainers.image.source="https://github.com/Oteryn/Oteryn-Game" \
    org.opencontainers.image.revision="${GAME_SOURCE_SHA}"
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=build /src/target/release/oteryn-game-server /usr/local/bin/oteryn-game-server
COPY --from=build /src/target/release/oteryn-game-ops /usr/local/bin/oteryn-game-ops
COPY --from=build /src/target/release/oteryn-game-migrate /usr/local/bin/oteryn-game-migrate
COPY --from=node-inputs /node-inputs/ /opt/oteryn-game/
ENV OTERYN_NATIVE_GAMEPLAY_MANIFEST=/opt/oteryn-game/gameplay/content/spells.manifest.json
USER 1001:1001
ENTRYPOINT ["/usr/local/bin/oteryn-game-server"]
CMD ["serve", "--config", "/etc/oteryn/node.toml"]
