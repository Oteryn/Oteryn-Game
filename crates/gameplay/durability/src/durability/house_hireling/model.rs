//! Persisted Hireling identity; no runtime actor handle is durable.
use super::super::DurabilityError;
use sqlx::{Row, postgres::PgRow};

pub const HIRELING_LAMP_KEY: &str = "oteryn:item.tibia.i29432";
pub const MAX_HIRELINGS: usize = 10;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HirelingSnapshot {
    pub id: [u8;16],
    pub owner: [u8;16],
    pub world: [u8;16],
    pub name: String,
    pub sex: u8,
    pub look_type: u16,
    pub colors: [u8;4],
    pub skills: u8,
    pub outfits: Vec<u16>,
    pub house: Option<String>,
    pub position: Option<(u16,u16,i8)>,
    pub lamp: Option<[u8;16]>,
    pub legacy_grant: bool,
}

impl HirelingSnapshot {
    pub fn has_skill(&self, id:u16)->bool {
        (1001..=1004).contains(&id) && self.skills & (1 << (id-1001)) != 0
    }
    pub fn allows_outfit(&self, look:u16)->bool {
        // Crystal201189f data/libs/systems/hireling.lua:30-41,205-227.
        let female=self.sex==2;
        if look==if female {1107} else {1108} {return true}
        [(2001,1109,1110),(2002,1113,1114),(2003,1115,1116),(2004,1111,1112),
         (2005,1117,1118),(2006,1129,1130),(2007,1131,1132),(2008,1123,1124),(2009,1125,1126)]
        .iter().any(|(id,f,m)|look==if female{*f}else{*m} && self.outfits.contains(id))
    }
    pub fn npc_key(&self)->String { npc_key(self.id) }
}

pub fn npc_key(id:[u8;16])->String {
    let mut value=String::from("oteryn:npc.hireling:");
    for byte in id {use std::fmt::Write; let _=write!(value,"{byte:02x}");}
    value
}

pub fn npc_identity(key:&str)->Option<[u8;16]> {
    let raw=key.strip_prefix("oteryn:npc.hireling:")?;
    if raw.len()!=32 || !raw.bytes().all(|b|b.is_ascii_hexdigit()&&!b.is_ascii_uppercase()) {return None}
    let mut id=[0;16];
    for (i,byte) in id.iter_mut().enumerate() {*byte=u8::from_str_radix(&raw[i*2..i*2+2],16).ok()?;}
    (id[6]>>4==7 && id[8]&0xc0==0x80).then_some(id)
}

// All call sites append a source literal; no runtime SQL construction is admitted.
macro_rules! snapshot_query {
    ($tail:literal) => { sqlx::query(concat!("SELECT ", "uuid_send(h.hireling_id) AS id, uuid_send(h.owner_character_id) AS owner, \
 uuid_send(h.world_id) AS world,h.name,h.sex,h.look_type,h.colors,h.active,h.house_key,h.x,h.y,h.floor, \
 uuid_send(h.lamp_item_instance_id) AS lamp,h.legacy_grant, \
 ARRAY(SELECT e.entitlement_id FROM game_hireling_entitlements e WHERE e.character_id=h.owner_character_id ORDER BY e.entitlement_id) AS entitlements", $tail)) };
}
pub(super) use snapshot_query;

pub(super) fn decode(row:&PgRow)->Result<HirelingSnapshot,DurabilityError> {
    fn bytes<const N:usize>(row:&PgRow,name:&str)->Result<[u8;N],DurabilityError>{
        row.try_get::<Vec<u8>,_>(name)?.try_into().map_err(|_|DurabilityError::InvalidStoredState)
    }
    let entitlements:Vec<i16>=row.try_get("entitlements")?;
    let mut skills=0;
    let mut outfits=Vec::new();
    for entitlement in entitlements {
        if (1001..=1004).contains(&entitlement) {skills|=1 << (entitlement-1001);}
        else if (2001..=2009).contains(&entitlement) {outfits.push(entitlement as u16);}
        else {return Err(DurabilityError::InvalidStoredState)}
    }
    Ok(HirelingSnapshot {
        id:bytes(row,"id")?,owner:bytes(row,"owner")?,world:bytes(row,"world")?,name:row.try_get("name")?,
        sex:u8::try_from(row.try_get::<i16,_>("sex")?).map_err(|_|DurabilityError::InvalidStoredState)?,
        look_type:u16::try_from(row.try_get::<i32,_>("look_type")?).map_err(|_|DurabilityError::InvalidStoredState)?,
        colors:bytes(row,"colors")?,skills,outfits,house:row.try_get("house_key")?,
        position:if row.try_get::<bool,_>("active")? {Some((
            u16::try_from(row.try_get::<i32,_>("x")?).map_err(|_|DurabilityError::InvalidStoredState)?,
            u16::try_from(row.try_get::<i32,_>("y")?).map_err(|_|DurabilityError::InvalidStoredState)?,
            i8::try_from(row.try_get::<i16,_>("floor")?).map_err(|_|DurabilityError::InvalidStoredState)?))}else{None},
        lamp:row.try_get::<Option<Vec<u8>>,_>("lamp")?.map(|v|v.try_into().map_err(|_|DurabilityError::InvalidStoredState)).transpose()?,
        legacy_grant:row.try_get("legacy_grant")?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn identity_keys_round_trip_and_reject_noncanonical_shapes(){
        let mut id=[7;16];id[6]=0x70;id[8]=0x80;assert_eq!(npc_identity(&npc_key(id)),Some(id));
        assert_eq!(npc_identity("oteryn:npc.hireling-bank:house"),None);
        assert_eq!(npc_identity("oteryn:npc.hireling:zz"),None);
    }
    #[test]
    fn outfit_entitlements_require_exact_sex_and_unlocked_profession(){
        let mut h=HirelingSnapshot{id:[0;16],owner:[0;16],world:[0;16],name:"Alice".into(),sex:2,look_type:136,
            colors:[0;4],skills:1,outfits:vec![2001],house:None,position:None,lamp:None,legacy_grant:false};
        assert!(h.allows_outfit(1107));assert!(h.allows_outfit(1109));assert!(!h.allows_outfit(1110));
        assert!(!h.allows_outfit(1113));assert!(h.has_skill(1001));assert!(!h.has_skill(1002));
        h.sex=1;assert!(h.allows_outfit(1108));assert!(h.allows_outfit(1110));assert!(!h.allows_outfit(1109));
    }
}
