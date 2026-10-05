# Event components: Source data conversion

The closed assignment contains 75 donor Lua files, not 75 independent quests.
`builder.py` uses existing verified corpus bytes and Lua AST captures; it never
fetches sources, executes Lua, or edits canonical Quest/NPC/runtime data.

Run:

```sh
python builder.py --assignment ../events-assignment.json \
  --corpus-manifest /path/to/corpus-manifest.json \
  --ast-root /path/to/ast \
  --out events.json.gz
python -m unittest discover -s . -p 'test_*.py'
```

The corpus manifest supports relative content-addressed paths. The AST root can
contain `index.json` plus `captures/`, or `index.json.gz` and the portable tar.gz
shards. Assigned capture bytes must match both compressed and decoded digests
from the index. Source raw bytes must match the assignment, corpus SHA256,
Git blob SHA1, byte length, and captured raw bytes.

Every grammar node is materialized into ordered Source IR. Branch tests and
alternative branches, loop bounds and bodies, assignments and tables, returns,
callback parameters, nested and named helpers, and unknown expressions retain
all original AST fields. Ref pointers address this materialized tree (AST field
shape is preserved). Each node has a Source reference; 9,473 nodes with existing
parser spans carry exact byte offsets and hashes. Nodes whose upstream parser
has no span remain explicit with a null span. Reversing the conversion reproduces
every original AST exactly for all 75 records.

The operation labels describe the donor syntax, not an approved runtime binding.
An apparent object method remains `dispatch_binding=NOT_PROVEN`. Event scheduling
methods receive config labels only with an event-constructor receiver witness;
`os.time()` therefore is not interpreted as event time configuration. Eight
quoted registerEvent expressions are donor/revision-scoped lexical caller
candidates, not execution or quest-owner proof. They are joined onto the matching
component records and retained at packet level.

Results: 75 materialized Source components; 89 callbacks; 43 named helpers;
510 conditional/loop/return nodes; 573 source operation annotations; 988 calls
without an operation annotation. All function bodies, including anonymous
callbacks, remain present. Nine focused tests and full 75-record AST/byte-span
roundtrip qualification pass. Zero records are certified fully semantically
bound. Remaining work is Source global/helper and dispatch binding, runtime
activation and approved Oteryn admission, rather than missing source files.

`qualify.py` is lane-local evidence generation and has explicit local cache paths;
Root should integrate the portable builder, schema, tests, packet and qualification
receipt, rather than this lane-local qualification script.
