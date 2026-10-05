# Portable thin Source map qualification

Integrate only this directory's JSON/schema/C++/Python/Markdown files. Do not
copy source-blobs or node-indexes into the product repository. The qualification
binds 51 map streams, 117 donor/member references, 81,495,134 nodes, the 20 map
archive members, and 12 exact missing-map acquisition receipts. Scope is lossless
structural capture; interpreted attributes, semantic1:1 and runtime remain false.

Paths in structural-receipt.json and inputs.normalized.json are relative to the
reference-cache layout, not a workstation. Git paths/revisions are upstream
identities. The compiled executable is a temporary test artifact, not integrated.

Run (working directory and checkout path can be arbitrary):

```
python verify.py --cache-root /path/to/placements-reference-cache
python verify.py --blob-root /path/to/source-blobs --index-root /path/to/node-indexes
python verify.py --manifest /path/to/qualification.json --cache-root /path/to/cache
python verify.py --structural-only
```

The first command expects source-blobs/<SHA256> and
complete-source-maps/node-indexes/<SHA256>.nodes.gz beneath cache-root. The explicit
directory options support caches reorganized independently. No network is used.
Without cache paths Source-backed verification fails. --structural-only reports
STRUCTURAL_ONLY / NOT_VERIFIED and cannot imply verified donor cache bytes.

Source-backed verification checks all thin input bindings, the closed receipt
schema and exact donor pins; every original artifact SHA/Git blob ID; every decoded
map stream SHA/length; every full compressed index SHA, gzip CRC and 24-byte record
count; archive/member bytes and Git IDs; and the exact Canary release hash.
It reuses recorded extraction/provenance receipts. It does not freshly query Git,
re-extract archives, interpret item attributes, or certify native execution.

Node index records are six little-endian uint32s:
node_id,parent_id,start,end_exclusive,payload_start,payload_end_exclusive.
Offsets refer to the exact decompressed source stream. IDs are preorder; record
emission is close order; root parent0xffffffff. Payload includes escaped node TYPE
and all properties before children; descendants/order/unknown fields are retained.
The four-byte file identifier stays in the bound decoded source. C++ decoder uses
existing zlib and rejects malformed structures and compressed-stream failures.

```
PYTHONDONTWRITEBYTECODE=1 python test_nodes.py
PYTHONDONTWRITEBYTECODE=1 python test_portable.py
```

11 structural regressions independently reconstruct fixture bytes, retain deep
containers/unknown attrs/order, and reject truncation/root/escape/CRC faults.
Portable regressions test relocated/explicit caches, required cache proof,
missing/corrupt bytes, unsafe paths, wrong pins/counts/index bindings and promotions.
Tests compile the ~60-line C++ decoder into a temporary directory (g++ and zlib).
Python qualification requires jsonschema, already used by repository tools.

307 unresolved map-metadata dependency references remain separate in the original
reference package; they are not missingQuest counts. Current canonical geometry
and native behavior remain separate from this donor structural capture.
