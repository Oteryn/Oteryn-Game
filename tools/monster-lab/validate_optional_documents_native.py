#!/usr/bin/env python3
"""Reproduce native Document and Creature-ref validation using cached production serde/validators.

Generates an external driver from the repository's existing native Creature fixture; no
contract source is edited. Cache library commit binding is explicitly not inferred.
"""
import argparse
import hashlib
import json
import subprocess
from pathlib import Path

DRIVER = r'''
fn bind_document(mut input: ProjectV2Draft, document: ProjectV2Declaration, wrong_ref: bool) -> ProjectV2Draft {
    let identity = match &document { ProjectV2Declaration::Document {identity,..} => identity.clone(), _ => panic!("Document required") };
    input.state.declarations.push(document);
    let profile = input.state.authoring_profiles.iter_mut().find(|p| p.target.family == ProjectV2Family::Creature).expect("Creature fixture");
    let mut data = serde_json::to_value(&*profile).expect("profile JSON");
    data["data"]["profile"]["details"]["encyclopedia_document"] = json!({"family":"Document", "key": if wrong_ref { "oteryn:document/missing" } else { &identity.key }, "revision": identity.revision});
    *profile = serde_json::from_value(data).expect("existing native Creature profile serde");
    input
}
fn main() {
    let file = std::env::args().nth(1).expect("native Document declarations file");
    let rows: Vec<Value> = serde_json::from_slice(&std::fs::read(file).expect("input bytes")).expect("JSON");
    let mut passed = 0;
    let mut negatives = 0;
    let mut bound = limits(); bound.max_reference_records = 4096; bound.max_document_bytes = 16777216; bound.max_total_bytes = 33554432; bound.max_decoded_fields = 262144; bound.max_string_bytes = 1048576;
    for row in &rows {
        let document: ProjectV2Declaration = serde_json::from_value(row.clone()).expect("existing Document serde");
        assert!(CanonicalProjectDocuments::from_v2_draft(bind_document(draft(), document.clone(), false), bound).is_ok(), "native positive declaration and Creature ref validation"); passed += 1;
        if passed == 1 {
            assert!(CanonicalProjectDocuments::from_v2_draft(bind_document(draft(), document.clone(), true), bound).is_err(), "missing exact Document ref must fail"); negatives += 1;
            let mut wrong_family = bind_document(draft(), document.clone(), false);
            let profile = wrong_family.state.authoring_profiles.iter_mut().find(|p| p.target.family == ProjectV2Family::Creature).expect("Creature");
            let mut wrong = serde_json::to_value(&*profile).expect("profile"); wrong["data"]["profile"]["details"]["encyclopedia_document"]["family"] = json!("Item");
            *profile = serde_json::from_value(wrong).expect("profile shape");
            assert!(CanonicalProjectDocuments::from_v2_draft(wrong_family, bound).is_err(), "wrong encyclopedia family rejected"); negatives += 1;
            let mut invalid = row.clone(); invalid["content"] = json!(["\0"]);
            let invalid_doc: ProjectV2Declaration = serde_json::from_value(invalid).expect("shape accepted before semantic text check");
            assert!(CanonicalProjectDocuments::from_v2_draft(bind_document(draft(), invalid_doc, false), bound).is_err(), "native semantic text validation rejects NUL"); negatives += 1;
            let mut bad_enum = row.clone(); bad_enum["document_type"] = json!("Encyclopedia");
            assert!(serde_json::from_value::<ProjectV2Declaration>(bad_enum).is_err(), "existing enum fence"); negatives += 1;
        }
    }
    println!("{{\"native_document_creature_ref_cases\":{},\"native_negative_cases\":{},\"public_contract_changes\":false}}", passed, negatives);
}

'''

def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for key in ['repository','documents','server-lib','serde-lib','rustc','output']:
        parser.add_argument('--'+key,type=Path,required=True)
    args = parser.parse_args()
    args.output.mkdir(parents=True,exist_ok=True)
    fixture = args.repository/'apps/game-server/tests/content_world_project_v2_creature_admission.rs'
    # All existing fixture construction helpers precede the first test function.
    prefix = fixture.read_text().split('#[test]',1)[0]
    prefix = '\n'.join(line for line in prefix.splitlines() if not line.startswith('#!') and not line.startswith('//!'))
    source = args.output/'native_document_driver.rs';source.write_text(prefix+'\n'+DRIVER)
    binary = args.output/'native_document_driver'
    command = [str(args.rustc),'--edition=2024','-A','warnings',str(source),'-L','dependency='+str(args.server_lib.parent),'--extern','oteryn_game_server='+str(args.server_lib),'--extern','serde_json='+str(args.serde_lib),'-o',str(binary)]
    with (args.output/'native-document-build.log').open('w') as log:
        subprocess.run(command,stdout=log,stderr=subprocess.STDOUT,check=True)
    result = subprocess.run([str(binary),str(args.documents)],capture_output=True,text=True,check=True)
    counts=json.loads(result.stdout.strip())
    receipt = dict(counts,scope='Native canonical Document declaration and Creature-ref validators via existing public fixture',build_command=command,driver_source_sha256=sha(source),producer_source_sha256=sha(__file__),existing_fixture_sha256=sha(fixture),documents_sha256=sha(args.documents),cached_server_library_sha256=sha(args.server_lib),cached_serde_library_sha256=sha(args.serde_lib),cache_library_commit='SOURCE_COMMIT_BINDING_UNVERIFIED',source_changes_executed=False,live_server_started=False)
    (args.output/'native-document-proof.json').write_text(json.dumps(receipt,indent=2)+'\n')
    print(json.dumps(counts))

if __name__=='__main__':main()
