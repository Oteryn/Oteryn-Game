#!/usr/bin/env python3
"""Evidence-only FND-02 retained semantic-record byte-bound candidate."""

from __future__ import annotations

import argparse, hashlib, json
from collections import OrderedDict
from dataclasses import dataclass, replace
from pathlib import Path

U64_MAX=(1<<64)-1
MAX=512
BOUND=4*8+6*2+6*MAX
BOUND_NAME="FIRST_PLAYABLE_RETAINED_SEMANTIC_RECORD_CHARGED_BYTE_BOUND"
MAIN_SHA="f04b75bfe53d3ebcda7d287df98345d1c37c2b8d"
BRANCH="agent/fnd02-retained-semantic-record-byte-bound-evidence-663"
ALLOCATION=5734402048
ARCHITECT=5734335743

class E(ValueError): pass
class TooLarge(E): pass
class Admission(E): pass
class Order(E): pass

def add(a:int,b:int)->int:
    if min(a,b)<0 or max(a,b)>U64_MAX or b>U64_MAX-a: raise OverflowError("u64 overflow")
    return a+b

def check_u64(name:str,v:int)->None:
    if type(v) is not int or not 0<=v<=U64_MAX: raise E(name)

def text(name:str,v:str,namespaced:bool=True)->bytes:
    if type(v) is not str or not v: raise E(name)
    try: raw=v.encode("ascii")
    except UnicodeEncodeError as exc: raise E(name) from exc
    if len(raw)>MAX: raise TooLarge(name)
    if not all(0x21<=b<=0x7e for b in raw): raise E(name)
    if namespaced:
        a,s,b=v.partition(":")
        if not s or not a or not b: raise E(name)
    return raw

def lp(raw:bytes)->bytes:
    return len(raw).to_bytes(2,"big")+raw

@dataclass(frozen=True)
class Intent:
    placement:str
    incarnation:int
    family:str
    expected_revision:int
    def valid(self)->None:
        text("placement",self.placement); check_u64("incarnation",self.incarnation)
        text("family",self.family); check_u64("expected_revision",self.expected_revision)

@dataclass(frozen=True)
class Binding:
    generation:str
    transition_key:str
    def valid(self)->None:
        text("generation",self.generation,False); text("transition_key",self.transition_key)

@dataclass(frozen=True)
class Result:
    outcome:str
    state:str
    revision:int
    def valid(self)->None:
        text("outcome",self.outcome); text("state",self.state); check_u64("revision",self.revision)

@dataclass(frozen=True)
class Record:
    command_id:int
    intent:Intent
    binding:Binding
    result:Result
    def encode(self)->bytes:
        check_u64("command_id",self.command_id)
        if self.command_id==0: raise E("command_id")
        self.intent.valid(); self.binding.valid(); self.result.valid()
        p=b"".join((
            self.command_id.to_bytes(8,"big"),
            lp(text("placement",self.intent.placement)),
            self.intent.incarnation.to_bytes(8,"big"),
            lp(text("family",self.intent.family)),
            self.intent.expected_revision.to_bytes(8,"big"),
            lp(text("generation",self.binding.generation,False)),
            lp(text("transition_key",self.binding.transition_key)),
            lp(text("outcome",self.result.outcome)),
            lp(text("state",self.result.state)),
            self.result.revision.to_bytes(8,"big"),
        ))
        if len(p)>BOUND: raise TooLarge("record")
        return p
    @classmethod
    def decode(cls,p:bytes)->"Record":
        if type(p) is not bytes or len(p)>BOUND: raise E("encoded")
        pos=0
        def take(n:int)->bytes:
            nonlocal pos
            if pos+n>len(p): raise E("truncated")
            out=p[pos:pos+n]; pos+=n; return out
        def u()->int: return int.from_bytes(take(8),"big")
        def s(name:str,ns:bool=True)->str:
            n=int.from_bytes(take(2),"big")
            try: v=take(n).decode("ascii")
            except UnicodeDecodeError as exc: raise E(name) from exc
            text(name,v,ns); return v
        r=cls(u(),Intent(s("placement"),u(),s("family"),u()),Binding(s("generation",False),s("transition_key")),Result(s("outcome"),s("state"),u()))
        if pos!=len(p): raise E("trailing")
        if r.encode()!=p: raise E("noncanonical")
        return r

@dataclass(frozen=True)
class Plan:
    command_id:int
    payload:bytes
    charge:int
    evicted:int|None

