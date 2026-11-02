//! Stateful command-7 service choices. Caller binds this state to the live NPC conversation.
use super::{
    Skill, catalogue,
    food::{self, FoodPlan},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Topic {
    #[default]
    Services,
    Bank,
    Food,
    SpecificFood,
    SkillFood,
    Goods,
    Packages,
    PackageConfirmation(usize),
    Lamp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Dialogue {
    pub topic: Topic,
    pub category: Option<u8>,
    pub page: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Reply(&'static str),
    Services,
    Bank,
    OpenStash,
    Trade { category: u8, page: u32 },
    Cook(FoodPlan),
    Package(usize),
    ReturnLamp,
    Outfit,
    Close,
}

const MISSING_SKILL: &str = "Sorry, but I do not have mastery in this skill yet.";
const FOOD_CHOICES: &str = "Choose chilli con carniphila, svargrond salmon filet, carrion casserole, consecrated beef, roasted wyvern wings, carrot pie, tropical marinated tiger, or delicatessen salad.";
const SKILL_CHOICES: &str =
    "Would you like to boost your magic, melee, shielding, or distance skill?";

impl Dialogue {
    /// `roll` is sampled by the server only for the applicable random-dish branch.
    /// Source: Crystal data-global/npc/hireling.lua:2224-2429.
    pub fn step(&mut self, text: &str, skills: u8, owner: bool, roll: u8) -> Action {
        let text = text.trim().to_ascii_lowercase();
        if matches!(text.as_str(), "bye" | "farewell") {
            *self = Self::default();
            return Action::Close;
        }
        if matches!(text.as_str(), "hi" | "hello" | "service" | "services") {
            self.topic = Topic::Services;
            return Action::Services;
        }
        // Crystal's package selector is outside the skill branch. Q3= A retains it.
        if text == "imbuement packages" {
            self.topic = Topic::Packages;
            return Action::Reply("Choose an imbuement package by name.");
        }
        if self.topic == Topic::Packages {
            if let Some(index) = catalogue::PACKAGES.iter().position(|p| p.name == text) {
                self.topic = Topic::PackageConfirmation(index);
                return Action::Reply(
                    "Confirm this imbuement package with yes, or cancel with no.",
                );
            }
            return Action::Reply("Please choose an available imbuement package.");
        }
        if let Topic::PackageConfirmation(index) = self.topic {
            if text == "yes" {
                self.topic = Topic::Services;
                return Action::Package(index);
            }
            if text == "no" {
                self.topic = Topic::Packages;
            }
            return Action::Reply("The imbuement package has not been purchased.");
        }
        if self.topic == Topic::Services {
            return match text.as_str() {
                "bank" if Skill::Banker.owned(skills) => {
                    self.topic = Topic::Bank;
                    Action::Reply("What can I do for you and your bank business?")
                }
                "food" if Skill::Cook.owned(skills) => {
                    self.topic = Topic::Food;
                    Action::Reply(
                        "A surprise costs 15000 gold; a specific meal costs 90000 gold. Which would you like?",
                    )
                }
                "stash" if Skill::Steward.owned(skills) => Action::OpenStash,
                "bank" | "food" | "stash" => Action::Reply(MISSING_SKILL),
                "goods" | "trade" => {
                    self.topic = Topic::Goods;
                    if Skill::Trader.owned(skills) {
                        Action::Reply(
                            "Choose various, exercise weapons, equipment, distance, wands, rods, potions, runes, supplies, tools, postal, buy loot, or imbuement packages.",
                        )
                    } else {
                        Action::Reply(
                            "While I'm not a trader, I still have a collection of various items to sell.",
                        )
                    }
                }
                "lamp" if owner => {
                    self.topic = Topic::Lamp;
                    Action::Reply("Are you sure you want me to go back to my lamp?")
                }
                "outfit" if owner => Action::Outfit,
                "lamp" | "outfit" => Action::Reply("You are not the master of this hireling."),
                _ => Action::Services,
            };
        }
        match self.topic {
            Topic::Bank => {
                if Skill::Banker.owned(skills) {
                    Action::Bank
                } else {
                    Action::Reply(MISSING_SKILL)
                }
            }
            Topic::Lamp => {
                self.topic = Topic::Services;
                if text == "yes" && owner {
                    Action::ReturnLamp
                } else if !owner {
                    Action::Reply("You are not the master of this hireling.")
                } else {
                    Action::Reply("Alright then, I will be here.")
                }
            }
            Topic::Food | Topic::SpecificFood | Topic::SkillFood => {
                if !Skill::Cook.owned(skills) {
                    return Action::Reply(MISSING_SKILL);
                }
                self.food(&text, roll)
            }
            Topic::Goods => self.goods(&text, skills),
            _ => Action::Services,
        }
    }

    fn food(&mut self, text: &str, roll: u8) -> Action {
        let plan = match self.topic {
            Topic::Food => match text {
                "specific" => {
                    self.topic = Topic::SpecificFood;
                    return Action::Reply(FOOD_CHOICES);
                }
                "surprise" => match food::surprise(roll) {
                    Some(Some(plan)) => Some(plan),
                    Some(None) => {
                        self.topic = Topic::SkillFood;
                        return Action::Reply(SKILL_CHOICES);
                    }
                    None => None,
                },
                "yes" => food::ordinary(roll),
                "no" => {
                    self.topic = Topic::Services;
                    return Action::Services;
                }
                _ => None,
            },
            Topic::SpecificFood => food::specific(text),
            Topic::SkillFood => food::skill_dish(text),
            _ => None,
        };
        if let Some(plan) = plan {
            self.topic = Topic::Services;
            Action::Cook(plan)
        } else {
            Action::Reply("Please choose a valid meal or skill for this order.")
        }
    }

    fn goods(&mut self, text: &str, skills: u8) -> Action {
        let selection = if text == "next" {
            self.category.zip(self.page.checked_add(1))
        } else if text == "previous" {
            self.category.zip(self.page.checked_sub(1))
        } else if let Some(category) = catalogue::category(text) {
            Some((category, 0))
        } else {
            text.rsplit_once(' ').and_then(|(name, page)| {
                Some((
                    catalogue::category(name)?,
                    page.parse::<u32>().ok()?.checked_sub(1)?,
                ))
            })
        };
        let Some((category, page)) = selection else {
            return Action::Reply("Choose a category and page, or use next and previous.");
        };
        if !catalogue::allowed(category, skills) {
            return Action::Reply(MISSING_SKILL);
        }
        self.category = Some(category);
        self.page = page;
        Action::Trade { category, page }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_refusals_do_not_grant_topics_or_management_to_guests() {
        let mut topic = Dialogue::default();
        for text in ["bank", "food", "stash"] {
            assert_eq!(topic.step(text, 0, false, 0), Action::Reply(MISSING_SKILL));
            assert_eq!(topic.topic, Topic::Services);
        }
        assert!(matches!(topic.step("lamp", 15, false, 0), Action::Reply(_)));
        assert_eq!(topic.topic, Topic::Services);
        assert!(matches!(
            topic.step("outfit", 15, false, 0),
            Action::Reply(_)
        ));
    }

    #[test]
    fn food_selection_requires_current_skill_and_the_correct_conversation_state() {
        let mut topic = Dialogue::default();
        assert_eq!(topic.step("carrot pie", 2, true, 0), Action::Services);
        topic.step("food", 2, true, 0);
        topic.step("specific", 2, true, 0);
        assert_eq!(
            topic.step("carrot pie", 0, true, 0),
            Action::Reply(MISSING_SKILL)
        );
        assert_eq!(
            topic.step("carrot pie", 2, true, 0),
            Action::Cook(FoodPlan {
                item_id: 29409,
                gold: 90000
            })
        );
        assert_eq!(topic.step("carrot pie", 2, true, 0), Action::Services);
        topic.step("food", 2, true, 0);
        topic.step("surprise", 2, true, 5);
        assert_eq!(topic.topic, Topic::SkillFood);
        assert_eq!(
            topic.step("magic", 2, true, 0),
            Action::Cook(FoodPlan {
                item_id: 29410,
                gold: 15000
            })
        );
    }

    #[test]
    fn free_various_and_expanded_pages_use_distinct_current_skill_gates() {
        let mut topic = Dialogue::default();
        topic.step("goods", 0, false, 0);
        assert_eq!(
            topic.step("various", 0, false, 0),
            Action::Trade {
                category: 0,
                page: 0
            }
        );
        assert_eq!(
            topic.step("buy loot", 0, false, 0),
            Action::Reply(MISSING_SKILL)
        );
        assert_eq!(
            topic.step("buy loot 2", 8, false, 0),
            Action::Trade {
                category: 11,
                page: 1
            }
        );
        assert_eq!(
            topic.step("previous", 8, false, 0),
            Action::Trade {
                category: 11,
                page: 0
            }
        );
        assert!(matches!(
            topic.step("buy loot 0", 8, false, 0),
            Action::Reply(_)
        ));
    }

    #[test]
    fn package_confirmation_is_one_selection_and_requires_an_explicit_yes() {
        let mut topic = Dialogue::default();
        assert_eq!(topic.step("bash", 0, false, 0), Action::Services);
        topic.step("imbuement packages", 0, false, 0);
        topic.step("bash", 0, false, 0);
        assert_eq!(topic.step("yes", 0, false, 0), Action::Package(0));
        assert_eq!(topic.step("yes", 0, false, 0), Action::Services);
    }

    #[test]
    fn lamp_confirmation_rechecks_owner_and_farewell_retires_all_dialogue_state() {
        let mut topic = Dialogue::default();
        topic.step("lamp", 0, true, 0);
        assert!(matches!(topic.step("yes", 0, false, 0), Action::Reply(_)));
        topic.step("lamp", 0, true, 0);
        assert_eq!(topic.step("yes", 0, true, 0), Action::ReturnLamp);
        topic.step("goods", 8, true, 0);
        topic.step("buy loot 2", 8, true, 0);
        assert_eq!(topic.step("bye", 8, true, 0), Action::Close);
        assert_eq!(topic, Dialogue::default());
    }
}
