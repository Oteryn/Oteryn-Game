//! Crystal 201189f8, data-global/npc/mordecai.lua. Category membership filters
//! the admitted shop offers in the existing trade window.

const NPC: &str = "oteryn:npc.mordecai";

pub(super) struct Category {
    name: &'static str,
    items: &'static [u32],
}

const CATEGORIES: [Category; 4] = [
    Category {
        name: "runes",
        items: &[
            3147, 3148, 3149, 3152, 3153, 3155, 3156, 3158, 3160, 3161, 3164, 3165, 3166, 3172,
            3173, 3174, 3175, 3176, 3177, 3178, 3179, 3180, 3182, 3188, 3189, 3190, 3191, 3192,
            3197, 3198, 3200, 3202, 3203,
        ],
    },
    Category {
        name: "wands",
        items: &[
            3065, 3066, 3067, 3069, 3070, 3071, 3072, 3073, 3074, 3075, 8082, 8083, 8084, 8092,
            8093, 8094,
        ],
    },
    Category {
        name: "exercise weapons",
        items: &[28556, 28557, 35283, 35284, 35289, 35290],
    },
    Category {
        name: "others",
        items: &[651],
    },
];

impl Category {
    pub(super) fn for_message(npc: &str, text: &str) -> Option<&'static Self> {
        (npc == NPC)
            .then(|| {
                CATEGORIES
                    .iter()
                    .find(|category| text.eq_ignore_ascii_case(category.name))
            })
            .flatten()
    }

    pub(super) fn includes(&self, item_key: &str) -> bool {
        item_key
            .strip_prefix("oteryn:item.tibia.i")
            .and_then(|id| id.parse::<u32>().ok())
            .is_some_and(|id| self.items.contains(&id))
    }

    pub(super) fn reply(&self) -> String {
        let alternatives = CATEGORIES
            .iter()
            .filter(|category| category.name != self.name)
            .map(|category| format!("{{{}}}", category.name))
            .collect::<Vec<_>>()
            .join(" or ");
        format!("Of course, just browse through my wares. You can also look at {alternatives}.")
    }
}
