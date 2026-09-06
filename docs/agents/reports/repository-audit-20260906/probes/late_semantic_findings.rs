//! Audit characterization only. No source mutation; assertions preserve observed defects.
#[allow(dead_code, unused_imports)]
#[path = "__PRODUCT__/apps/game-server/src/ai/mod.rs"]
mod ai;
#[allow(dead_code, unused_imports)]
#[path = "__PRODUCT__/apps/game-server/src/ability/mod.rs"]
mod ability;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    use ai::{Candidate, CandidateId, canonicalize_perception};
    let base = [
        Candidate::new(CandidateId::new(1), 30),
        Candidate::new(CandidateId::new(2), 20),
        Candidate::new(CandidateId::new(1), 10),
    ];
    let permutations = [[0,1,2],[0,2,1],[1,0,2],[1,2,0],[2,0,1],[2,1,0]];
    let mut accepted_duplicates = 0;
    for order in permutations {
        let values = [base[order[0]],base[order[1]],base[order[2]]];
        let actual = canonicalize_perception(&values);
        assert!(actual.is_ok(), "characterization changed: inspect before updating audit");
        let perception = actual.map_err(|_| "unexpected rejected characterization")?;
        let ids: Vec<_> = perception.candidates().iter().map(Candidate::id).collect();
        assert_eq!(ids, vec![CandidateId::new(1), CandidateId::new(2), CandidateId::new(1)]);
        accepted_duplicates += 1;
    }
    assert!(canonicalize_perception(&[base[0],base[0]]).is_err());
    assert!(canonicalize_perception(&[base[0],base[1]]).is_ok());
    println!("AUDIT_AI_DUPLICATE: priority-separated duplicate ID accepted in {accepted_duplicates}/6 permutations; adjacent duplicate rejected; unique control accepted");

    use ability::{AbilityEngine,AbilityIntent,AbilityOccurrence,CommitGroup,Effect,EffectPlan,ProposalSource,RevisionSet,TargetId};
    assert!(Effect::damage("target", -7).is_err());
    assert!(Effect::heal("target", 0).is_err());
    let revisions = RevisionSet::new("rules:1","content:1","world:1","formula:1","sim:1")?;
    let occurrence = AbilityOccurrence::new("audit:negative-effect", revisions)?;
    let intent = AbilityIntent::normalize(ProposalSource::Client,"actor", &["target"])?;
    let effect = Effect::Damage { target: TargetId::new("target")?, magnitude: -7 };
    let plan = EffectPlan::new(occurrence,intent,vec![effect],vec![],CommitGroup::atomic("scope:1","group:1")?)?;
    let mut engine = AbilityEngine::new();
    assert!(engine.commit(plan)?.applied());
    assert_eq!(engine.fixture_health("target"),Some(7));
    println!("AUDIT_EFFECT_BYPASS: validated constructor rejects -7; direct public variant passes EffectPlan and commit; Damage(-7) produces fixture health +7");
    Ok(())
}
