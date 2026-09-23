//! Versioned editable-source additions. Declarative v2 records never become runtime definitions
//! by appearing in the project: only the existing Reference linker owns executable lowering.

use super::*;

pub const WORLD_PROJECT_V2_SOURCE_PROFILE: &str = "OTERYN_WORLD_PROJECT_SOURCE_PROFILE/v2";
pub const WORLD_PROJECT_V2_ROOT_SCHEMA: &str = "OTERYN_WORLD_PROJECT_ROOT/v2";
pub const WORLD_PROJECT_V2_MANIFEST_SCHEMA: &str = "OTERYN_WORLD_PROJECT_MANIFEST/v2";
pub const WORLD_PROJECT_V2_LOCK_SCHEMA: &str = "OTERYN_WORLD_PROJECT_CONTENT_LOCK/v2";

const DECLARATIONS_SCHEMA: &str = "OTERYN_WORLD_PROJECT_DECLARATIONS/v2";
const WORLDS_SCHEMA: &str = "OTERYN_WORLD_PROJECT_WORLDS/v2";
const PRESENTATIONS_SCHEMA: &str = "OTERYN_WORLD_PROJECT_PRESENTATION_BINDINGS/v2";
const ASSETS_SCHEMA: &str = "OTERYN_WORLD_PROJECT_ASSETS/v2";
const PROVENANCE_SCHEMA: &str = "OTERYN_WORLD_PROJECT_PROVENANCE/v2";
const EDITOR_SCHEMA: &str = "OTERYN_WORLD_PROJECT_EDITOR/v2";

const ROLE_SPECS: [(&str, &str, &str); 8] = [
    (
        "reference-records",
        "definitions/reference.json",
        WORLD_PROJECT_REFERENCE_SCHEMA,
    ),
    (
        "declarative-definitions",
        "definitions/declarations.json",
        DECLARATIONS_SCHEMA,
    ),
    ("world-records", "worlds/world.json", WORLDS_SCHEMA),
    (
        "presentation-bindings",
        "presentations/bindings.json",
        PRESENTATIONS_SCHEMA,
    ),
    ("asset-records", "assets/catalog.json", ASSETS_SCHEMA),
    (
        "import-candidates",
        "provenance/imports.json",
        WORLD_PROJECT_IMPORT_SCHEMA,
    ),
    (
        "provenance-records",
        "provenance/sources.json",
        PROVENANCE_SCHEMA,
    ),
    ("editor-records", "editor/author.json", EDITOR_SCHEMA),
];

/// This authoring vocabulary is intentionally distinct from executable `DefinitionFamily`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ProjectV2Family {
    Terrain,
    Presentation,
    LocalObject,
    WorldObject,
    Area,
    Document,
    Achievement,
    Outfit,
    Mount,
    Charm,
    Item,
    Creature,
    Ability,
    Effect,
    Formula,
    Loot,
    Behavior,
    #[serde(rename = "NPC")]
    Npc,
    Dialogue,
    Service,
    Interaction,
    Quest,
    Transition,
    House,
    Encounter,
}

