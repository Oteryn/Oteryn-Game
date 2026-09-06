//! Audit-only characterization of source-local models, not product fixes.
//! Compile with rustc --edition=2024 from this repository path; no Cargo changes.
#![allow(dead_code, unused_imports)]
#[path = "../../../../../apps/game-server/src/ai/mod.rs"] mod ai;
#[path = "../../../../../apps/game-server/src/ability/mod.rs"] mod ability;
#[path = "../../../../../apps/game-server/src/content/digest.rs"] mod digest;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    use ai::{Candidate, CandidateId};
    let id=CandidateId::new(9);
    let records=[Candidate::new(id,3),Candidate::new(CandidateId::new(10),2),Candidate::new(id,1)];
    let permutations=[[0,1,2],[0,2,1],[1,0,2],[1,2,0],[2,0,1],[2,1,0]];
    for permutation in permutations {
        let input=permutation.map(|index| records[index]);
        let actual=ai::canonicalize_perception(&input).map_err(|e|format!("unexpected rejection: {e:?}"))?;
        assert_eq!(actual.candidates().iter().filter(|candidate|candidate.id()==id).count(),2);
    }
    assert!(ai::canonicalize_perception(&[records[0],records[2]]).is_err());
    assert!(ai::canonicalize_perception(&[Candidate::new(CandidateId::new(1),3),Candidate::new(CandidateId::new(2),2)]).is_ok());
    println!("AI_DUPLICATE_CHARACTERIZATION: six permutations accepted separated duplicate IDs; adjacent duplicate rejected; unique control accepted");

    use ability::{AbilityEngine, AbilityIntent, AbilityOccurrence, CommitGroup, Effect, EffectPlan, ProposalSource, RevisionSet, TargetId};
    assert!(Effect::damage("target:a",-1).is_err());
    assert!(Effect::heal("target:a",-1).is_err());
    for (ordinal,damage,magnitude,expected) in [(1,true,-1,1),(2,false,-1,-1),(3,true,0,0)] {
        let effect=if damage {Effect::Damage {target:TargetId::new("target:a")?,magnitude}} else {Effect::Heal {target:TargetId::new("target:a")?,magnitude}};
        let plan=EffectPlan::new(AbilityOccurrence::new(&format!("occurrence:{ordinal}"),RevisionSet::new("r:1","c:1","w:1","f:1","s:1")?)?,AbilityIntent::normalize(ProposalSource::Client,"actor:a",&["target:a"] )?,vec![effect],vec![],CommitGroup::atomic("scope:a","group:a")?)?;
        let mut engine=AbilityEngine::new();assert!(engine.commit(plan)?.applied());assert_eq!(engine.fixture_health("target:a"),Some(expected));
    }
    println!("ABILITY_MAGNITUDE_CHARACTERIZATION: constructor negatives rejected; raw Damage(-1), Heal(-1), Damage(0) accepted by plan and committed; fixture-only, not a network exploit");
    for size in [0_usize,1,3,55,56,63,64,65,127,128,129,1024,1048576] {
        let data=(0..size).map(|n|(n%251) as u8).collect::<Vec<_>>();
        println!("SHA256_VECTOR size={size} digest={}",digest::sha256(&data).iter().map(|b|format!("{b:02x}")).collect::<String>());
    }
    Ok(())
}
