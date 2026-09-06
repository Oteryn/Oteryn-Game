//! Audit-only source-identical adapter/router composition; no product API widening.
#![allow(dead_code, unused_imports)]
#[path = "../../../../../crates/input-platform/src/error.rs"] mod platform_error;
pub use platform_error::InputPlatformError;
#[path = "../../../../../crates/input-platform/src/adapter.rs"] mod adapter;
use adapter::{InputPlatformAdapter,PlatformEvent,PlatformButtonState};
use oteryn_input_actions::*;

fn router(modifiers:Modifiers)->Result<InputRouter,InputError> {
    let context=ContextId::new("audit.global".to_owned())?;
    Ok(InputRouter::new(BindingMap::new(vec![ContextDefinition::new(context.clone(),ContextKind::Global,0)],vec![Binding::new(context,InputChord::new(modifiers,vec![InputAtom::Key(KeyCode::KEY_C)])?,ActionId::new("audit.copy".to_owned())?,RepeatPolicy::Allow)],&[])?))
}
fn emit(adapter:&mut InputPlatformAdapter,router:&mut InputRouter,event:PlatformEvent<'_>)->Result<Vec<ActionEvent>,InputPlatformError> {
    let normalized=adapter.process_platform_event(event)?;
    Ok(normalized.iter().flat_map(|event|router.process(event)).collect())
}
fn key(code:u16,state:PlatformButtonState,repeat:bool)->PlatformEvent<'static> {
    PlatformEvent::Key {code:Some(code),state,repeat,text:None,synthetic:false}
}
fn main()->Result<(),Box<dyn std::error::Error>> {
    let cases=[(224,Modifier::Control),(225,Modifier::Shift),(226,Modifier::Alt),(227,Modifier::Super),(228,Modifier::Control),(229,Modifier::Shift),(230,Modifier::Alt),(231,Modifier::Super)];
    for (code,modifier) in cases {
        let modifiers=Modifiers::one(modifier);
        let mut direct=router(modifiers)?;
        let positive=direct.process(&NormalizedInputEvent::Key {code:KeyCode::KEY_C,state:ButtonState::Pressed,modifiers,repeat:false});
        assert_eq!(positive.len(),1);assert_eq!(positive[0].phase(),ActionPhase::Started);
        let mut adapter=InputPlatformAdapter::new();let mut routed=router(modifiers)?;
        assert!(emit(&mut adapter,&mut routed,key(code,PlatformButtonState::Pressed,false))?.is_empty());
        let actual=emit(&mut adapter,&mut routed,key(6,PlatformButtonState::Pressed,false))?;
        assert!(actual.is_empty(),"characterization changed: physical modifier chord now works");
        println!("INPUT_MODIFIER_CHARACTERIZATION: modifier_code={code} direct_normalized_starts=1 adapter_then_router_starts=0");
    }
    let mut adapter=InputPlatformAdapter::new();let mut routed=router(Modifiers::NONE)?;
    assert_eq!(emit(&mut adapter,&mut routed,key(6,PlatformButtonState::Pressed,false))?[0].phase(),ActionPhase::Started);
    println!("INPUT_PLAIN_KEY_CONTROL: adapter_then_router_starts=1");
    let mut adapter=InputPlatformAdapter::new();let mut routed=router(Modifiers::one(Modifier::Control))?;
    emit(&mut adapter,&mut routed,PlatformEvent::Modifiers {bits:Modifiers::one(Modifier::Control).bits()})?;
    assert_eq!(emit(&mut adapter,&mut routed,key(6,PlatformButtonState::Pressed,false))?[0].phase(),ActionPhase::Started);
    let release=emit(&mut adapter,&mut routed,key(224,PlatformButtonState::Released,false))?;
    assert!(release.is_empty());
    assert_eq!(emit(&mut adapter,&mut routed,key(6,PlatformButtonState::Pressed,true))?[0].phase(),ActionPhase::Repeated);
    println!("INPUT_MODIFIER_REPEAT_CHARACTERIZATION: reported-control start; physical-control release emits no terminal event; C repeat still repeats prior modifier-qualified action");
    Ok(())
}