class Session:
    def __init__(self,max_charge:int=BOUND)->None:
        check_u64("max_charge",max_charge)
        self.max_charge=max_charge; self.next_id=1
        self.pending:OrderedDict[int,tuple[Intent,Binding]]=OrderedDict()
        self.retained:bytes|None=None; self.mutations=0
    def current(self)->Record|None:
        return None if self.retained is None else Record.decode(self.retained)
    def classify(self,cid:int,i:Intent,b:Binding)->str:
        if cid in self.pending: return "PENDING" if self.pending[cid]==(i,b) else "CONFLICT"
        r=self.current()
        if r and r.command_id==cid: return "REPLAY" if (r.intent,r.binding)==(i,b) else "CONFLICT"
        return "EXPIRED" if cid<self.next_id else "NEW"
    def replay_result(self,cid:int,i:Intent,b:Binding)->Result:
        if self.classify(cid,i,b)!="REPLAY": raise E("not replayable")
        r=self.current()
        if r is None or r.command_id!=cid: raise E("retained result missing")
        return r.result
    def reserve(self,cid:int,i:Intent,b:Binding)->str:
        check_u64("cid",cid); i.valid(); b.valid()
        if cid<self.next_id: return self.classify(cid,i,b)
        if cid!=self.next_id: raise E("gap")
        self.pending[cid]=(i,b); self.next_id=add(self.next_id,1); return "RESERVED"
    def preflight(self,cid:int,r:Result)->Plan:
        if cid not in self.pending: raise E("not pending")
        if cid!=next(iter(self.pending)): raise Order("terminal order")
        i,b=self.pending[cid]; payload=Record(cid,i,b,r).encode(); charge=add(0,len(payload))
        if charge>self.max_charge: raise Admission("retention")
        old=self.current()
        return Plan(cid,payload,charge,None if old is None else old.command_id)
    def finish(self,cid:int,r:Result,mutate=None)->Plan:
        plan=self.preflight(cid,r)
        if mutate: mutate()
        self.mutations+=1; del self.pending[cid]; self.retained=plan.payload
        assert self.retained is plan.payload
        return plan
    @classmethod
    def recover(cls,next_id:int,claimed:bool,payload:bytes|None)->tuple[str,"Session|None"]:
        check_u64("next_id",next_id)
        if next_id==0 or (claimed and payload is None): return "NON_RESUMABLE",None
        s=cls(); s.next_id=next_id
        if payload is not None:
            try: r=Record.decode(payload)
            except E: return "NON_RESUMABLE",None
            if r.command_id>=next_id: return "NON_RESUMABLE",None
            s.retained=payload
        return "RESUMABLE",s

def mkey(ch:str)->str: return "oteryn:"+ch*(MAX-len("oteryn:"))
def max_record()->Record:
    return Record(U64_MAX,Intent(mkey("a"),U64_MAX,mkey("b"),U64_MAX),Binding("g"*MAX,mkey("c")),Result(mkey("d"),mkey("e"),U64_MAX))
def oi(rev:int=0)->Intent: return Intent("oteryn:reference.placement.local-door",1,"oteryn:reference.intent.local-object-open",rev)
def ci()->Intent: return Intent("oteryn:reference.placement.local-door",1,"oteryn:reference.intent.local-object-close",1)
def bi(t:str="oteryn:reference.transition.open",g:str="reference-generation-1")->Binding: return Binding(g,t)
def rs(opened:bool,rev:int)->Result: return Result("oteryn:result.committed","oteryn:reference.state.open" if opened else "oteryn:reference.state.closed",rev)

def raises(exc,fn)->None:
    try: fn()
    except exc: return
    raise AssertionError(exc.__name__)

