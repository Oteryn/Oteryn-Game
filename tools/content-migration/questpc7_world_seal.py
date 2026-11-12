"""Reseal staged authoring documents before mandatory native WorldProject completion."""
import hashlib
import json
from pathlib import Path


def refresh(root, package_root=None):
    base=Path(package_root) if package_root is not None else root/'content/world';paths=['manifest.json','content.lock.json','project.json']
    original=[(base/p).read_bytes() for p in paths];manifest,lock,project=map(json.loads,original)
    sha=lambda raw:hashlib.sha256(raw).hexdigest()
    def provenance(digest):
        values=[manifest[k] for k in ['package_key','package_revision','semantic_schema_version','licensing_metadata']]+[digest]
        framed=b''.join(len(v.encode()).to_bytes(4,'big')+v.encode() for v in values)
        return sha(framed)
    entry,=[e for e in lock['entries'] if e['package_key']==manifest['package_key'] and e['package_revision']==manifest['package_revision']]
    assert project['manifest_sha256']==sha(original[0]) and project['content_lock_sha256']==sha(original[1])
    assert entry['package_provenance_digest']==provenance(sha(original[0]))
    for document in manifest['documents']:
        path=(base/document['locator']).resolve();assert path.is_relative_to(base.resolve())
        raw=path.read_bytes();document.update(byte_length=len(raw),sha256=sha(raw))
    encode=lambda doc:(json.dumps(doc,ensure_ascii=False,separators=(',',':'))+'\n').encode()
    manifest_raw=encode(manifest);entry['package_provenance_digest']=provenance(sha(manifest_raw))
    lock_raw=encode(lock);project.update(manifest_sha256=sha(manifest_raw),content_lock_sha256=sha(lock_raw))
    for path,raw in zip(paths,[manifest_raw,lock_raw,encode(project)]):
        target=base/path
        if target.read_bytes()!=raw:target.write_bytes(raw)