impl ProjectV2Family {
    fn from_reference(value: &str) -> Result<Self, ProjectError> {
        Ok(match parse_family(value)? {
            DefinitionFamily::Terrain => Self::Terrain,
            DefinitionFamily::Presentation => Self::Presentation,
            DefinitionFamily::LocalObject => Self::LocalObject,
            DefinitionFamily::Item => Self::Item,
            DefinitionFamily::Creature => Self::Creature,
            DefinitionFamily::Ability => Self::Ability,
            DefinitionFamily::Effect => Self::Effect,
            DefinitionFamily::Formula => Self::Formula,
            DefinitionFamily::Loot => Self::Loot,
            DefinitionFamily::Behavior => Self::Behavior,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2DefinitionRef {
    pub family: ProjectV2Family,
    pub key: String,
    pub revision: String,
}

impl ProjectV2DefinitionRef {
    fn validate(&self) -> Result<(), ProjectError> {
        ProductionKey::new(&self.key)?;
        DefinitionRevisionRef::new(&self.revision)?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2Identity {
    pub key: String,
    pub revision: String,
}

impl ProjectV2Identity {
    fn validate(&self) -> Result<(), ProjectError> {
        ProductionKey::new(&self.key)?;
        DefinitionRevisionRef::new(&self.revision)?;
        Ok(())
    }
}

/// Candidate-only structural declarations. No enum variant has an executable lowering.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum ProjectV2Declaration {
    WorldObject {
        identity: ProjectV2Identity,
        presentation: Option<ProjectV2DefinitionRef>,
        fields: Vec<ProjectV2CandidateField>,
    },
    Area {
        identity: ProjectV2Identity,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        parent: Option<ProjectV2DefinitionRef>,
        fields: Vec<ProjectV2CandidateField>,
    },
    Document {
        identity: ProjectV2Identity,
        document_type: ProjectV2DocumentType,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        author: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        language: Option<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        content: Vec<String>,
        fields: Vec<ProjectV2CandidateField>,
    },
    Achievement {
        identity: ProjectV2Identity,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        presentation: Option<ProjectV2DefinitionRef>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        source_id: Option<u64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        degree: Option<u8>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        points: Option<u16>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        secret: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        premium: Option<bool>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        unlock_interactions: Vec<ProjectV2DefinitionRef>,
        fields: Vec<ProjectV2CandidateField>,
    },
    Outfit {
        identity: ProjectV2Identity,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        presentations: Vec<ProjectV2DefinitionRef>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        premium: Option<bool>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        acquisition_interactions: Vec<ProjectV2DefinitionRef>,
        fields: Vec<ProjectV2CandidateField>,
    },
    Mount {
        identity: ProjectV2Identity,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        presentation: Option<ProjectV2DefinitionRef>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        speed_bonus: Option<i32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        premium: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        taming_item: Option<ProjectV2DefinitionRef>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        acquisition_interactions: Vec<ProjectV2DefinitionRef>,
        fields: Vec<ProjectV2CandidateField>,
    },
    Charm {
        identity: ProjectV2Identity,
        charm_type: ProjectV2CharmType,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        ranks: Vec<ProjectV2CharmRank>,
        fields: Vec<ProjectV2CandidateField>,
    },
    #[serde(rename = "NPC")]
    Npc {
        identity: ProjectV2Identity,
        presentation: Option<ProjectV2DefinitionRef>,
        behavior: Option<ProjectV2DefinitionRef>,
        dialogue: Option<ProjectV2DefinitionRef>,
        services: Vec<ProjectV2DefinitionRef>,
        fields: Vec<ProjectV2CandidateField>,
    },
    Dialogue {
        identity: ProjectV2Identity,
        fields: Vec<ProjectV2CandidateField>,
    },
    Service {
        identity: ProjectV2Identity,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        offers: Vec<ProjectV2ServiceOffer>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        recipes: Vec<ProjectV2ServiceRecipe>,
        fields: Vec<ProjectV2CandidateField>,
    },
    Interaction {
        identity: ProjectV2Identity,
        fields: Vec<ProjectV2CandidateField>,
    },
    Quest {
        identity: ProjectV2Identity,
        fields: Vec<ProjectV2CandidateField>,
    },
    Transition {
        identity: ProjectV2Identity,
        fields: Vec<ProjectV2CandidateField>,
    },
    House {
        identity: ProjectV2Identity,
        fields: Vec<ProjectV2CandidateField>,
    },
    Encounter {
        identity: ProjectV2Identity,
        fields: Vec<ProjectV2CandidateField>,
    },
}

impl ProjectV2Declaration {
    fn family(&self) -> ProjectV2Family {
        match self {
            Self::WorldObject { .. } => ProjectV2Family::WorldObject,
            Self::Area { .. } => ProjectV2Family::Area,
            Self::Document { .. } => ProjectV2Family::Document,
            Self::Achievement { .. } => ProjectV2Family::Achievement,
            Self::Outfit { .. } => ProjectV2Family::Outfit,
            Self::Mount { .. } => ProjectV2Family::Mount,
            Self::Charm { .. } => ProjectV2Family::Charm,
            Self::Npc { .. } => ProjectV2Family::Npc,
            Self::Dialogue { .. } => ProjectV2Family::Dialogue,
            Self::Service { .. } => ProjectV2Family::Service,
            Self::Interaction { .. } => ProjectV2Family::Interaction,
            Self::Quest { .. } => ProjectV2Family::Quest,
            Self::Transition { .. } => ProjectV2Family::Transition,
            Self::House { .. } => ProjectV2Family::House,
            Self::Encounter { .. } => ProjectV2Family::Encounter,
        }
    }

    fn identity(&self) -> &ProjectV2Identity {
        match self {
            Self::WorldObject { identity, .. }
            | Self::Area { identity, .. }
            | Self::Document { identity, .. }
            | Self::Achievement { identity, .. }
            | Self::Outfit { identity, .. }
            | Self::Mount { identity, .. }
            | Self::Charm { identity, .. }
            | Self::Npc { identity, .. }
            | Self::Dialogue { identity, .. }
            | Self::Service { identity, .. }
            | Self::Interaction { identity, .. }
            | Self::Quest { identity, .. }
            | Self::Transition { identity, .. }
            | Self::House { identity, .. }
            | Self::Encounter { identity, .. } => identity,
        }
    }

    fn fields(&self) -> &[ProjectV2CandidateField] {
        match self {
            Self::WorldObject { fields, .. }
            | Self::Area { fields, .. }
            | Self::Document { fields, .. }
            | Self::Achievement { fields, .. }
            | Self::Outfit { fields, .. }
            | Self::Mount { fields, .. }
            | Self::Charm { fields, .. }
            | Self::Npc { fields, .. }
            | Self::Dialogue { fields, .. }
            | Self::Service { fields, .. }
            | Self::Interaction { fields, .. }
            | Self::Quest { fields, .. }
            | Self::Transition { fields, .. }
            | Self::House { fields, .. }
            | Self::Encounter { fields, .. } => fields,
        }
    }

    fn fields_mut(&mut self) -> &mut Vec<ProjectV2CandidateField> {
        match self {
            Self::WorldObject { fields, .. }
            | Self::Area { fields, .. }
            | Self::Document { fields, .. }
            | Self::Achievement { fields, .. }
            | Self::Outfit { fields, .. }
            | Self::Mount { fields, .. }
            | Self::Charm { fields, .. }
            | Self::Npc { fields, .. }
            | Self::Dialogue { fields, .. }
            | Self::Service { fields, .. }
            | Self::Interaction { fields, .. }
            | Self::Quest { fields, .. }
            | Self::Transition { fields, .. }
            | Self::House { fields, .. }
            | Self::Encounter { fields, .. } => fields,
        }
    }

    fn canonicalize(&mut self) {
        self.fields_mut()
            .sort_by(|left, right| left.field_path.cmp(&right.field_path));
        match self {
            Self::Service {
                offers, recipes, ..
            } => {
                offers.sort();
                recipes.sort_by(|left, right| left.key.cmp(&right.key));
                for recipe in recipes {
                    recipe.canonicalize();
                }
            }
            Self::Achievement {
                unlock_interactions,
                ..
            } => unlock_interactions.sort(),
            Self::Outfit {
                presentations,
                acquisition_interactions,
                ..
            } => {
                presentations.sort();
                acquisition_interactions.sort();
            }
            Self::Mount {
                acquisition_interactions,
                ..
            } => acquisition_interactions.sort(),
            Self::Charm { ranks, .. } => {
                ranks.sort_by_key(|rank| rank.rank);
                for rank in ranks {
                    rank.fields
                        .sort_by(|left, right| left.field_path.cmp(&right.field_path));
                }
            }
            _ => {}
        }
    }

    fn references(&self) -> Vec<(ProjectV2Family, &ProjectV2DefinitionRef)> {
        let mut references = Vec::new();
        match self {
            Self::WorldObject { presentation, .. } => {
                if let Some(reference) = presentation {
                    references.push((ProjectV2Family::Presentation, reference));
                }
            }
            Self::Area { parent, .. } => {
                if let Some(reference) = parent {
                    references.push((ProjectV2Family::Area, reference));
                }
            }
            Self::Document { .. } => {}
            Self::Achievement {
                presentation,
                unlock_interactions,
                ..
            } => {
                if let Some(reference) = presentation {
                    references.push((ProjectV2Family::Presentation, reference));
                }
                references.extend(
                    unlock_interactions
                        .iter()
                        .map(|reference| (ProjectV2Family::Interaction, reference)),
                );
            }
            Self::Outfit {
                presentations,
                acquisition_interactions,
                ..
            } => {
                references.extend(
                    presentations
                        .iter()
                        .map(|reference| (ProjectV2Family::Presentation, reference)),
                );
                references.extend(
                    acquisition_interactions
                        .iter()
                        .map(|reference| (ProjectV2Family::Interaction, reference)),
                );
            }
            Self::Mount {
                presentation,
                taming_item,
                acquisition_interactions,
                ..
            } => {
                if let Some(reference) = presentation {
                    references.push((ProjectV2Family::Presentation, reference));
                }
                if let Some(reference) = taming_item {
                    references.push((ProjectV2Family::Item, reference));
                }
                references.extend(
                    acquisition_interactions
                        .iter()
                        .map(|reference| (ProjectV2Family::Interaction, reference)),
                );
            }
            Self::Charm { ranks, .. } => {
                for rank in ranks {
                    if let Some(reference) = &rank.effect {
                        references.push((ProjectV2Family::Effect, reference));
                    }
                }
            }
            Self::Npc {
                presentation,
                behavior,
                dialogue,
                services,
                ..
            } => {
                if let Some(reference) = presentation {
                    references.push((ProjectV2Family::Presentation, reference));
                }
                if let Some(reference) = behavior {
                    references.push((ProjectV2Family::Behavior, reference));
                }
                if let Some(reference) = dialogue {
                    references.push((ProjectV2Family::Dialogue, reference));
                }
                references.extend(
                    services
                        .iter()
                        .map(|reference| (ProjectV2Family::Service, reference)),
                );
            }
            Self::Service {
                offers, recipes, ..
            } => {
                for offer in offers {
                    references.push((ProjectV2Family::Item, &offer.item));
                    if let Some(currency) = &offer.currency {
                        references.push((ProjectV2Family::Item, currency));
                    }
                }
                for recipe in recipes {
                    for input in &recipe.inputs {
                        references.push((ProjectV2Family::Item, &input.item));
                    }
                    for output in &recipe.outputs {
                        references.push((ProjectV2Family::Item, &output.item));
                    }
                    if let Some(currency) = &recipe.currency {
                        references.push((ProjectV2Family::Item, currency));
                    }
                }
            }
            _ => {}
        }
        references
    }
}

/// Evidence-carrying source observations, with no executable field interpretation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2CandidateField {
    pub field_path: String,
    pub value: ProjectV2CandidateValue,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", deny_unknown_fields)]
pub enum ProjectV2CandidateValue {
    Text(String),
    Integer(i64),
    Boolean(bool),
    SourceId(u64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectV2DocumentType {
    Book,
    Letter,
    Note,
    Diary,
    Report,
    Scroll,
    Parchment,
    Tablet,
    Inscription,
    Notice,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectV2CharmType {
    Major,
    Minor,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2CharmRank {
    pub rank: u8,
    pub points_cost: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chance: Option<ProjectV2ExactRatio>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effect: Option<ProjectV2DefinitionRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<ProjectV2CandidateField>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2ItemQuantity {
    pub item: ProjectV2DefinitionRef,
    pub quantity: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2ServiceRecipe {
    pub key: String,
    pub inputs: Vec<ProjectV2ItemQuantity>,
    pub outputs: Vec<ProjectV2ItemQuantity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fee: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub currency: Option<ProjectV2DefinitionRef>,
}

impl ProjectV2ServiceRecipe {
    fn canonicalize(&mut self) {
        self.inputs.sort();
        self.outputs.sort();
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2Resistance {
    pub damage_type: String,
    pub percent: ProjectV2ExactRatio,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2BestiaryProfile {
    pub difficulty: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub occurrence: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub kill_thresholds: Vec<u32>,
    pub charm_points: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2BosstiaryProfile {
    pub category: String,
    pub prowess_kills: u32,
    pub expertise_kills: u32,
    pub mastery_kills: u32,
    pub boss_points: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2FamiliarProfile {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vocation: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summon_ability: Option<ProjectV2DefinitionRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_seconds: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mana_cost: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner_speed_bonus: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2CreatureAuthoring {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub health: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub experience: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub speed: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub armor: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mitigation: Option<ProjectV2ExactRatio>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub resistances: Vec<ProjectV2Resistance>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub immunities: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pushable: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pushes_objects: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pass_through: Option<bool>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub abilities: Vec<ProjectV2DefinitionRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bestiary: Option<ProjectV2BestiaryProfile>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bosstiary: Option<ProjectV2BosstiaryProfile>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub familiar: Option<ProjectV2FamiliarProfile>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<ProjectV2CandidateField>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2AbilityAuthoring {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub incantation: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub vocations: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_level: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cooldown_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_cooldown_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub premium: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mana_cost: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_power: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub range: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub damage_type: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub acquisition_interactions: Vec<ProjectV2DefinitionRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub augments: Vec<ProjectV2AugmentBinding>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<ProjectV2CandidateField>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2QuestAuthoring {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_level: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub premium: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repeatable: Option<bool>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub prerequisites: Vec<ProjectV2DefinitionRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reward_items: Vec<ProjectV2ItemQuantity>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reward_achievements: Vec<ProjectV2DefinitionRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub encounters: Vec<ProjectV2DefinitionRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<ProjectV2CandidateField>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2HouseAuthoring {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub area: Option<ProjectV2DefinitionRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size_sqm: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rent_amount: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rent_currency: Option<ProjectV2DefinitionRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub beds: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub floors: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rooms: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player_ownable: Option<bool>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub streets: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<ProjectV2CandidateField>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectV2EncounterType {
    Generic,
    Boss,
    Raid,
    WorldChange,
    MiniWorldChange,
    Arena,
    WorldQuest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectV2EncounterScope {
    World,
    Channel,
    Instance,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2EncounterAuthoring {
    pub encounter_type: ProjectV2EncounterType,
    pub scope: ProjectV2EncounterScope,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub areas: Vec<ProjectV2DefinitionRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cooldown_seconds: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repeatable: Option<bool>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub interactions: Vec<ProjectV2DefinitionRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<ProjectV2CandidateField>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2WorldObjectAuthoring {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub area: Option<ProjectV2DefinitionRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub interactions: Vec<ProjectV2DefinitionRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub transitions: Vec<ProjectV2DefinitionRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub document: Option<ProjectV2DefinitionRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<ProjectV2CandidateField>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "profile", deny_unknown_fields)]
pub enum ProjectV2AuthoringProfileData {
    Creature(ProjectV2CreatureAuthoring),
    Ability(ProjectV2AbilityAuthoring),
    Quest(ProjectV2QuestAuthoring),
    House(ProjectV2HouseAuthoring),
    Encounter(ProjectV2EncounterAuthoring),
    WorldObject(ProjectV2WorldObjectAuthoring),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2AuthoringProfile {
    pub target: ProjectV2DefinitionRef,
    pub data: ProjectV2AuthoringProfileData,
}

impl ProjectV2AuthoringProfile {
    fn canonicalize(&mut self) {
        match &mut self.data {
            ProjectV2AuthoringProfileData::Creature(profile) => {
                profile
                    .resistances
                    .sort_by(|left, right| left.damage_type.cmp(&right.damage_type));
                profile.immunities.sort();
                profile.abilities.sort();
                if let Some(bestiary) = &mut profile.bestiary {
                    bestiary.kill_thresholds.sort();
                }
                profile
                    .fields
                    .sort_by(|left, right| left.field_path.cmp(&right.field_path));
            }
            ProjectV2AuthoringProfileData::Ability(profile) => {
                profile.vocations.sort();
                profile.acquisition_interactions.sort();
                profile
                    .augments
                    .sort_by(|left, right| left.key.cmp(&right.key));
                for augment in &mut profile.augments {
                    augment.canonicalize();
                }
                profile
                    .fields
                    .sort_by(|left, right| left.field_path.cmp(&right.field_path));
            }
            ProjectV2AuthoringProfileData::Quest(profile) => {
                profile.prerequisites.sort();
                profile.reward_items.sort();
                profile.reward_achievements.sort();
                profile.encounters.sort();
                profile
                    .fields
                    .sort_by(|left, right| left.field_path.cmp(&right.field_path));
            }
            ProjectV2AuthoringProfileData::House(profile) => {
                profile.streets.sort();
                profile
                    .fields
                    .sort_by(|left, right| left.field_path.cmp(&right.field_path));
            }
            ProjectV2AuthoringProfileData::Encounter(profile) => {
                profile.areas.sort();
                profile.interactions.sort();
                profile
                    .fields
                    .sort_by(|left, right| left.field_path.cmp(&right.field_path));
            }
            ProjectV2AuthoringProfileData::WorldObject(profile) => {
                profile.interactions.sort();
                profile.transitions.sort();
                profile
                    .fields
                    .sort_by(|left, right| left.field_path.cmp(&right.field_path));
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2ItemTaxonomy {
    pub primary: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secondary: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tertiary: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2ItemForgeProfile {
    pub classification: u8,
    pub max_tier: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2ItemConsumableProfile {
    pub edible: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub regeneration_seconds: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", deny_unknown_fields)]
pub enum ProjectV2ItemDamageObservation {
    Integer(i64),
    Range { min: i64, max: i64 },
    Text(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2ItemUseObservation {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub damage: Option<ProjectV2ItemDamageObservation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub damage_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mana_cost: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2ItemLifecycle {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enchantable: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub destructible: Option<bool>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub enchant_interactions: Vec<ProjectV2DefinitionRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub destroy_interactions: Vec<ProjectV2DefinitionRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2ItemSourceLifecycle {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub implemented: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub removed: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2ExactRatio {
    pub numerator: i64,
    pub denominator: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", deny_unknown_fields)]
pub enum ProjectV2AugmentValue {
    Boolean(bool),
    SignedPoints(i64),
    RationalPercent(ProjectV2ExactRatio),
    Milliseconds(u64),
    Cells(u16),
    Count(u32),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2AugmentRankValue {
    pub rank: u8,
    pub value: ProjectV2AugmentValue,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum ProjectV2AugmentTarget {
    Ability { ability: ProjectV2DefinitionRef },
    AutoAttack,
    OffensiveRune,
    CreatureClass { class_key: String },
    Generic { target_key: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2AugmentBinding {
    pub key: String,
    pub target: ProjectV2AugmentTarget,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effect: Option<ProjectV2DefinitionRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rank_values: Vec<ProjectV2AugmentRankValue>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<ProjectV2CandidateField>,
}

impl ProjectV2AugmentBinding {
    fn canonicalize(&mut self) {
        self.rank_values.sort_by_key(|value| value.rank);
        self.fields
            .sort_by(|left, right| left.field_path.cmp(&right.field_path));
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2ProficiencyLevel {
    pub level: u8,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub perks: Vec<ProjectV2AugmentBinding>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2PerkShaping {
    pub max_rank: u8,
    pub replace_slots: u8,
    pub refine_enabled: bool,
    pub reshape_enabled: bool,
    pub clear_enabled: bool,
    pub lunar_ascension_enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_service: Option<ProjectV2DefinitionRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2WeaponProficiencyProfile {
    pub levels: Vec<ProjectV2ProficiencyLevel>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shaping: Option<ProjectV2PerkShaping>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2ItemAuthoring {
    pub item: ProjectV2DefinitionRef,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub presentation: Option<ProjectV2DefinitionRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub document: Option<ProjectV2DefinitionRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub taxonomy: Option<ProjectV2ItemTaxonomy>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub forge: Option<ProjectV2ItemForgeProfile>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proficiency: Option<ProjectV2WeaponProficiencyProfile>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub augments: Vec<ProjectV2AugmentBinding>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub on_use_interactions: Vec<ProjectV2DefinitionRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_ability: Option<ProjectV2DefinitionRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_magic_level: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub consumable: Option<ProjectV2ItemConsumableProfile>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_observation: Option<ProjectV2ItemUseObservation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lifecycle: Option<ProjectV2ItemLifecycle>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_lifecycle: Option<ProjectV2ItemSourceLifecycle>,
}

impl ProjectV2ItemAuthoring {
    fn canonicalize(&mut self) {
        self.augments
            .sort_by(|left, right| left.key.cmp(&right.key));
        for augment in &mut self.augments {
            augment.canonicalize();
        }
        self.on_use_interactions.sort();
        if let Some(lifecycle) = &mut self.lifecycle {
            lifecycle.enchant_interactions.sort();
            lifecycle.destroy_interactions.sort();
        }
        if let Some(proficiency) = &mut self.proficiency {
            proficiency.levels.sort_by_key(|level| level.level);
            for level in &mut proficiency.levels {
                level.perks.sort_by(|left, right| left.key.cmp(&right.key));
                for perk in &mut level.perks {
                    perk.canonicalize();
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ProjectV2ServiceOfferDirection {
    BuyFromPlayer,
    SellToPlayer,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2ServiceOffer {
    pub item: ProjectV2DefinitionRef,
    pub direction: ProjectV2ServiceOfferDirection,
    pub unit_price: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub currency: Option<ProjectV2DefinitionRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2Bounds {
    pub min_x: i64,
    pub min_y: i64,
    pub max_x_exclusive: i64,
    pub max_y_exclusive: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2World {
    pub key: String,
    pub world_id: String,
    pub coordinate_frame: String,
    pub bounds: ProjectV2Bounds,
    pub floors: Vec<i16>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectV2Disposition {
    CandidateOnly,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2Placement {
    pub key: String,
    pub world: String,
    pub map_revision: String,
    pub definition: ProjectV2DefinitionRef,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub area: Option<ProjectV2DefinitionRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub document: Option<ProjectV2DefinitionRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_placement: Option<String>,
    pub coordinate_frame: String,
    pub x: i32,
    pub y: i32,
    pub floor: i16,
    pub presentation_order: ProjectV2PresentationOrder,
    pub disposition: ProjectV2Disposition,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2PresentationOrder {
    pub plane: i32,
    pub order: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2AssetRef {
    pub key: String,
    pub revision: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2Asset {
    pub identity: ProjectV2AssetRef,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2AppearanceBinding {
    pub presentation: ProjectV2DefinitionRef,
    pub asset: ProjectV2AssetRef,
    pub disposition: ProjectV2Disposition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectV2EvidenceClass {
    Proven,
    Derived,
    Unknown,
    Conflict,
    OtsHypothesisOnly,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2Source {
    pub key: String,
    /// Binds this authoring source to the complete v1 provenance/reimport batch.
    pub import_batch_id: String,
    pub revision: String,
    pub sha256: String,
    pub evidence: ProjectV2EvidenceClass,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2EditorEntry {
    pub target: ProjectV2DefinitionRef,
    pub display_name: String,
    pub description: String,
    pub categories: Vec<String>,
    pub notes: Vec<String>,
    pub aliases: Vec<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProjectV2State {
    pub declarations: Vec<ProjectV2Declaration>,
    pub item_authoring: Vec<ProjectV2ItemAuthoring>,
    pub authoring_profiles: Vec<ProjectV2AuthoringProfile>,
    pub worlds: Vec<ProjectV2World>,
    pub placements: Vec<ProjectV2Placement>,
    pub appearance_bindings: Vec<ProjectV2AppearanceBinding>,
    pub assets: Vec<ProjectV2Asset>,
    pub sources: Vec<ProjectV2Source>,
    pub editor: Vec<ProjectV2EditorEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectV2Draft {
    pub core: ProjectDraft,
    pub state: ProjectV2State,
}

impl ProjectV2Draft {
    pub fn from_project(project: &WorldProject) -> Self {
        Self {
            core: ProjectDraft {
                project_revision: project.root.project_revision.clone(),
                package_key: project.manifest.package_key.clone(),
                semantic_schema_version: project.manifest.semantic_schema_version.clone(),
                licensing_metadata: project.manifest.licensing_metadata.clone(),
                world_id: project.reference.world_id.clone(),
                coordinate_frame: project.reference.coordinate_frame.clone(),
                records: project.reference.records.clone(),
                imports: project.imports.batches.clone(),
                metadata: project.metadata.entries.clone(),
            },
            state: project.v2.clone().unwrap_or_default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DeclarationsDocument {
    schema: String,
    records: Vec<ProjectV2Declaration>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    item_authoring: Vec<ProjectV2ItemAuthoring>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    authoring_profiles: Vec<ProjectV2AuthoringProfile>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct WorldsDocument {
    schema: String,
    worlds: Vec<ProjectV2World>,
    placements: Vec<ProjectV2Placement>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct BindingsDocument {
    schema: String,
    bindings: Vec<ProjectV2AppearanceBinding>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AssetsDocument {
    schema: String,
    assets: Vec<ProjectV2Asset>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourcesDocument {
    schema: String,
    sources: Vec<ProjectV2Source>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct EditorDocument {
    schema: String,
    legacy_entries: Vec<AuthorMetadataEntry>,
    entries: Vec<ProjectV2EditorEntry>,
}

pub(super) fn validate_v2_roles(
    roles: &BTreeMap<String, Vec<&ManifestDocument>>,
) -> Result<(), ProjectError> {
    if roles.len() != ROLE_SPECS.len() {
        return Err(ProjectError::InvalidProject(
            "unsupported v2 manifest role set",
        ));
    }
    for (role, locator, schema) in ROLE_SPECS {
        let prefix = locator
            .split_once('/')
            .ok_or(ProjectError::InvalidProject(
                "v2 role locator missing directory",
            ))?
            .0;
        let prefix = format!("{prefix}/");
        let found = require_role(roles, role, &prefix, schema)?;
        if found.len() != 1 {
            return Err(ProjectError::InvalidProject(
                "v2 role requires exactly one document",
            ));
        }
    }
    Ok(())
}

fn role_bytes<'a>(
    snapshot: &'a ProjectSnapshot,
    plan: &ProjectCapturePlan,
    role: &str,
) -> Result<&'a [u8], ProjectError> {
    let info = plan
        .manifest
        .documents
        .iter()
        .find(|entry| entry.role == role)
        .ok_or(ProjectError::InvalidProject("v2 role missing"))?;
    required(snapshot, &info.locator)
}

pub(super) fn parse_v2_snapshot(
    snapshot: &ProjectSnapshot,
    limits: ProjectEvidenceLimits,
    plan: ProjectCapturePlan,
    manifest_bytes: &[u8],
) -> Result<WorldProject, ProjectError> {
    let reference: ReferenceDocument =
        parse_strict(role_bytes(snapshot, &plan, "reference-records")?, limits)?;
    let imports: ImportDocument =
        parse_strict(role_bytes(snapshot, &plan, "import-candidates")?, limits)?;
    let declarations: DeclarationsDocument = parse_strict(
        role_bytes(snapshot, &plan, "declarative-definitions")?,
        limits,
    )?;
    let worlds: WorldsDocument =
        parse_strict(role_bytes(snapshot, &plan, "world-records")?, limits)?;
    let bindings: BindingsDocument = parse_strict(
        role_bytes(snapshot, &plan, "presentation-bindings")?,
        limits,
    )?;
    let assets: AssetsDocument =
        parse_strict(role_bytes(snapshot, &plan, "asset-records")?, limits)?;
    let sources: SourcesDocument =
        parse_strict(role_bytes(snapshot, &plan, "provenance-records")?, limits)?;
    let editor: EditorDocument =
        parse_strict(role_bytes(snapshot, &plan, "editor-records")?, limits)?;
    for (actual, expected) in [
        (&reference.schema, WORLD_PROJECT_REFERENCE_SCHEMA),
        (&imports.schema, WORLD_PROJECT_IMPORT_SCHEMA),
        (&declarations.schema, DECLARATIONS_SCHEMA),
        (&worlds.schema, WORLDS_SCHEMA),
        (&bindings.schema, PRESENTATIONS_SCHEMA),
        (&assets.schema, ASSETS_SCHEMA),
        (&sources.schema, PROVENANCE_SCHEMA),
        (&editor.schema, EDITOR_SCHEMA),
    ] {
        if actual != expected {
            return Err(ProjectError::InvalidProject(
                "v2 managed document schema mismatch",
            ));
        }
    }
    ProjectDraft {
        project_revision: plan.root.project_revision.clone(),
        package_key: plan.manifest.package_key.clone(),
        semantic_schema_version: plan.manifest.semantic_schema_version.clone(),
        licensing_metadata: plan.manifest.licensing_metadata.clone(),
        world_id: reference.world_id.clone(),
        coordinate_frame: reference.coordinate_frame.clone(),
        records: reference.records.clone(),
        imports: imports.batches.clone(),
        metadata: editor.legacy_entries.clone(),
    }
    .validate(limits)?;
    let state = ProjectV2State {
        declarations: declarations.records,
        item_authoring: declarations.item_authoring,
        authoring_profiles: declarations.authoring_profiles,
        worlds: worlds.worlds,
        placements: worlds.placements,
        appearance_bindings: bindings.bindings,
        assets: assets.assets,
        sources: sources.sources,
        editor: editor.entries,
    };
    validate_v2_state(&state, &reference.records, &imports.batches, limits)?;
    Ok(WorldProject {
        root: plan.root,
        manifest: plan.manifest,
        lock: plan.lock,
        reference,
        imports,
        metadata: MetadataDocument {
            schema: WORLD_PROJECT_METADATA_SCHEMA.to_owned(),
            entries: editor.legacy_entries,
        },
        manifest_bytes: manifest_bytes.to_vec(),
        v2: Some(state),
    })
}

fn validate_v2_source_text(
    field: &'static str,
    value: &str,
    limits: ProjectEvidenceLimits,
) -> Result<(), ProjectError> {
    limits.check(field, value.len(), limits.max_string_bytes)?;
    if value.trim() != value || value.is_empty() || value.chars().any(char::is_control) {
        return Err(ProjectError::InvalidProject("invalid v2 source text"));
    }
    Ok(())
}

fn validate_v2_candidate_fields(fields: &[ProjectV2CandidateField]) -> Result<(), ProjectError> {
    let mut previous: Option<&str> = None;
    for field in fields {
        ProductionKey::new(&field.field_path)?;
        if previous.is_some_and(|prior| prior >= field.field_path.as_str()) {
            return Err(ProjectError::InvalidProject(
                "v2 candidate fields are not sorted and unique",
            ));
        }
        previous = Some(&field.field_path);
    }
    Ok(())
}

fn gcd_v2(mut left: u64, mut right: u64) -> u64 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left
}

fn validate_v2_augment(
    augment: &ProjectV2AugmentBinding,
    require_ref: &impl Fn(&ProjectV2DefinitionRef) -> Result<(), ProjectError>,
    limits: ProjectEvidenceLimits,
) -> Result<(), ProjectError> {
    ProductionKey::new(&augment.key)?;
    match &augment.target {
        ProjectV2AugmentTarget::Ability { ability } => {
            if ability.family != ProjectV2Family::Ability {
                return Err(ProjectError::InvalidProject(
                    "v2 augment Ability target family mismatch",
                ));
            }
            require_ref(ability)?;
        }
        ProjectV2AugmentTarget::CreatureClass { class_key } => {
            ProductionKey::new(class_key)?;
        }
        ProjectV2AugmentTarget::Generic { target_key } => {
            ProductionKey::new(target_key)?;
        }
        ProjectV2AugmentTarget::AutoAttack | ProjectV2AugmentTarget::OffensiveRune => {}
    }
    if let Some(effect) = &augment.effect {
        if effect.family != ProjectV2Family::Effect {
            return Err(ProjectError::InvalidProject(
                "v2 augment effect family mismatch",
            ));
        }
        require_ref(effect)?;
    }
    limits.check(
        "v2 augment rank values",
        augment.rank_values.len(),
        limits.max_reference_records,
    )?;
    if augment
        .rank_values
        .windows(2)
        .any(|pair| pair[0].rank >= pair[1].rank)
    {
        return Err(ProjectError::InvalidProject(
            "v2 augment ranks are not sorted and unique",
        ));
    }
    for rank in &augment.rank_values {
        if let ProjectV2AugmentValue::RationalPercent(ratio) = rank.value
            && (ratio.denominator == 0
                || gcd_v2(ratio.numerator.unsigned_abs(), ratio.denominator) != 1)
        {
            return Err(ProjectError::InvalidProject(
                "v2 augment rational percent is not canonical",
            ));
        }
    }
    limits.check(
        "v2 augment candidate fields",
        augment.fields.len(),
        limits.max_reference_records,
    )?;
    validate_v2_candidate_fields(&augment.fields)
}

fn validate_v2_ratio(ratio: ProjectV2ExactRatio, error: &'static str) -> Result<(), ProjectError> {
    if ratio.denominator == 0 || gcd_v2(ratio.numerator.unsigned_abs(), ratio.denominator) != 1 {
        return Err(ProjectError::InvalidProject(error));
    }
    Ok(())
}

fn validate_v2_ref_list(
    values: &[ProjectV2DefinitionRef],
    family: ProjectV2Family,
    label: &'static str,
    error: &'static str,
    require_ref: &impl Fn(&ProjectV2DefinitionRef) -> Result<(), ProjectError>,
    limits: ProjectEvidenceLimits,
) -> Result<(), ProjectError> {
    limits.check(label, values.len(), limits.max_reference_records)?;
    if values.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(ProjectError::InvalidProject(error));
    }
    for reference in values {
        if reference.family != family {
            return Err(ProjectError::InvalidProject(error));
        }
        require_ref(reference)?;
    }
    Ok(())
}

fn validate_v2_item_quantities(
    values: &[ProjectV2ItemQuantity],
    label: &'static str,
    require_ref: &impl Fn(&ProjectV2DefinitionRef) -> Result<(), ProjectError>,
    limits: ProjectEvidenceLimits,
) -> Result<(), ProjectError> {
    limits.check(label, values.len(), limits.max_reference_records)?;
    if values.is_empty() || values.windows(2).any(|pair| pair[0].item >= pair[1].item) {
        return Err(ProjectError::InvalidProject(
            "v2 Item quantities are empty, duplicated or unsorted",
        ));
    }
    for value in values {
        if value.quantity == 0 || value.item.family != ProjectV2Family::Item {
            return Err(ProjectError::InvalidProject(
                "v2 Item quantity requires a positive Item reference",
            ));
        }
        require_ref(&value.item)?;
    }
    Ok(())
}

fn validate_v2_declaration(
    declaration: &ProjectV2Declaration,
    require_ref: &impl Fn(&ProjectV2DefinitionRef) -> Result<(), ProjectError>,
    limits: ProjectEvidenceLimits,
) -> Result<(), ProjectError> {
    match declaration {
        ProjectV2Declaration::Document {
            title,
            author,
            language,
            content,
            ..
        } => {
            if let Some(value) = title {
                validate_v2_source_text("v2 Document title", value, limits)?;
            }
            if let Some(value) = author {
                validate_v2_source_text("v2 Document author", value, limits)?;
            }
            if let Some(value) = language {
                validate_v2_source_text("v2 Document language", value, limits)?;
            }
            limits.check(
                "v2 Document content paragraphs",
                content.len(),
                limits.max_reference_records,
            )?;
            for paragraph in content {
                validate_v2_source_text("v2 Document paragraph", paragraph, limits)?;
            }
        }
        ProjectV2Declaration::Achievement {
            source_id,
            unlock_interactions,
            ..
        } => {
            if source_id == &Some(0) {
                return Err(ProjectError::InvalidProject(
                    "v2 Achievement source id must be positive",
                ));
            }
            validate_v2_ref_list(
                unlock_interactions,
                ProjectV2Family::Interaction,
                "v2 Achievement interactions",
                "v2 Achievement interactions are invalid",
                require_ref,
                limits,
            )?;
        }
        ProjectV2Declaration::Outfit {
            presentations,
            acquisition_interactions,
            ..
        } => {
            validate_v2_ref_list(
                presentations,
                ProjectV2Family::Presentation,
                "v2 Outfit presentations",
                "v2 Outfit presentations are invalid",
                require_ref,
                limits,
            )?;
            validate_v2_ref_list(
                acquisition_interactions,
                ProjectV2Family::Interaction,
                "v2 Outfit acquisition interactions",
                "v2 Outfit acquisition interactions are invalid",
                require_ref,
                limits,
            )?;
        }
        ProjectV2Declaration::Mount {
            acquisition_interactions,
            ..
        } => {
            validate_v2_ref_list(
                acquisition_interactions,
                ProjectV2Family::Interaction,
                "v2 Mount acquisition interactions",
                "v2 Mount acquisition interactions are invalid",
                require_ref,
                limits,
            )?;
        }
        ProjectV2Declaration::Charm { ranks, .. } => {
            limits.check("v2 Charm ranks", ranks.len(), limits.max_reference_records)?;
            if ranks
                .windows(2)
                .any(|pair| pair[0].rank == 0 || pair[0].rank >= pair[1].rank)
                || ranks.last().is_some_and(|rank| rank.rank == 0)
            {
                return Err(ProjectError::InvalidProject(
                    "v2 Charm ranks are not positive sorted and unique",
                ));
            }
            for rank in ranks {
                if let Some(chance) = rank.chance {
                    validate_v2_ratio(chance, "v2 Charm chance is not canonical")?;
                }
                validate_v2_candidate_fields(&rank.fields)?;
            }
        }
        ProjectV2Declaration::Service { recipes, .. } => {
            limits.check(
                "v2 Service recipes",
                recipes.len(),
                limits.max_reference_records,
            )?;
            if recipes.windows(2).any(|pair| pair[0].key >= pair[1].key) {
                return Err(ProjectError::InvalidProject(
                    "v2 Service recipes are not key sorted and unique",
                ));
            }
            for recipe in recipes {
                ProductionKey::new(&recipe.key)?;
                validate_v2_item_quantities(
                    &recipe.inputs,
                    "v2 Service recipe inputs",
                    require_ref,
                    limits,
                )?;
                validate_v2_item_quantities(
                    &recipe.outputs,
                    "v2 Service recipe outputs",
                    require_ref,
                    limits,
                )?;
                if let Some(currency) = &recipe.currency {
                    if currency.family != ProjectV2Family::Item {
                        return Err(ProjectError::InvalidProject(
                            "v2 Service recipe currency requires Item",
                        ));
                    }
                    require_ref(currency)?;
                }
            }
        }
        _ => {}
    }
    Ok(())
}

fn validate_v2_authoring_profile(
    profile: &ProjectV2AuthoringProfile,
    require_ref: &impl Fn(&ProjectV2DefinitionRef) -> Result<(), ProjectError>,
    limits: ProjectEvidenceLimits,
) -> Result<(), ProjectError> {
    require_ref(&profile.target)?;
    match &profile.data {
        ProjectV2AuthoringProfileData::Creature(value) => {
            if profile.target.family != ProjectV2Family::Creature {
                return Err(ProjectError::InvalidProject(
                    "v2 Creature authoring requires Creature target",
                ));
            }
            if value.health == Some(0) {
                return Err(ProjectError::InvalidProject(
                    "v2 Creature health must be positive when present",
                ));
            }
            if let Some(ratio) = value.mitigation {
                validate_v2_ratio(ratio, "v2 Creature mitigation is not canonical")?;
            }
            limits.check(
                "v2 Creature resistances",
                value.resistances.len(),
                limits.max_reference_records,
            )?;
            if value
                .resistances
                .windows(2)
                .any(|pair| pair[0].damage_type >= pair[1].damage_type)
            {
                return Err(ProjectError::InvalidProject(
                    "v2 Creature resistances are not sorted and unique",
                ));
            }
            for resistance in &value.resistances {
                validate_v2_source_text(
                    "v2 Creature resistance damage type",
                    &resistance.damage_type,
                    limits,
                )?;
                validate_v2_ratio(
                    resistance.percent,
                    "v2 Creature resistance percent is not canonical",
                )?;
            }
            if value.immunities.windows(2).any(|pair| pair[0] >= pair[1]) {
                return Err(ProjectError::InvalidProject(
                    "v2 Creature immunities are not sorted and unique",
                ));
            }
            for immunity in &value.immunities {
                validate_v2_source_text("v2 Creature immunity", immunity, limits)?;
            }
            validate_v2_ref_list(
                &value.abilities,
                ProjectV2Family::Ability,
                "v2 Creature abilities",
                "v2 Creature abilities are invalid",
                require_ref,
                limits,
            )?;
            if let Some(bestiary) = &value.bestiary {
                validate_v2_source_text("v2 Bestiary difficulty", &bestiary.difficulty, limits)?;
                if let Some(occurrence) = &bestiary.occurrence {
                    validate_v2_source_text("v2 Bestiary occurrence", occurrence, limits)?;
                }
                if bestiary
                    .kill_thresholds
                    .windows(2)
                    .any(|pair| pair[0] == 0 || pair[0] >= pair[1])
                    || bestiary
                        .kill_thresholds
                        .last()
                        .is_some_and(|value| *value == 0)
                {
                    return Err(ProjectError::InvalidProject(
                        "v2 Bestiary kill thresholds are not positive sorted and unique",
                    ));
                }
            }
            if let Some(bosstiary) = &value.bosstiary {
                validate_v2_source_text("v2 Bosstiary category", &bosstiary.category, limits)?;
                if bosstiary.prowess_kills == 0
                    || bosstiary.prowess_kills >= bosstiary.expertise_kills
                    || bosstiary.expertise_kills >= bosstiary.mastery_kills
                {
                    return Err(ProjectError::InvalidProject(
                        "v2 Bosstiary thresholds are invalid",
                    ));
                }
            }
            if let Some(familiar) = &value.familiar {
                if let Some(vocation) = &familiar.vocation {
                    validate_v2_source_text("v2 Familiar vocation", vocation, limits)?;
                }
                if let Some(ability) = &familiar.summon_ability {
                    if ability.family != ProjectV2Family::Ability {
                        return Err(ProjectError::InvalidProject(
                            "v2 Familiar summon ability family mismatch",
                        ));
                    }
                    require_ref(ability)?;
                }
                if familiar.duration_seconds == Some(0) {
                    return Err(ProjectError::InvalidProject(
                        "v2 Familiar duration must be positive when present",
                    ));
                }
            }
            validate_v2_candidate_fields(&value.fields)?;
        }
        ProjectV2AuthoringProfileData::Ability(value) => {
            if profile.target.family != ProjectV2Family::Ability {
                return Err(ProjectError::InvalidProject(
                    "v2 Ability authoring requires Ability target",
                ));
            }
            for (label, candidate) in [
                ("v2 Ability incantation", value.incantation.as_ref()),
                ("v2 Ability group", value.group.as_ref()),
                ("v2 Ability damage type", value.damage_type.as_ref()),
            ] {
                if let Some(candidate) = candidate {
                    validate_v2_source_text(label, candidate, limits)?;
                }
            }
            if value.vocations.windows(2).any(|pair| pair[0] >= pair[1]) {
                return Err(ProjectError::InvalidProject(
                    "v2 Ability vocations are not sorted and unique",
                ));
            }
            for vocation in &value.vocations {
                validate_v2_source_text("v2 Ability vocation", vocation, limits)?;
            }
            validate_v2_ref_list(
                &value.acquisition_interactions,
                ProjectV2Family::Interaction,
                "v2 Ability acquisition interactions",
                "v2 Ability acquisition interactions are invalid",
                require_ref,
                limits,
            )?;
            limits.check(
                "v2 Ability augments",
                value.augments.len(),
                limits.max_reference_records,
            )?;
            if value
                .augments
                .windows(2)
                .any(|pair| pair[0].key >= pair[1].key)
            {
                return Err(ProjectError::InvalidProject(
                    "v2 Ability augments are not sorted and unique",
                ));
            }
            for augment in &value.augments {
                validate_v2_augment(augment, require_ref, limits)?;
            }
            validate_v2_candidate_fields(&value.fields)?;
        }
        ProjectV2AuthoringProfileData::Quest(value) => {
            if profile.target.family != ProjectV2Family::Quest {
                return Err(ProjectError::InvalidProject(
                    "v2 Quest authoring requires Quest target",
                ));
            }
            validate_v2_ref_list(
                &value.prerequisites,
                ProjectV2Family::Quest,
                "v2 Quest prerequisites",
                "v2 Quest prerequisites are invalid",
                require_ref,
                limits,
            )?;
            if !value.reward_items.is_empty() {
                validate_v2_item_quantities(
                    &value.reward_items,
                    "v2 Quest reward items",
                    require_ref,
                    limits,
                )?;
            }
            validate_v2_ref_list(
                &value.reward_achievements,
                ProjectV2Family::Achievement,
                "v2 Quest reward achievements",
                "v2 Quest reward achievements are invalid",
                require_ref,
                limits,
            )?;
            validate_v2_ref_list(
                &value.encounters,
                ProjectV2Family::Encounter,
                "v2 Quest encounters",
                "v2 Quest encounters are invalid",
                require_ref,
                limits,
            )?;
            validate_v2_candidate_fields(&value.fields)?;
        }
        ProjectV2AuthoringProfileData::House(value) => {
            if profile.target.family != ProjectV2Family::House {
                return Err(ProjectError::InvalidProject(
                    "v2 House authoring requires House target",
                ));
            }
            if let Some(area) = &value.area {
                if area.family != ProjectV2Family::Area {
                    return Err(ProjectError::InvalidProject(
                        "v2 House area family mismatch",
                    ));
                }
                require_ref(area)?;
            }
            if let Some(currency) = &value.rent_currency {
                if currency.family != ProjectV2Family::Item {
                    return Err(ProjectError::InvalidProject(
                        "v2 House rent currency requires Item",
                    ));
                }
                require_ref(currency)?;
            }
            if value.streets.windows(2).any(|pair| pair[0] >= pair[1]) {
                return Err(ProjectError::InvalidProject(
                    "v2 House streets are not sorted and unique",
                ));
            }
            for street in &value.streets {
                validate_v2_source_text("v2 House street", street, limits)?;
            }
            validate_v2_candidate_fields(&value.fields)?;
        }
        ProjectV2AuthoringProfileData::Encounter(value) => {
            if profile.target.family != ProjectV2Family::Encounter {
                return Err(ProjectError::InvalidProject(
                    "v2 Encounter authoring requires Encounter target",
                ));
            }
            validate_v2_ref_list(
                &value.areas,
                ProjectV2Family::Area,
                "v2 Encounter areas",
                "v2 Encounter areas are invalid",
                require_ref,
                limits,
            )?;
            validate_v2_ref_list(
                &value.interactions,
                ProjectV2Family::Interaction,
                "v2 Encounter interactions",
                "v2 Encounter interactions are invalid",
                require_ref,
                limits,
            )?;
            validate_v2_candidate_fields(&value.fields)?;
        }
        ProjectV2AuthoringProfileData::WorldObject(value) => {
            if profile.target.family != ProjectV2Family::WorldObject {
                return Err(ProjectError::InvalidProject(
                    "v2 WorldObject authoring requires WorldObject target",
                ));
            }
            if let Some(area) = &value.area {
                if area.family != ProjectV2Family::Area {
                    return Err(ProjectError::InvalidProject(
                        "v2 WorldObject area family mismatch",
                    ));
                }
                require_ref(area)?;
            }
            validate_v2_ref_list(
                &value.interactions,
                ProjectV2Family::Interaction,
                "v2 WorldObject interactions",
                "v2 WorldObject interactions are invalid",
                require_ref,
                limits,
            )?;
            validate_v2_ref_list(
                &value.transitions,
                ProjectV2Family::Transition,
                "v2 WorldObject transitions",
                "v2 WorldObject transitions are invalid",
                require_ref,
                limits,
            )?;
            if let Some(document) = &value.document {
                if document.family != ProjectV2Family::Document {
                    return Err(ProjectError::InvalidProject(
                        "v2 WorldObject document family mismatch",
                    ));
                }
                require_ref(document)?;
            }
            validate_v2_candidate_fields(&value.fields)?;
        }
    }
    Ok(())
}

fn validate_v2_state(
    state: &ProjectV2State,
    records: &[ProjectReferenceRecord],
    imports: &[ImportBatch],
    limits: ProjectEvidenceLimits,
) -> Result<(), ProjectError> {
    let mut identities = BTreeSet::new();
    for record in records {
        let identity = record.identity();
        identities.insert(ProjectV2DefinitionRef {
            family: ProjectV2Family::from_reference(&identity.family)?,
            key: identity.key.clone(),
            revision: identity.revision.clone(),
        });
    }
    limits.check(
        "v2 declarations",
        state.declarations.len(),
        limits.max_reference_records,
    )?;
    let mut previous: Option<(ProjectV2Family, &str)> = None;
    for declaration in &state.declarations {
        let identity = declaration.identity();
        identity.validate()?;
        validate_v2_candidate_fields(declaration.fields())?;
        let key = (declaration.family(), identity.key.as_str());
        if previous.is_some_and(|prior| prior >= key) {
            return Err(ProjectError::InvalidProject(
                "v2 declarations are not identity sorted",
            ));
        }
        previous = Some(key);
        if !identities.insert(ProjectV2DefinitionRef {
            family: key.0,
            key: identity.key.clone(),
            revision: identity.revision.clone(),
        }) {
            return Err(ProjectError::InvalidProject(
                "duplicate v2 definition identity",
            ));
        }
    }
    let require_ref = |reference: &ProjectV2DefinitionRef| -> Result<(), ProjectError> {
        reference.validate()?;
        if !identities.contains(reference) {
            return Err(ProjectError::InvalidProject(
                "unresolved v2 typed definition reference",
            ));
        }
        Ok(())
    };
    for declaration in &state.declarations {
        for (family, reference) in declaration.references() {
            if reference.family != family {
                return Err(ProjectError::InvalidProject(
                    "v2 definition reference family mismatch",
                ));
            }
            require_ref(reference)?;
        }
        validate_v2_declaration(declaration, &require_ref, limits)?;
    }

    limits.check(
        "v2 authoring profiles",
        state.authoring_profiles.len(),
        limits.max_reference_records,
    )?;
    if state
        .authoring_profiles
        .windows(2)
        .any(|pair| pair[0].target >= pair[1].target)
    {
        return Err(ProjectError::InvalidProject(
            "v2 authoring profiles are not target sorted and unique",
        ));
    }
    for profile in &state.authoring_profiles {
        validate_v2_authoring_profile(profile, &require_ref, limits)?;
    }

    limits.check(
        "v2 item authoring",
        state.item_authoring.len(),
        limits.max_reference_records,
    )?;
    if state
        .item_authoring
        .windows(2)
        .any(|pair| pair[0].item >= pair[1].item)
    {
        return Err(ProjectError::InvalidProject(
            "v2 Item authoring is not identity sorted and unique",
        ));
    }
    for item in &state.item_authoring {
        if item.item.family != ProjectV2Family::Item {
            return Err(ProjectError::InvalidProject(
                "v2 Item authoring requires Item target",
            ));
        }
        require_ref(&item.item)?;
        if let Some(presentation) = &item.presentation {
            if presentation.family != ProjectV2Family::Presentation {
                return Err(ProjectError::InvalidProject(
                    "v2 Item presentation family mismatch",
                ));
            }
            require_ref(presentation)?;
        }
        if let Some(document) = &item.document {
            if document.family != ProjectV2Family::Document {
                return Err(ProjectError::InvalidProject(
                    "v2 Item document family mismatch",
                ));
            }
            require_ref(document)?;
        }
        if let Some(taxonomy) = &item.taxonomy {
            validate_v2_source_text("v2 Item primary taxonomy", &taxonomy.primary, limits)?;
            if let Some(value) = &taxonomy.secondary {
                validate_v2_source_text("v2 Item secondary taxonomy", value, limits)?;
            }
            if let Some(value) = &taxonomy.tertiary {
                validate_v2_source_text("v2 Item tertiary taxonomy", value, limits)?;
            }
        }
        if let Some(forge) = item.forge
            && (forge.classification == 0 || forge.max_tier == 0)
        {
            return Err(ProjectError::InvalidProject(
                "v2 Item Forge profile requires nonzero class and max tier",
            ));
        }
        if let Some(ability) = &item.use_ability {
            if ability.family != ProjectV2Family::Ability {
                return Err(ProjectError::InvalidProject(
                    "v2 Item use Ability family mismatch",
                ));
            }
            require_ref(ability)?;
        }
        if let Some(consumable) = item.consumable
            && consumable.regeneration_seconds == Some(0)
        {
            return Err(ProjectError::InvalidProject(
                "v2 Item regeneration seconds must be positive when present",
            ));
        }
        if let Some(observation) = &item.use_observation {
            if let Some(damage) = &observation.damage {
                match damage {
                    ProjectV2ItemDamageObservation::Range { min, max } if min > max => {
                        return Err(ProjectError::InvalidProject(
                            "v2 Item damage observation range is inverted",
                        ));
                    }
                    ProjectV2ItemDamageObservation::Text(value) => {
                        validate_v2_source_text("v2 Item damage source text", value, limits)?;
                    }
                    ProjectV2ItemDamageObservation::Integer(_)
                    | ProjectV2ItemDamageObservation::Range { .. } => {}
                }
            }
            if let Some(value) = &observation.damage_type {
                validate_v2_source_text("v2 Item damage type source text", value, limits)?;
            }
        }
        limits.check(
            "v2 Item augments",
            item.augments.len(),
            limits.max_reference_records,
        )?;
        if item
            .augments
            .windows(2)
            .any(|pair| pair[0].key >= pair[1].key)
        {
            return Err(ProjectError::InvalidProject(
                "v2 Item augments are not key sorted and unique",
            ));
        }
        for augment in &item.augments {
            validate_v2_augment(augment, &require_ref, limits)?;
        }
        if let Some(proficiency) = &item.proficiency {
            if proficiency.levels.is_empty() {
                return Err(ProjectError::InvalidProject(
                    "v2 Item proficiency requires at least one level",
                ));
            }
            limits.check(
                "v2 Item proficiency levels",
                proficiency.levels.len(),
                limits.max_reference_records,
            )?;
            if proficiency
                .levels
                .windows(2)
                .any(|pair| pair[0].level == 0 || pair[0].level >= pair[1].level)
                || proficiency
                    .levels
                    .last()
                    .is_some_and(|level| level.level == 0)
            {
                return Err(ProjectError::InvalidProject(
                    "v2 Item proficiency levels are not positive sorted and unique",
                ));
            }
            for level in &proficiency.levels {
                limits.check(
                    "v2 Item proficiency perks",
                    level.perks.len(),
                    limits.max_reference_records,
                )?;
                if level
                    .perks
                    .windows(2)
                    .any(|pair| pair[0].key >= pair[1].key)
                {
                    return Err(ProjectError::InvalidProject(
                        "v2 Item proficiency perks are not key sorted and unique",
                    ));
                }
                for perk in &level.perks {
                    validate_v2_augment(perk, &require_ref, limits)?;
                }
            }
            if let Some(shaping) = &proficiency.shaping {
                if shaping.max_rank == 0 {
                    return Err(ProjectError::InvalidProject(
                        "v2 Item perk shaping max rank must be positive",
                    ));
                }
                if let Some(service) = &shaping.cost_service {
                    if service.family != ProjectV2Family::Service {
                        return Err(ProjectError::InvalidProject(
                            "v2 Item perk shaping Service family mismatch",
                        ));
                    }
                    require_ref(service)?;
                }
            }
        }
        limits.check(
            "v2 Item on-use interactions",
            item.on_use_interactions.len(),
            limits.max_reference_records,
        )?;
        if item
            .on_use_interactions
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
        {
            return Err(ProjectError::InvalidProject(
                "v2 Item on-use interactions are not sorted and unique",
            ));
        }
        for interaction in &item.on_use_interactions {
            if interaction.family != ProjectV2Family::Interaction {
                return Err(ProjectError::InvalidProject(
                    "v2 Item on-use Interaction family mismatch",
                ));
            }
            require_ref(interaction)?;
        }
        if let Some(lifecycle) = &item.lifecycle {
            for interactions in [
                &lifecycle.enchant_interactions,
                &lifecycle.destroy_interactions,
            ] {
                limits.check(
                    "v2 Item lifecycle interactions",
                    interactions.len(),
                    limits.max_reference_records,
                )?;
                if interactions.windows(2).any(|pair| pair[0] >= pair[1]) {
                    return Err(ProjectError::InvalidProject(
                        "v2 Item lifecycle interactions are not sorted and unique",
                    ));
                }
                for interaction in interactions {
                    if interaction.family != ProjectV2Family::Interaction {
                        return Err(ProjectError::InvalidProject(
                            "v2 Item lifecycle Interaction family mismatch",
                        ));
                    }
                    require_ref(interaction)?;
                }
            }
        }
        if let Some(source_lifecycle) = &item.source_lifecycle {
            if let Some(value) = &source_lifecycle.implemented {
                validate_v2_source_text("v2 Item implemented source text", value, limits)?;
            }
            if let Some(value) = &source_lifecycle.removed {
                validate_v2_source_text("v2 Item removed source text", value, limits)?;
            }
        }
    }
    limits.check(
        "v2 worlds",
        state.worlds.len(),
        limits.max_reference_records,
    )?;
    let mut worlds = BTreeMap::new();
    let mut world_ids = BTreeSet::new();
    for world in &state.worlds {
        ProductionKey::new(&world.key)?;
        decode_world_id(&world.world_id)?;
        super::super::CoordinateFrameRef::new(&world.coordinate_frame)?;
        let bounds = &world.bounds;
        if bounds.min_x < i64::from(i32::MIN)
            || bounds.min_y < i64::from(i32::MIN)
            || bounds.max_x_exclusive > i64::from(i32::MAX) + 1
            || bounds.max_y_exclusive > i64::from(i32::MAX) + 1
            || bounds.min_x >= bounds.max_x_exclusive
            || bounds.min_y >= bounds.max_y_exclusive
            || world.floors.is_empty()
            || world.floors.windows(2).any(|pair| pair[0] >= pair[1])
        {
            return Err(ProjectError::InvalidProject(
                "invalid v2 world bounds or floors",
            ));
        }
        limits.check(
            "v2 world floors",
            world.floors.len(),
            limits.max_reference_records,
        )?;
        if worlds.insert(&world.key, world).is_some() || !world_ids.insert(&world.world_id) {
            return Err(ProjectError::InvalidProject("duplicate v2 world identity"));
        }
    }
    if state
        .worlds
        .windows(2)
        .any(|pair| pair[0].key >= pair[1].key)
    {
        return Err(ProjectError::InvalidProject(
            "v2 worlds are not identity sorted",
        ));
    }
    limits.check(
        "v2 placements",
        state.placements.len(),
        limits.max_reference_records,
    )?;
    let mut keys = BTreeSet::new();
    let mut orders = BTreeSet::new();
    for placement in &state.placements {
        ProductionKey::new(&placement.key)?;
        ProductionAtom::new("v2 map revision", &placement.map_revision)?;
        super::super::CoordinateFrameRef::new(&placement.coordinate_frame)?;
        require_ref(&placement.definition)?;
        if let Some(area) = &placement.area {
            if area.family != ProjectV2Family::Area {
                return Err(ProjectError::InvalidProject(
                    "v2 placement area family mismatch",
                ));
            }
            require_ref(area)?;
        }
        if let Some(document) = &placement.document {
            if document.family != ProjectV2Family::Document {
                return Err(ProjectError::InvalidProject(
                    "v2 placement document family mismatch",
                ));
            }
            require_ref(document)?;
        }
        if let Some(parent) = &placement.parent_placement {
            ProductionKey::new(parent)?;
            if parent == &placement.key {
                return Err(ProjectError::InvalidProject(
                    "v2 placement cannot parent itself",
                ));
            }
        }
        let world = worlds
            .get(&placement.world)
            .ok_or(ProjectError::InvalidProject(
                "v2 placement world is missing",
            ))?;
        if world.coordinate_frame != placement.coordinate_frame
            || i64::from(placement.x) < world.bounds.min_x
            || i64::from(placement.x) >= world.bounds.max_x_exclusive
            || i64::from(placement.y) < world.bounds.min_y
            || i64::from(placement.y) >= world.bounds.max_y_exclusive
            || world.floors.binary_search(&placement.floor).is_err()
        {
            return Err(ProjectError::InvalidProject(
                "v2 placement world/frame/position mismatch",
            ));
        }
        if !orders.insert((
            &placement.world,
            placement.x,
            placement.y,
            placement.floor,
            &placement.presentation_order,
        )) {
            return Err(ProjectError::InvalidProject(
                "duplicate v2 presentation order at tile",
            ));
        }
        if !keys.insert(&placement.key) {
            return Err(ProjectError::InvalidProject(
                "duplicate v2 placement identity",
            ));
        }
    }
    if state
        .placements
        .windows(2)
        .any(|pair| pair[0].key >= pair[1].key)
    {
        return Err(ProjectError::InvalidProject(
            "v2 placements are not identity sorted",
        ));
    }
    for placement in &state.placements {
        if let Some(parent_key) = &placement.parent_placement {
            let parent = state
                .placements
                .iter()
                .find(|candidate| &candidate.key == parent_key)
                .ok_or(ProjectError::InvalidProject(
                    "v2 parent placement is missing",
                ))?;
            if parent.world != placement.world || parent.map_revision != placement.map_revision {
                return Err(ProjectError::InvalidProject(
                    "v2 parent placement world or map revision mismatch",
                ));
            }
        }
    }
    for placement in &state.placements {
        let mut seen = BTreeSet::new();
        let mut current = Some(placement);
        while let Some(node) = current {
            if !seen.insert(node.key.as_str()) {
                return Err(ProjectError::InvalidProject(
                    "v2 placement parent chain contains a cycle",
                ));
            }
            current = node.parent_placement.as_ref().and_then(|parent_key| {
                state
                    .placements
                    .iter()
                    .find(|candidate| &candidate.key == parent_key)
            });
        }
    }
    limits.check(
        "v2 assets",
        state.assets.len(),
        limits.max_reference_records,
    )?;
    let mut assets = BTreeSet::new();
    for asset in &state.assets {
        ProductionKey::new(&asset.identity.key)?;
        DefinitionRevisionRef::new(&asset.identity.revision)?;
        Sha256HexDigest::new(&asset.sha256)?;
        if !assets.insert(&asset.identity) {
            return Err(ProjectError::InvalidProject("duplicate v2 asset"));
        }
    }
    if state
        .assets
        .windows(2)
        .any(|pair| pair[0].identity >= pair[1].identity)
    {
        return Err(ProjectError::InvalidProject(
            "v2 assets are not identity sorted",
        ));
    }
    limits.check(
        "v2 appearance bindings",
        state.appearance_bindings.len(),
        limits.max_reference_records,
    )?;
    let mut bound = BTreeSet::new();
    for binding in &state.appearance_bindings {
        if binding.presentation.family != ProjectV2Family::Presentation {
            return Err(ProjectError::InvalidProject(
                "appearance binding requires Presentation",
            ));
        }
        require_ref(&binding.presentation)?;
        if !assets.contains(&binding.asset) {
            return Err(ProjectError::InvalidProject("appearance asset is missing"));
        }
        if !bound.insert(&binding.presentation) {
            return Err(ProjectError::InvalidProject("duplicate appearance binding"));
        }
    }
    if state
        .appearance_bindings
        .windows(2)
        .any(|pair| pair[0].presentation >= pair[1].presentation)
    {
        return Err(ProjectError::InvalidProject(
            "appearance bindings are not identity sorted",
        ));
    }
    limits.check("v2 sources", state.sources.len(), limits.max_import_records)?;
    let mut source_keys = BTreeSet::new();
    for source in &state.sources {
        ProductionKey::new(&source.key)?;
        ProductionAtom::new("v2 source revision", &source.revision)?;
        Sha256HexDigest::new(&source.sha256)?;
        let batch = imports
            .iter()
            .find(|batch| batch.batch_id == source.import_batch_id)
            .ok_or(ProjectError::InvalidProject(
                "v2 source import batch is missing",
            ))?;
        if batch.source_revision != source.revision || batch.source_artifact_sha256 != source.sha256
        {
            return Err(ProjectError::InvalidProject(
                "v2 source disagrees with import batch",
            ));
        }
        if !source_keys.insert((&source.key, &source.revision)) {
            return Err(ProjectError::InvalidProject("duplicate v2 source"));
        }
    }
    if state
        .sources
        .windows(2)
        .any(|pair| (&pair[0].key, &pair[0].revision) >= (&pair[1].key, &pair[1].revision))
    {
        return Err(ProjectError::InvalidProject(
            "v2 sources are not identity sorted",
        ));
    }
    limits.check(
        "v2 editor entries",
        state.editor.len(),
        limits.max_reference_records,
    )?;
    let mut alias_keys = BTreeSet::new();
    let mut targets = BTreeSet::new();
    for entry in &state.editor {
        require_ref(&entry.target)?;
        if !targets.insert(&entry.target) {
            return Err(ProjectError::InvalidProject("duplicate v2 editor target"));
        }
        if entry.aliases.windows(2).any(|pair| pair[0] >= pair[1]) {
            return Err(ProjectError::InvalidProject(
                "v2 editor aliases are not sorted and unique",
            ));
        }
        for alias in &entry.aliases {
            if alias.trim() != alias
                || alias.is_empty()
                || alias.chars().any(char::is_control)
                || !alias_keys.insert((entry.target.family, alias))
            {
                return Err(ProjectError::InvalidProject(
                    "invalid or duplicate v2 alias",
                ));
            }
        }
        for tag in &entry.tags {
            ProductionKey::new(tag)?;
        }
        if entry.tags.windows(2).any(|pair| pair[0] >= pair[1]) {
            return Err(ProjectError::InvalidProject(
                "v2 editor tags are not sorted and unique",
            ));
        }
    }
    if state
        .editor
        .windows(2)
        .any(|pair| pair[0].target >= pair[1].target)
    {
        return Err(ProjectError::InvalidProject(
            "v2 editor entries are not identity sorted",
        ));
    }
    Ok(())
}

impl CanonicalProjectDocuments {
    pub fn from_v2_draft(
        mut draft: ProjectV2Draft,
        limits: ProjectEvidenceLimits,
    ) -> Result<Self, ProjectError> {
        let limits = limits.validate()?;
        draft.core.records = sorted_records(draft.core.records);
        draft.core.imports = sorted_imports(draft.core.imports);
        draft.core.metadata = sorted_metadata(draft.core.metadata);
        draft
            .state
            .declarations
            .sort_by(|a, b| (a.family(), &a.identity().key).cmp(&(b.family(), &b.identity().key)));
        for declaration in &mut draft.state.declarations {
            declaration.canonicalize();
        }
        draft
            .state
            .authoring_profiles
            .sort_by(|left, right| left.target.cmp(&right.target));
        for profile in &mut draft.state.authoring_profiles {
            profile.canonicalize();
        }
        draft
            .state
            .item_authoring
            .sort_by(|left, right| left.item.cmp(&right.item));
        for item in &mut draft.state.item_authoring {
            item.canonicalize();
        }
        draft.state.worlds.sort_by(|a, b| a.key.cmp(&b.key));
        draft.state.placements.sort_by(|a, b| a.key.cmp(&b.key));
        draft
            .state
            .assets
            .sort_by(|a, b| a.identity.cmp(&b.identity));
        draft
            .state
            .appearance_bindings
            .sort_by(|a, b| a.presentation.cmp(&b.presentation));
        draft
            .state
            .sources
            .sort_by(|a, b| (&a.key, &a.revision).cmp(&(&b.key, &b.revision)));
        draft.state.editor.sort_by(|a, b| a.target.cmp(&b.target));
        for entry in &mut draft.state.editor {
            entry.aliases.sort();
            entry.tags.sort();
        }
        draft.core.validate(limits)?;
        validate_v2_state(
            &draft.state,
            &draft.core.records,
            &draft.core.imports,
            limits,
        )?;
        limits.check(
            "project documents",
            3 + ROLE_SPECS.len(),
            limits.max_documents,
        )?;
        let mut budget = CanonicalWriteBudget::new(limits);
        let mut managed = BTreeMap::new();
        managed.insert(
            ROLE_SPECS[0].1.to_owned(),
            budget.encode(&ReferenceDocument {
                schema: WORLD_PROJECT_REFERENCE_SCHEMA.to_owned(),
                world_id: draft.core.world_id.clone(),
                coordinate_frame: draft.core.coordinate_frame.clone(),
                records: draft.core.records,
            })?,
        );
        managed.insert(
            ROLE_SPECS[1].1.to_owned(),
            budget.encode(&DeclarationsDocument {
                schema: DECLARATIONS_SCHEMA.to_owned(),
                records: draft.state.declarations,
                item_authoring: draft.state.item_authoring,
                authoring_profiles: draft.state.authoring_profiles,
            })?,
        );
        managed.insert(
            ROLE_SPECS[2].1.to_owned(),
            budget.encode(&WorldsDocument {
                schema: WORLDS_SCHEMA.to_owned(),
                worlds: draft.state.worlds,
                placements: draft.state.placements,
            })?,
        );
        managed.insert(
            ROLE_SPECS[3].1.to_owned(),
            budget.encode(&BindingsDocument {
                schema: PRESENTATIONS_SCHEMA.to_owned(),
                bindings: draft.state.appearance_bindings,
            })?,
        );
        managed.insert(
            ROLE_SPECS[4].1.to_owned(),
            budget.encode(&AssetsDocument {
                schema: ASSETS_SCHEMA.to_owned(),
                assets: draft.state.assets,
            })?,
        );
        managed.insert(
            ROLE_SPECS[5].1.to_owned(),
            budget.encode(&ImportDocument {
                schema: WORLD_PROJECT_IMPORT_SCHEMA.to_owned(),
                batches: draft.core.imports,
            })?,
        );
        managed.insert(
            ROLE_SPECS[6].1.to_owned(),
            budget.encode(&SourcesDocument {
                schema: PROVENANCE_SCHEMA.to_owned(),
                sources: draft.state.sources,
            })?,
        );
        managed.insert(
            ROLE_SPECS[7].1.to_owned(),
            budget.encode(&EditorDocument {
                schema: EDITOR_SCHEMA.to_owned(),
                legacy_entries: draft.core.metadata,
                entries: draft.state.editor,
            })?,
        );
        let mut inventory = Vec::new();
        for (role, locator, schema) in ROLE_SPECS {
            validate_locator(locator, limits)?;
            let bytes = managed
                .get(locator)
                .ok_or(ProjectError::InvalidProject("missing v2 canonical role"))?;
            inventory.push(ManifestDocument {
                role: role.to_owned(),
                schema: schema.to_owned(),
                locator: locator.to_owned(),
                byte_length: bytes.len(),
                sha256: digest_hex(bytes),
            });
        }
        inventory.sort_by(|a, b| a.locator.cmp(&b.locator));
        let manifest = ManifestDocumentRoot {
            schema: WORLD_PROJECT_V2_MANIFEST_SCHEMA.to_owned(),
            package_key: draft.core.package_key,
            package_revision: draft.core.project_revision.clone(),
            semantic_schema_version: draft.core.semantic_schema_version,
            licensing_metadata: draft.core.licensing_metadata,
            required_features: Vec::new(),
            optional_features: Vec::new(),
            documents: inventory,
        };
        let manifest_bytes = budget.encode(&manifest)?;
        let package = package_binding(&manifest, &manifest_bytes)?;
        let lock = LockDocument {
            schema: WORLD_PROJECT_V2_LOCK_SCHEMA.to_owned(),
            project_revision: draft.core.project_revision.clone(),
            revision_digest_token: format!("lock:{}", draft.core.project_revision),
            entries: vec![LockEntryDocument {
                package_key: manifest.package_key.clone(),
                package_revision: manifest.package_revision.clone(),
                package_provenance_digest: package.package_provenance_digest()?.as_str().to_owned(),
                floating: false,
                dependency: false,
            }],
        };
        let lock_bytes = budget.encode(&lock)?;
        let root = RootDocument {
            schema: WORLD_PROJECT_V2_ROOT_SCHEMA.to_owned(),
            source_profile: WORLD_PROJECT_V2_SOURCE_PROFILE.to_owned(),
            project_revision: draft.core.project_revision,
            manifest_locator: MANIFEST_LOCATOR.to_owned(),
            manifest_sha256: digest_hex(&manifest_bytes),
            content_lock_locator: LOCK_LOCATOR.to_owned(),
            content_lock_sha256: digest_hex(&lock_bytes),
        };
        managed.insert(MANIFEST_LOCATOR.to_owned(), manifest_bytes);
        managed.insert(LOCK_LOCATOR.to_owned(), lock_bytes);
        managed.insert(PROJECT_LOCATOR.to_owned(), budget.encode(&root)?);
        let snapshot = ProjectSnapshot::new(managed.into_iter(), limits)?;
        snapshot.parse(limits)?;
        Ok(Self {
            documents: snapshot.documents,
        })
    }
}
