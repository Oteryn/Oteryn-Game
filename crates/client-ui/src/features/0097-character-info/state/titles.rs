use super::Panel;
use oteryn_client_runtime::character_info_panel::{
    self as wire, CharacterInfoOperation as Op, CharacterTitle,
};
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OwnTitles {
    pub earned: Vec<CharacterTitle>,
    pub selected: Option<CharacterTitle>,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TitleFilter {
    #[default]
    All,
    Earned,
    Locked,
    Permanent,
    Temporary,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TitleChoice {
    pub title: CharacterTitle,
    pub earned: bool,
    pub selected: bool,
}
/// Only a self query establishes the complete ownership list. Visible profiles never replace it.
pub fn apply_title_reply(p: &mut Panel, op: Op, bytes: &[u8]) -> bool {
    if wire::result_text(op, bytes).is_none() {
        return false;
    }
    match op {
        Op::QuerySelf(_) => {
            let count = usize::from(bytes[69]);
            let earned = bytes[70..70 + count]
                .iter()
                .filter_map(|id| CharacterTitle::from_id(*id))
                .collect();
            p.own_titles = Some(OwnTitles {
                earned,
                selected: CharacterTitle::from_id(bytes[70 + count]),
            });
        }
        Op::Unlock(title) => {
            if let Some(own) = &mut p.own_titles
                && !own.earned.contains(&title)
            {
                own.earned.push(title);
            }
        }
        Op::Select(title) => {
            if let Some(own) = &mut p.own_titles {
                own.selected = title;
            }
        }
        _ => {}
    }
    true
}
pub fn title_rows(p: &Panel) -> Vec<TitleChoice> {
    let Some(own) = &p.own_titles else {
        return Vec::new();
    };
    let search = p.title_search.to_lowercase();
    let mut rows: Vec<_> = CharacterTitle::ALL
        .into_iter()
        .filter_map(|title| {
            let earned = own.earned.contains(&title);
            let include = match p.title_filter {
                TitleFilter::All => true,
                TitleFilter::Earned => earned,
                TitleFilter::Locked => !earned,
                TitleFilter::Permanent => title.permanent(),
                TitleFilter::Temporary => !title.permanent(),
            };
            (include && title.name().to_lowercase().contains(&search)).then_some(TitleChoice {
                title,
                earned,
                selected: own.selected == Some(title),
            })
        })
        .collect();
    rows.sort_by_key(|r| r.title.name());
    rows
}
