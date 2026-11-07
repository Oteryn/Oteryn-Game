//! Fixed data from Crystal 201189f, not a second Item or creature authority.
//! Prices: data/scripts/lib/task_board_shop.lua:33-248.
//! Delivery quantities: data/scripts/lib/task_board_delivery_items.lua:23-EOF.
//! Approved source correction: PR #2764 ANSWER 6544430997 replaces non-pickupable
//! grappling hook 5800 (Lua:64) with portable 35588 (items.xml:66376-66379), still 50.
use super::TaskBoardItemQuantity;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskBoardShopKind {
    Item,
    Mount,
    Outfit,
    ItemDouble,
    Promotion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaskBoardShopOffer {
    pub id: u16,
    pub kind: TaskBoardShopKind,
    pub name: &'static str,
    pub appearance_or_item: u16,
    pub addon: u8,
    pub second_item: Option<u16>,
    pub points: u32,
}

impl TaskBoardShopOffer {
    /// Crystal weekly shop cpp:595-610,638-650 also wraps non-pickupable rewards.
    /// That includes Cerberus 31465: appearance take=false, even without wrapableTo.
    /// PR #2764 ANSWERs 6542950356/6544295916 leave the Item/House slice separate: the writer
    /// must refuse these offers without debiting points or reserving an Inbox slot.
    #[must_use]
    pub const fn requires_decoration_kit(&self) -> bool {
        matches!(self.id, 10..=22)
    }
}

pub const SHOP_OFFERS: &[TaskBoardShopOffer] = &[
    TaskBoardShopOffer {
        id: 0,
        kind: TaskBoardShopKind::Outfit,
        name: "Feral Trapper (Base Outfit)",
        appearance_or_item: 1907,
        addon: 0,
        second_item: None,
        points: 100000,
    },
    TaskBoardShopOffer {
        id: 1,
        kind: TaskBoardShopKind::Outfit,
        name: "Feral Trapper (Addon 1)",
        appearance_or_item: 1907,
        addon: 1,
        second_item: None,
        points: 35000,
    },
    TaskBoardShopOffer {
        id: 2,
        kind: TaskBoardShopKind::Outfit,
        name: "Feral Trapper (Addon 2)",
        appearance_or_item: 1907,
        addon: 2,
        second_item: None,
        points: 35000,
    },
    TaskBoardShopOffer {
        id: 3,
        kind: TaskBoardShopKind::Outfit,
        name: "Falconer (Base Outfit)",
        appearance_or_item: 1282,
        addon: 0,
        second_item: None,
        points: 100000,
    },
    TaskBoardShopOffer {
        id: 4,
        kind: TaskBoardShopKind::Outfit,
        name: "Falconer (Addon 1)",
        appearance_or_item: 1282,
        addon: 1,
        second_item: None,
        points: 35000,
    },
    TaskBoardShopOffer {
        id: 5,
        kind: TaskBoardShopKind::Outfit,
        name: "Falconer (Addon 2)",
        appearance_or_item: 1282,
        addon: 2,
        second_item: None,
        points: 35000,
    },
    TaskBoardShopOffer {
        id: 6,
        kind: TaskBoardShopKind::Mount,
        name: "Tidal Seawater Predator (Mount)",
        appearance_or_item: 1908,
        addon: 0,
        second_item: None,
        points: 145000,
    },
    TaskBoardShopOffer {
        id: 7,
        kind: TaskBoardShopKind::Mount,
        name: "Ashen Coast Predator (Mount)",
        appearance_or_item: 1909,
        addon: 0,
        second_item: None,
        points: 145000,
    },
    TaskBoardShopOffer {
        id: 8,
        kind: TaskBoardShopKind::Mount,
        name: "Crimson Bay Predator (Mount)",
        appearance_or_item: 1910,
        addon: 0,
        second_item: None,
        points: 145000,
    },
    TaskBoardShopOffer {
        id: 9,
        kind: TaskBoardShopKind::Mount,
        name: "Antelope (Mount)",
        appearance_or_item: 1281,
        addon: 0,
        second_item: None,
        points: 145000,
    },
    TaskBoardShopOffer {
        id: 10,
        kind: TaskBoardShopKind::Item,
        name: "Falcon Pet",
        appearance_or_item: 36753,
        addon: 0,
        second_item: None,
        points: 135000,
    },
    TaskBoardShopOffer {
        id: 11,
        kind: TaskBoardShopKind::Item,
        name: "Cerberus Champion Puppy",
        appearance_or_item: 31465,
        addon: 0,
        second_item: None,
        points: 75000,
    },
    TaskBoardShopOffer {
        id: 12,
        kind: TaskBoardShopKind::Item,
        name: "Hellflayer Trophy",
        appearance_or_item: 32753,
        addon: 0,
        second_item: None,
        points: 80000,
    },
    TaskBoardShopOffer {
        id: 13,
        kind: TaskBoardShopKind::Item,
        name: "Golden Hunter Trophy",
        appearance_or_item: 32756,
        addon: 0,
        second_item: None,
        points: 80000,
    },
    TaskBoardShopOffer {
        id: 14,
        kind: TaskBoardShopKind::Item,
        name: "Brachiodemon Trophy",
        appearance_or_item: 36748,
        addon: 0,
        second_item: None,
        points: 80000,
    },
    TaskBoardShopOffer {
        id: 15,
        kind: TaskBoardShopKind::Item,
        name: "Many Faces Trophy",
        appearance_or_item: 36749,
        addon: 0,
        second_item: None,
        points: 50000,
    },
    TaskBoardShopOffer {
        id: 16,
        kind: TaskBoardShopKind::Item,
        name: "Sea Serpent Trophy",
        appearance_or_item: 32752,
        addon: 0,
        second_item: None,
        points: 15000,
    },
    TaskBoardShopOffer {
        id: 17,
        kind: TaskBoardShopKind::Item,
        name: "Silver Hunter Trophy",
        appearance_or_item: 32755,
        addon: 0,
        second_item: None,
        points: 15000,
    },
    TaskBoardShopOffer {
        id: 18,
        kind: TaskBoardShopKind::Item,
        name: "Bronze Hunter Trophy",
        appearance_or_item: 32754,
        addon: 0,
        second_item: None,
        points: 3000,
    },
    TaskBoardShopOffer {
        id: 19,
        kind: TaskBoardShopKind::Item,
        name: "Gozzler Trophy",
        appearance_or_item: 32751,
        addon: 0,
        second_item: None,
        points: 3000,
    },
    TaskBoardShopOffer {
        id: 20,
        kind: TaskBoardShopKind::Item,
        name: "Demon Doll",
        appearance_or_item: 32918,
        addon: 0,
        second_item: None,
        points: 37500,
    },
    TaskBoardShopOffer {
        id: 21,
        kind: TaskBoardShopKind::Item,
        name: "Vexclaw Doll",
        appearance_or_item: 32943,
        addon: 0,
        second_item: None,
        points: 37500,
    },
    TaskBoardShopOffer {
        id: 22,
        kind: TaskBoardShopKind::ItemDouble,
        name: "Bone Bed",
        appearance_or_item: 32795,
        addon: 0,
        second_item: Some(32796),
        points: 35000,
    },
    TaskBoardShopOffer {
        id: 23,
        kind: TaskBoardShopKind::Promotion,
        name: "Bonus Promotion",
        appearance_or_item: 0,
        addon: 0,
        second_item: None,
        points: 100,
    },
];

pub const DELIVERY_ITEMS: &[TaskBoardItemQuantity] = &[
    TaskBoardItemQuantity {
        item_id: 811,
        amount: 20,
    },
    TaskBoardItemQuantity {
        item_id: 813,
        amount: 20,
    },
    TaskBoardItemQuantity {
        item_id: 818,
        amount: 20,
    },
    TaskBoardItemQuantity {
        item_id: 819,
        amount: 20,
    },
    TaskBoardItemQuantity {
        item_id: 820,
        amount: 20,
    },
    TaskBoardItemQuantity {
        item_id: 821,
        amount: 20,
    },
    TaskBoardItemQuantity {
        item_id: 824,
        amount: 20,
    },
    TaskBoardItemQuantity {
        item_id: 825,
        amount: 20,
    },
    TaskBoardItemQuantity {
        item_id: 826,
        amount: 20,
    },
    TaskBoardItemQuantity {
        item_id: 827,
        amount: 20,
    },
    TaskBoardItemQuantity {
        item_id: 828,
        amount: 20,
    },
    TaskBoardItemQuantity {
        item_id: 829,
        amount: 20,
    },
    TaskBoardItemQuantity {
        item_id: 830,
        amount: 20,
    },
    TaskBoardItemQuantity {
        item_id: 3234,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 3269,
        amount: 20,
    },
    TaskBoardItemQuantity {
        item_id: 3275,
        amount: 20,
    },
    TaskBoardItemQuantity {
        item_id: 3291,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 3292,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 3301,
        amount: 20,
    },
    TaskBoardItemQuantity {
        item_id: 3306,
        amount: 20,
    },
    TaskBoardItemQuantity {
        item_id: 3307,
        amount: 20,
    },
    TaskBoardItemQuantity {
        item_id: 3316,
        amount: 20,
    },
    TaskBoardItemQuantity {
        item_id: 3318,
        amount: 20,
    },
    TaskBoardItemQuantity {
        item_id: 3320,
        amount: 20,
    },
    TaskBoardItemQuantity {
        item_id: 3322,
        amount: 20,
    },
    TaskBoardItemQuantity {
        item_id: 3330,
        amount: 20,
    },
    TaskBoardItemQuantity {
        item_id: 3333,
        amount: 20,
    },
    TaskBoardItemQuantity {
        item_id: 3337,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 3342,
        amount: 20,
    },
    TaskBoardItemQuantity {
        item_id: 3346,
        amount: 20,
    },
    TaskBoardItemQuantity {
        item_id: 3364,
        amount: 10,
    },
    TaskBoardItemQuantity {
        item_id: 3371,
        amount: 20,
    },
    TaskBoardItemQuantity {
        item_id: 3373,
        amount: 20,
    },
    TaskBoardItemQuantity {
        item_id: 3413,
        amount: 20,
    },
    TaskBoardItemQuantity {
        item_id: 3415,
        amount: 20,
    },
    TaskBoardItemQuantity {
        item_id: 3421,
        amount: 20,
    },
    TaskBoardItemQuantity {
        item_id: 3429,
        amount: 20,
    },
    TaskBoardItemQuantity {
        item_id: 3509,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 3557,
        amount: 20,
    },
    TaskBoardItemQuantity {
        item_id: 3575,
        amount: 20,
    },
    TaskBoardItemQuantity {
        item_id: 5479,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 35588,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 5888,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 5889,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 6558,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 7381,
        amount: 20,
    },
    TaskBoardItemQuantity {
        item_id: 7383,
        amount: 20,
    },
    TaskBoardItemQuantity {
        item_id: 7386,
        amount: 20,
    },
    TaskBoardItemQuantity {
        item_id: 7387,
        amount: 20,
    },
    TaskBoardItemQuantity {
        item_id: 7388,
        amount: 20,
    },
    TaskBoardItemQuantity {
        item_id: 7402,
        amount: 10,
    },
    TaskBoardItemQuantity {
        item_id: 7413,
        amount: 20,
    },
    TaskBoardItemQuantity {
        item_id: 7425,
        amount: 20,
    },
    TaskBoardItemQuantity {
        item_id: 7427,
        amount: 20,
    },
    TaskBoardItemQuantity {
        item_id: 7428,
        amount: 20,
    },
    TaskBoardItemQuantity {
        item_id: 7436,
        amount: 20,
    },
    TaskBoardItemQuantity {
        item_id: 7449,
        amount: 20,
    },
    TaskBoardItemQuantity {
        item_id: 7454,
        amount: 10,
    },
    TaskBoardItemQuantity {
        item_id: 8043,
        amount: 20,
    },
    TaskBoardItemQuantity {
        item_id: 8045,
        amount: 20,
    },
    TaskBoardItemQuantity {
        item_id: 8050,
        amount: 10,
    },
    TaskBoardItemQuantity {
        item_id: 8061,
        amount: 10,
    },
    TaskBoardItemQuantity {
        item_id: 8074,
        amount: 10,
    },
    TaskBoardItemQuantity {
        item_id: 9054,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 9055,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 9631,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 9632,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 9637,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 9643,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 9649,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 9652,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 9656,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 9659,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 9662,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 9667,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 9668,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 9688,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 10276,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 10277,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 10278,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 10282,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 10291,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 10293,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 10301,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 10303,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 10305,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 10306,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 10308,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 10310,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 10312,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 10313,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 10315,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 10316,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 10317,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 10320,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 10321,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 10384,
        amount: 20,
    },
    TaskBoardItemQuantity {
        item_id: 10386,
        amount: 20,
    },
    TaskBoardItemQuantity {
        item_id: 10397,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 10404,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 10407,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 10408,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 10410,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 10411,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 10413,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 10414,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 10415,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 10416,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 10418,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 10438,
        amount: 20,
    },
    TaskBoardItemQuantity {
        item_id: 10439,
        amount: 20,
    },
    TaskBoardItemQuantity {
        item_id: 10444,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 10449,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 10454,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 10455,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 10456,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 11443,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 11449,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 11450,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 11454,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 11456,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 11457,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 11465,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 11471,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 11477,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 11482,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 11487,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 11488,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 11490,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 11491,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 11510,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 11511,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 11512,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 11514,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 11659,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 11660,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 11661,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 11671,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 11672,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 11673,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 12313,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 12683,
        amount: 20,
    },
    TaskBoardItemQuantity {
        item_id: 12730,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 14008,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 14009,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 14010,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 14011,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 14013,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 14017,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 14041,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 14044,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 14076,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 14078,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 14080,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 14083,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 14225,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 14247,
        amount: 20,
    },
    TaskBoardItemQuantity {
        item_id: 16130,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 16132,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 16133,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 16135,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 16138,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 16139,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 16140,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 16160,
        amount: 20,
    },
    TaskBoardItemQuantity {
        item_id: 16163,
        amount: 20,
    },
    TaskBoardItemQuantity {
        item_id: 17463,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 17809,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 17812,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 17817,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 17818,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 17819,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 17824,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 17826,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 17829,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 17830,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 17831,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 17847,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 17850,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 17851,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 17853,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 17854,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 17855,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 17856,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 17857,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 17859,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 18924,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 18925,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 18926,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 18927,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 18928,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 18995,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 19110,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 19111,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 20198,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 20201,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 20202,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 20203,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 20204,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 20206,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 20207,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 21103,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 21164,
        amount: 20,
    },
    TaskBoardItemQuantity {
        item_id: 21165,
        amount: 20,
    },
    TaskBoardItemQuantity {
        item_id: 21168,
        amount: 20,
    },
    TaskBoardItemQuantity {
        item_id: 21169,
        amount: 20,
    },
    TaskBoardItemQuantity {
        item_id: 21171,
        amount: 20,
    },
    TaskBoardItemQuantity {
        item_id: 21174,
        amount: 20,
    },
    TaskBoardItemQuantity {
        item_id: 21175,
        amount: 20,
    },
    TaskBoardItemQuantity {
        item_id: 21178,
        amount: 20,
    },
    TaskBoardItemQuantity {
        item_id: 21179,
        amount: 20,
    },
    TaskBoardItemQuantity {
        item_id: 21180,
        amount: 20,
    },
    TaskBoardItemQuantity {
        item_id: 21182,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 21193,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 21195,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 21196,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 21197,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 21199,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 21201,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 21204,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 21974,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 22051,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 22054,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 22055,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 22056,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 22057,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 22085,
        amount: 20,
    },
    TaskBoardItemQuantity {
        item_id: 22086,
        amount: 20,
    },
    TaskBoardItemQuantity {
        item_id: 22087,
        amount: 20,
    },
    TaskBoardItemQuantity {
        item_id: 22184,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 22186,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 22188,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 22191,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 22729,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 23502,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 23503,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 23504,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 23505,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 23506,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 23510,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 23511,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 23514,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 23515,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 23516,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 23518,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 23519,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 23520,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 23521,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 23523,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 24380,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 24384,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 24385,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 24938,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 25691,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 25693,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 25695,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 25696,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 25697,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 25699,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 25701,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 25742,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 27369,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 27462,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 27463,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 27592,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 27593,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 27594,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 27595,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 27596,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 27597,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 27598,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 27599,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 27600,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 27601,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 27602,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 27603,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 27604,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 27606,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 28566,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 28569,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 28570,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 28822,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 28823,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 30005,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 30058,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 31331,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 31438,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 31439,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 31440,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 31441,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 31442,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 31443,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 31558,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 31559,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 31560,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 31561,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 31678,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 32698,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 33934,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 33938,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 33943,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 33944,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 33945,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 33982,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 34141,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 34160,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 34162,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 35571,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 35572,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 35573,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 35574,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 35596,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 36769,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 36770,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 36771,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 36772,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 36773,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 36774,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 36775,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 36776,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 36777,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 36778,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 36779,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 36780,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 36785,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 36786,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 36789,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 36790,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 36805,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 36806,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 36807,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 36820,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 36821,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 36822,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 36823,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 36971,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 36972,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 37109,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 37110,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 39375,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 39376,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 39377,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 39378,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 39379,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 39380,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 39381,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 39382,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 39383,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 39384,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 39386,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 39387,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 39388,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 39389,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 39391,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 39392,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 39393,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 39394,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 39395,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 39401,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 39402,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 39406,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 39407,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 39408,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 39409,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 39410,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 39411,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 39412,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 39413,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 39414,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 40582,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 40583,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 40584,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 40585,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 40586,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 40587,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 43729,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 43730,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 43731,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 43732,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 43734,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 43846,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 43848,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 43849,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 43850,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 43851,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 43852,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 43853,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 44743,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 44744,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 44745,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 44746,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 44747,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 44748,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 44749,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 48252,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 48255,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 48508,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 48509,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 48510,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 48511,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 49894,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 49908,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 50056,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 50057,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 50058,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 50059,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 51419,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 51420,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 51422,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 51423,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 51425,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 51426,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 51427,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 51472,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 51473,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 51474,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 51475,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 51476,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 51477,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 51478,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 51479,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 51480,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 51482,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 51484,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 51485,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 51486,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 52633,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 52662,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 52663,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 52664,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 52705,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 52706,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 52707,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 52718,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 52719,
        amount: 50,
    },
    TaskBoardItemQuantity {
        item_id: 52720,
        amount: 50,
    },
];
