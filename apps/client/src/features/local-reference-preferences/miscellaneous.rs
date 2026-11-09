//! Local confirmations and session choices; Platform retains all session authority.
use serde::{Serialize,Deserialize};
#[derive(Clone,Copy,Debug,PartialEq,Eq,Serialize,Deserialize)]
#[serde(default,deny_unknown_fields)]
pub struct MiscellaneousPreferences {
    pub stay_logged_in:bool,pub remember_device:bool,pub quick_login:bool,
    pub confirm_logout:bool,pub confirm_exit:bool,
}
impl Default for MiscellaneousPreferences {
    fn default()->Self{Self{stay_logged_in:false,remember_device:false,quick_login:false,confirm_logout:true,confirm_exit:true}}
}
impl MiscellaneousPreferences {
    pub fn path()->Option<std::path::PathBuf>{super::PreferenceStore::path().map(|p|p.with_extension("session-options.json"))}
    pub fn load()->Self {Self::path().and_then(|p|Self::read(&p).ok()).unwrap_or_default()}
    pub fn read(path:&std::path::Path)->std::io::Result<Self>{
        use std::io::Read;
        let mut bytes=Vec::new();std::fs::File::open(path)?.take(4097).read_to_end(&mut bytes)?;
        if bytes.len()>4096{return Err(std::io::Error::other("Invalid session options size"));}
        serde_json::from_slice(&bytes).map_err(std::io::Error::other)
    }
    pub fn save(&self,path:&std::path::Path)->std::io::Result<()> {
        use std::io::Write;
        if let Some(parent)=path.parent(){std::fs::create_dir_all(parent)?;}
        let temporary=path.with_extension("next");let mut file=std::fs::File::create(&temporary)?;
        file.write_all(&serde_json::to_vec(self).map_err(std::io::Error::other)?)?;file.sync_all()?;
        std::fs::rename(temporary,path)
    }

}
impl super::SettingsState {
    pub fn request_native_sign_out(&mut self){self.native_sign_out=true;}
    pub fn take_native_sign_out(&mut self)->bool{std::mem::take(&mut self.native_sign_out)}
}
