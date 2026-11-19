# Oracle Game node smoke test (NON-PRODUCTION)

Internal smoke test of the Game node image 6aa81f41 at 172.18.0.11, using a **test CA** and no
public route. Nothing here is production PKI; production follows ADR-0024 (#3327). No private
key is in this branch.

## 1. Test gameplay CA and listener certificate (on the Oracle host, as root)

```sh
umask 077; d=/srv/oteryn/game/test-ca; mkdir -p $d && cd $d
openssl ecparam -name prime256v1 -genkey -noout -out test-root.key
openssl req -x509 -new -key test-root.key -sha256 -days 30 -subj "/CN=Oteryn TEST Gameplay Root (non-production)" \
  -addext "basicConstraints=critical,CA:TRUE,pathlen:0" -addext "keyUsage=critical,keyCertSign,cRLSign" -out test-root.crt
openssl ecparam -name prime256v1 -genkey -noout -out gameplay.key
openssl req -new -key gameplay.key -subj "/CN=game.oteryn.com" -out gameplay.csr
printf 'basicConstraints=critical,CA:FALSE\nkeyUsage=critical,digitalSignature\nextendedKeyUsage=serverAuth\nsubjectAltName=DNS:game.oteryn.com\n' > leaf.ext
openssl x509 -req -in gameplay.csr -CA test-root.crt -CAkey test-root.key -CAcreateserial -days 7 -sha256 -extfile leaf.ext -out gameplay.crt
install -o 1001 -m 0600 gameplay.crt gameplay.key /srv/oteryn/game/node/secrets/
```

The client side of the test uses `OTERYN_DEV_ROOT=test-root.crt` (dev build). `test-root.key` stays
in `test-ca/` (root, 0600) and is deleted after the smoke test.

## 2. node.toml

See `node.toml` beside this file. It is derived from `deploy/synology-game/node.toml.template` with:

- `[readiness] required_gameplay_capabilities = []` added. **The template on main lacks this
  required field, so a node built from the template fails to parse its config.**
- The optional Platform producers omitted (account_characters, character_commands, bazaar,
  public_projections).
- `[platform.world_event_outcomes] enabled = false` set. The section defaults to enabled with
  `/etc/oteryn/credentials/t94-client.*` when absent.

Placeholders to fill: `<DB_IP>`, `<GAME_COMMIT_12HEX>` (`6aa81f41…` first 12 hex),
`<UNIX_SECONDS_AT_SETUP>`. `route_revision` must equal the Registry value once a route exists. For
a no-route smoke test, any `rt.1.<hex>` value works, but admission rejects grants until they match.

## 3. Database

Use the TLS server name and CA from Platform #1981. `db-ca.pem` is the CA that signed the PostgreSQL
server certificate. The runtime login is a member of `oteryn_game_runtime` only.

## 4. mTLS to Platform

- Endpoint: Platform's internal mTLS listener (`[platform] endpoint`, today `172.18.0.2:8543`)
  with `peer_name` equal to the name in Platform's internal server certificate.
- `trust_roots_file`: Platform's **internal CA** certificate. Platform supplies it.
- Client certificates: one per purpose, never shared. The node host generates each key and
  sends a CSR. Platform's internal CA signs it, and Platform registers the subject for that
  purpose:
  - `[platform] client_certificate_file`: native admission evidence (required);
  - `[platform.runtime_status]`: the node host identity (subject = `NODE_IDENTITY` in
    `scope.env` / `report.toml`), with the scopes it may serve;
  - further producers only when enabled (projection, character commands, bazaar, T94,
    public projections), each with its own subject.
- Platform needs from Game: each CSR, the purpose, and the World/Channel scope.
  Game needs from Platform: the internal CA, the endpoint, peer_name, and the signed certificates.
