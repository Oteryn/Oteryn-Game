use oteryn_client_runtime::cui3::daily_boost::DailyBoostView;
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DailyBoostPanel {
    pub selected: bool,
    pub open: bool,
    pub pending: bool,
    pub last_command: Option<u32>,
    pub boost: Option<DailyBoostView>,
    pub result: Vec<u8>,
    pub feedback: Option<&'static str>,
}
