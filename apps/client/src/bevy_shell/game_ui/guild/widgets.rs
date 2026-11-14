use super::*;
use bevy::{input_focus::tab_navigation::{TabGroup,TabIndex}, ui::InteractionDisabled, ui_widgets::{ScrollArea,TextInput}};
use oteryn_client_ui_bevy as source;

pub(super) fn build(commands:&mut Commands,skin:&Skins,state:&State) -> Entity {
    let root=commands.spawn((Name::new("guild.window"),super::super::GameModal,GlobalZIndex(170),TabGroup {order:3,modal:true},Node {
        position_type:PositionType::Absolute,width:percent(100.),height:percent(100.),justify_content:JustifyContent::Center,align_items:AlignItems::Center,..default()
    },BackgroundColor(Color::srgba(0.,0.,0.,0.45)))).id();
    let panel=commands.spawn((crate::bevy_shell::window_drag::DraggableDialog,source::frame(skin.frame.clone(),[4.,17.,4.,4.]),Node {
        width:px(720.),max_width:percent(95.),max_height:percent(90.),padding:UiRect {top:px(24.),bottom:px(10.),left:px(12.),right:px(12.)},flex_direction:FlexDirection::Column,row_gap:px(7.),..default()
    })).id(); commands.entity(root).add_child(panel);
    let Some(snapshot)=&state.snapshot else { return root; };
    let bank=snapshot.bank.panel.open;
    source::dialog_caption(commands,panel,if bank {"Guild bank"} else {"Guild"},skin.font.clone());
    let body=commands.spawn((ScrollArea,Node { max_height:px(480.),min_height:px(0.),overflow:Overflow::scroll_y(),flex_direction:FlexDirection::Column,row_gap:px(5.),..default() })).id();
    commands.entity(panel).add_child(body);
    if bank { bank_body(commands,skin,body,state); } else { guild_body(commands,skin,body,state); }
    if let Some(confirm)=&state.confirmation {
        label(commands,skin,body,&confirmation_text(confirm));
        let row=row(commands,body);
        button(commands,skin,row,state,Action::Confirm,"Confirm",true);
        button(commands,skin,row,state,Action::Cancel,"Cancel",true);
    }
    let footer=row(commands,panel); button(commands,skin,footer,state,Action::Close,"Close",true);
    root
}
fn confirmation_text(confirm:&Confirmation) -> String {
    match &confirm.action {
        Action::Kick(character)|Action::Rank(character,_)=> {
            let name=confirm.snapshot.guild.panel.roster.iter().find(|m|m.character==*character).map_or("",|m|m.name.as_str());
            if let Action::Rank(_,level)=confirm.action {
                let rank=confirm.snapshot.guild.panel.own.as_ref().and_then(|own|own.ranks.get(usize::from(level.saturating_sub(1)))).map_or("",String::as_str);
                format!("Change {name} to {rank} (rank {level})?")
            } else { format!("Kick {name} from the guild?") }
        }
        Action::Leave=>"Leave your guild?".into(),
        Action::Bank(intent)=>format!("{} {} — guild {}?",match intent {BankIntent::Deposit=>"Deposit",BankIntent::Withdraw=>"Withdraw",_=>"Claim"},confirm.draft.fields[1],confirm.draft.fields[0]),
        _=>"Confirm guild action?".into(),
    }
}
fn guild_body(commands:&mut Commands,skin:&Skins,parent:Entity,state:&State) {
    let Some(snapshot)=&state.snapshot else { return; };
    let panel=&snapshot.guild.panel;
    if !panel.selected { label(commands,skin,parent,"Guild unavailable in this session"); return; }
    let enabled=!panel.pending && state.confirmation.is_none();
    if let Some(own)=&panel.own {
        label(commands,skin,parent,&format!("{} — {}",own.name,own.title));
        label(commands,skin,parent,&own.message);
        for (index,rank) in own.ranks.iter().enumerate() { label(commands,skin,parent,&format!("Rank {}: {rank}",index+1)); }
        label(commands,skin,parent,"Roster");
        for member in &panel.roster {
            label(commands,skin,parent,&format!("{} — {} (rank {})",member.name,member.title,member.level));
            let row=row(commands,parent);
            let kick=Intent::Kick(member.character);
            if permitted(panel,&kick) { button(commands,skin,row,state,Action::Kick(member.character),"Kick",enabled); }
            if let Some(level)=member.level.checked_sub(1) && permitted(panel,&Intent::SetRank {character:member.character,level}) {
                button(commands,skin,row,state,Action::Rank(member.character,level),"Promote",enabled);
            }
            if let Some(level)=member.level.checked_add(1) && permitted(panel,&Intent::SetRank {character:member.character,level}) {
                button(commands,skin,row,state,Action::Rank(member.character,level),"Demote",enabled);
            }
        }
        // Visibility uses the same server-rank predicate as the atomic request method.
        if permitted(panel,&Intent::Invite(String::new())) {
            label(commands,skin,parent,"Character to invite"); field(commands,skin,parent,Field::Name,&state.draft.name,29);
            button(commands,skin,parent,state,Action::Invite,"Invite",enabled);
        }
        if permitted(panel,&Intent::Leave) { button(commands,skin,parent,state,Action::Leave,"Leave",enabled); }
    } else { label(commands,skin,parent,"No guild"); }
    label(commands,skin,parent,"Invitations");
    for invitation in &panel.invitations {
        label(commands,skin,parent,&format!("{} — expires {}",invitation.name,invitation.expires_at_unix_ms));
        if panel.own.is_none() { button(commands,skin,parent,state,Action::Join(invitation.guild),"Join",enabled); }
    }
    if panel.pending { label(commands,skin,parent,"Waiting for the server"); }
    if let Some(feedback)=&panel.feedback { label(commands,skin,parent,feedback); }
    let row=row(commands,parent);
    for (action,caption,intent) in [(Action::Refresh,"Refresh",Intent::RefreshOwn),(Action::Invitations,"Invitations",Intent::RefreshInvitations),(Action::Roster,"Roster",Intent::RefreshRoster),(Action::More,"More",Intent::MoreRoster)] {
        button(commands,skin,row,state,action,caption,enabled && permitted(panel,&intent));
    }
}
fn bank_body(commands:&mut Commands,skin:&Skins,parent:Entity,state:&State) {
    let Some(snapshot)=&state.snapshot else { return; };
    let panel=&snapshot.bank.panel;
    label(commands,skin,parent,&oteryn_client_ui::guild_bank::panel_text(panel));
    label(commands,skin,parent,"Guild UUID"); field(commands,skin,parent,Field::Guild,&state.draft.fields[0],36);
    label(commands,skin,parent,"Amount"); field(commands,skin,parent,Field::Amount,&state.draft.fields[1],20);
    let row=row(commands,parent);
    for (intent,caption) in [(BankIntent::Refresh,"Refresh"),(BankIntent::Deposit,"Deposit"),(BankIntent::Withdraw,"Withdraw"),(BankIntent::Claims,"Claims"),(BankIntent::Claim,"Claim"),(BankIntent::Older,"Older"),(BankIntent::MoreClaims,"More claims")] {
        let available=match intent {BankIntent::Older=>panel.next.is_some(),BankIntent::MoreClaims=>panel.next_claim.is_some(),_=>true};
        button(commands,skin,row,state,Action::Bank(intent),caption,available && panel.selected && !panel.pending && state.confirmation.is_none());
    }
}
fn label(commands:&mut Commands,skin:&Skins,parent:Entity,text:&str) {
    let child=commands.spawn((source::text(text,source::font(skin.font.clone(),false)),TextLayout::default(),Node {flex_shrink:0.,..default()})).id();commands.entity(parent).add_child(child);
}
fn row(commands:&mut Commands,parent:Entity) -> Entity {
    let entity=commands.spawn(Node {column_gap:px(5.),row_gap:px(4.),flex_wrap:FlexWrap::Wrap,flex_shrink:0.,..default()}).id();commands.entity(parent).add_child(entity);entity
}
fn button(commands:&mut Commands,skin:&Skins,parent:Entity,state:&State,action:Action,caption:&str,enabled:bool) {
    let entity=commands.spawn((source::button(skin.buttons.clone(),85.),Control {serial:state.serial,action})).observe(activate).id();
    if !enabled { commands.entity(entity).insert(InteractionDisabled); }
    let caption=commands.spawn(source::button_caption(caption,skin.font.clone())).id(); commands.entity(entity).add_child(caption);commands.entity(parent).add_child(entity);
}
fn field(commands:&mut Commands,skin:&Skins,parent:Entity,field:Field,value:&str,limit:usize) {
    let mut editable=EditableText::new(value);editable.max_characters=Some(limit);
    let entity=commands.spawn((field,TextInput,editable,source::font(skin.font.clone(),false),TextColor(source::TEXT_COLOR),TextLayout::no_wrap(),TextCursorStyle::default(),TabIndex(0),
        source::frame(skin.inset.clone(),[1.;4]),Node {width:percent(100.),min_height:px(26.),padding:px(4.).all(),..default()})).id();commands.entity(parent).add_child(entity);
}