def tests()->int:
    done=0
    def ok(cond=True):
        nonlocal done
        assert cond; done+=1
    ok(BOUND==3116 and add(32,add(12,3072))==3116)
    mr=max_record(); enc=mr.encode(); ok(len(enc)==BOUND)
    raises(TooLarge,replace(mr,intent=replace(mr.intent,placement="oteryn:"+"x"*506)).encode); ok()
    raises(TooLarge,replace(mr,binding=replace(mr.binding,generation="g"*513)).encode); ok()
    ok(mr.encode()==enc and Record.decode(enc)==mr)
    raises(E,lambda:Record.decode(enc+b"x")); ok()
    s=Session(); i,b=oi(),bi(); original=rs(True,1); s.reserve(1,i,b); s.finish(1,original); before=s.mutations
    ok(s.classify(1,i,b)=="REPLAY" and s.replay_result(1,i,b)==original and s.mutations==before)
    changed=(replace(i,expected_revision=1),replace(i,incarnation=2),replace(i,family="oteryn:reference.intent.local-object-close"),replace(i,placement="oteryn:reference.placement.other-door"))
    ok(all(s.classify(1,x,b)=="CONFLICT" for x in changed))
    ok(s.classify(1,i,replace(b,generation="reference-generation-2"))=="CONFLICT")
    ok(s.classify(1,i,replace(b,transition_key="oteryn:reference.transition.close"))=="CONFLICT")
    s=Session(BOUND-1); s.pending[mr.command_id]=(mr.intent,mr.binding); called=[False]
    raises(Admission,lambda:s.finish(mr.command_id,mr.result,lambda:called.__setitem__(0,True))); ok(not called[0] and s.retained is None and s.mutations==0)
    s=Session(); s.pending[mr.command_id]=(mr.intent,mr.binding); p=s.finish(mr.command_id,mr.result); ok(p.charge==BOUND and s.retained is p.payload)
    s=Session(); i1,b1=oi(),bi(); s.reserve(1,i1,b1); s.finish(1,rs(True,1)); i2,b2=ci(),bi("oteryn:reference.transition.close"); s.reserve(2,i2,b2); p=s.finish(2,rs(False,2)); ok(p.evicted==1 and s.classify(1,i1,b1)=="EXPIRED" and s.reserve(1,i1,b1)=="EXPIRED")
    s=Session(); s.reserve(1,oi(),bi()); s.reserve(2,ci(),bi("oteryn:reference.transition.close")); raises(Order,lambda:s.finish(2,rs(False,2))); ok(tuple(s.pending)==(1,2))
    a,b=Session(),Session(); ai,ab=oi(),bi(); a_result=rs(True,1); a.reserve(1,ai,ab); a.finish(1,a_result); b.reserve(1,ci(),bi("oteryn:reference.transition.close")); b.finish(1,rs(False,2)); before=a.mutations; ok(a.classify(1,ai,ab)=="REPLAY" and a.replay_result(1,ai,ab)==a_result and a.mutations==before and a.current() is not None and b.current() is not None)
    status,recovered=Session.recover(a.next_id,True,a.retained); ok(status=="RESUMABLE" and recovered is not None and recovered.classify(1,ai,ab)=="REPLAY" and recovered.replay_result(1,ai,ab)==a_result)
    missing_status,missing=Session.recover(a.next_id,True,None); corrupt_status,corrupt=Session.recover(a.next_id,True,a.retained+b"x"); ok(missing_status=="NON_RESUMABLE" and missing is None and corrupt_status=="NON_RESUMABLE" and corrupt is None)
    raises(OverflowError,lambda:add(U64_MAX,1)); ok()
    ok(set(Binding.__dataclass_fields__)=={"generation","transition_key"} and "policy_guard_refs" not in Record.__dataclass_fields__)
    return done

def packet()->dict:
    count=tests(); mr=max_record(); enc=mr.encode()
    return {
      "schema_version":1,
      "classification":"NON_PRODUCTION_BOUND_EVIDENCE",
      "admission_main_sha":MAIN_SHA,
      "branch":BRANCH,
      "allocation_comment_id":ALLOCATION,
      "architect_comment_id":ARCHITECT,
      "evidence_result":"BOUND_ESTABLISHED_FOR_ARCHITECT_CONSIDERATION",
      "established_discriminator":{"name":BOUND_NAME,"value":BOUND,"unit":"charged_bytes_per_retained_semantic_record","retained_records_per_gamesession":1,"candidate_aggregate_bytes_per_gamesession":BOUND,"production_value_selected_here":False},
      "protected_inputs":{"production_key_max_bytes":MAX,"production_atom_max_bytes":MAX,"transition_key_wraps_production_key":True},
      "record_candidate":{"normalized_intent":["placement","incarnation","family","expected_revision"],"original_binding":["exact_generation_identity","transition_key"],"terminal_result":["outcome","state","revision"],"retention_metadata":["command_id"],"owned_representation":"one immutable canonical byte buffer","transition_binding_copy":False,"policy_guard_refs_retained":False,"re_resolve_binding_on_replay":False},
      "charge":{"equation":"4*8 + 6*2 + 6*512","fixed_u64_bytes":32,"length_prefix_bytes":12,"max_variable_bytes":3072,"checked_total":BOUND,"owned_retained_copy_count":1,"shared_reference_double_charge":False,"allocator_rss_rust_abi_wire_persistence_claim":False},
      "boundary":{"exact_max_bytes":len(enc),"exact_max_sha256":hashlib.sha256(enc).hexdigest(),"repeat_equal":enc==mr.encode(),"roundtrip_equal":Record.decode(enc)==mr,"key_513":"REJECTED","generation_513":"REJECTED","overflow":"REJECTED","pre_mutation_over_envelope":"REJECTED"},
      "behavior":{"same_intent_binding":"REPLAY","changed_intent_or_binding":"CONFLICT","evicted_terminal":"EXPIRED","evicted_reservable_again":False,"pending_overtake":False,"recovery":"RECONSTRUCT_OR_NON_RESUMABLE"},
      "validation":{"focused_tests":count},
      "scope":{"foundation_runtime":False,"resource_registry":False,"cw3_cw4":False,"protocol":False,"cargo_workspace":False,"production":False},
      "handoff":{"value_ready_for_architect_consideration":BOUND,"production_authority":False,"cw4_resume_authority":False}
    }

def jtext()->str: return json.dumps(packet(),indent=2,sort_keys=True)+"\n"
def mtext()->str:
    p=packet(); h=p["boundary"]
    return f"""# FND-02 retained semantic-record charged-byte bound evidence

Status: NON-PRODUCTION EVIDENCE / BOUND ESTABLISHED FOR ARCHITECT CONSIDERATION

FACT: protected #663 evidence fixes one retained terminal record per GameSession, original
binding identity as exact active Content-generation identity plus TransitionKey, and protected
512-byte ceilings for ProductionKey and ProductionAtom. TransitionKey wraps ProductionKey.

DERIVED: this child uses one immutable owned canonical byte buffer. It contains normalized
intent (placement, incarnation, family, expected revision), the original exact-generation plus
TransitionKey binding reference, the original terminal semantic result (outcome, state,
revision), and command_id retention metadata. It retains no complete TransitionBinding or
policy_guard_refs and never re-resolves the binding against current/new Content on replay.

{BOUND_NAME} = 4*8 + 6*2 + 6*512 = {BOUND} charged bytes.

With the already accepted count 1/GameSession, the candidate aggregate for the exact
first-playable slice is also {BOUND} bytes/GameSession.

Boundary evidence:
- exact maximum encoded candidate: {h["exact_max_bytes"]} bytes
- exact maximum SHA-256: {h["exact_max_sha256"]}
- deterministic repeat: {h["repeat_equal"]}
- decode round trip: {h["roundtrip_equal"]}
- 513-byte key: rejected
- 513-byte generation candidate: rejected
- checked u64 overflow: rejected
- over-envelope retention: rejected before modeled gameplay mutation

Replay/recovery evidence:
- same normalized intent plus exact-generation/TransitionKey replays original outcome
- changed normalized intent, generation, or TransitionKey conflicts
- singleton terminal eviction makes the old outcome expired, never fresh/reservable
- later terminalization cannot pass an earlier pending CommandId
- A/1 open -> B/1 close -> replay A/1 retains one record in each GameSession
- recovery reconstructs the same record or the old GameSession is NON_RESUMABLE

The generation field reuses only the protected 512-byte ProductionAtom ceiling as an evidence
candidate. This does not freeze a production ContentGenerationRef, Rust ABI, allocator/RSS
accounting, wire/persistence/Content format, CW4 store, deployment topology, TTL, or future
general replay window.

No Foundation runtime, resource-registry, CW3/CW4, protocol, Cargo/workspace, persistence,
external-repository, or production surface is changed.

Handoff: route #663 to Oteryn: sol supervising architect to decide/freeze any production
resource value and authority. Do not resume CW4 from this evidence child alone.
"""

def write(path:Path,text_value:str)->None:
    path.parent.mkdir(parents=True,exist_ok=True); path.write_text(text_value,encoding="utf-8",newline="\n")
def check(path:Path,text_value:str)->None:
    if path.read_text(encoding="utf-8")!=text_value: raise SystemExit("deterministic mismatch: "+str(path))
    print("PASS",path)

def main()->int:
    ap=argparse.ArgumentParser()
    ap.add_argument("--self-test",action="store_true")
    ap.add_argument("--write-json",type=Path); ap.add_argument("--write-md",type=Path)
    ap.add_argument("--check-json",type=Path); ap.add_argument("--check-md",type=Path)
    a=ap.parse_args()
    if a.self_test: print("PASS",tests(),"tests")
    if a.write_json: write(a.write_json,jtext())
    if a.write_md: write(a.write_md,mtext())
    if a.check_json: check(a.check_json,jtext())
    if a.check_md: check(a.check_md,mtext())
    if not any(vars(a).values()): print(jtext(),end="")
    return 0
if __name__=="__main__": raise SystemExit(main())
