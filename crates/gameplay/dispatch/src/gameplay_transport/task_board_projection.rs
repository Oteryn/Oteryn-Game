//! Bounded projections of the immutable durable command result.
use crate::durability::task_board::{TaskBoardCommit, TaskBoardRefusal};
use oteryn_gameplay_quests::quest::task_board::{self, full as rules};
use oteryn_protocol_oteryn::task_board::{self as wire, full as full_wire};

pub(super) fn legacy(commit: &TaskBoardCommit) -> Option<wire::TaskBoardReply> {
    let balances = &commit.view.balances;
    // The old fixed five-task selection/claim surface is retired. Its query keeps
    // balances and upgrades readable without inventing a second weekly board.
    Some(wire::TaskBoardReply {
        refused: commit.refusal.is_some(),
        selected: 0,
        kills: 0,
        points: balances.points,
        bounty_points: balances.bounty_points,
        talisman_levels: balances.talisman_levels,
        talisman_bonuses: std::array::from_fn(|path| {
            task_board::talisman_bonus(path, balances.talisman_levels[path])
        }),
        talisman_next_costs: std::array::from_fn(|path| {
            task_board::talisman_upgrade_cost_for_path(path, balances.talisman_levels[path])
                .unwrap_or(0)
        }),
        tasks: Vec::new(),
    })
}

fn refusal(value: Option<&TaskBoardRefusal>) -> u8 {
    use rules::TaskBoardRefusal as Rule;
    match value {
        None => 0,
        Some(TaskBoardRefusal::Rule(rule)) => match rule {
            Rule::InvalidInput => 1,
            Rule::WrongState => 2,
            Rule::LevelTooLow => 3,
            Rule::InsufficientResources => 4,
            Rule::Cooldown => 5,
            Rule::LimitReached => 6,
            Rule::AlreadyPurchased => 7,
            Rule::RequiresOutfit => 8,
            Rule::InvalidRandomDraw => 9,
            Rule::MissingCatalogue => 10,
            Rule::ArithmeticOverflow => 11,
            Rule::RetiredAction => 12,
        },
        Some(TaskBoardRefusal::InsufficientItems) => 13,
        Some(TaskBoardRefusal::InsufficientGold) => 14,
        Some(TaskBoardRefusal::UnavailableItem) => 15,
        Some(TaskBoardRefusal::CapacityExceeded) => 16,
        Some(TaskBoardRefusal::UnsupportedEntitlement) => 17,
    }
}

fn offer(task: &rules::BountyTask) -> full_wire::BountyOffer {
    full_wire::BountyOffer {
        race: task.race.clone(),
        required: task.required,
        experience: task.experience,
        points: task.points,
        grade: task.grade,
    }
}

pub(super) fn full(commit: &TaskBoardCommit) -> Option<full_wire::Reply> {
    let view = &commit.view;
    let data = &view.data;
    let balances = &view.balances;
    let board = data.weekly.as_ref().filter(|board| !board.finished);
    let weekly = if let Some(board) = board {
        let (points, seals) = board.rewards().ok()?;
        let experience = rules::weekly_experience(view.level, board.difficulty).ok()?;
        let mut kills = board
            .kills
            .iter()
            .map(|task| full_wire::KillTask {
                race: task.race.clone(),
                required: task.required,
                current: task.current,
            })
            .collect::<Vec<_>>();
        kills.push(full_wire::KillTask {
            race: String::new(),
            required: board.any_required,
            current: board.any_kills,
        });
        full_wire::Weekly {
            started: true,
            difficulty: board.difficulty,
            expanded: data.expansion,
            multiplier: u8::try_from(rules::completion_multiplier(
                board.completed_kills() + board.completed_deliveries(),
            ))
            .ok()?,
            kill_experience: experience,
            delivery_experience: experience,
            pending_points: u64::from(points),
            pending_soulseals: u64::from(seals),
            kills,
            deliveries: board
                .deliveries
                .iter()
                .map(|task| full_wire::DeliveryTask {
                    item: task.item_id,
                    required: task.required,
                    current: if task.delivered {
                        task.required
                    } else {
                        view.delivery_available
                            .get(&task.item_id)
                            .copied()
                            .unwrap_or(0)
                    },
                    delivered: task.delivered,
                })
                .collect(),
        }
    } else {
        full_wire::Weekly {
            started: false,
            difficulty: 0,
            expanded: data.expansion,
            multiplier: 1,
            kill_experience: 0,
            delivery_experience: 0,
            pending_points: 0,
            pending_soulseals: 0,
            kills: Vec::new(),
            deliveries: Vec::new(),
        }
    };
    let bounty = &data.bounty;
    let active = bounty.active.as_ref();
    let mut talisman = [full_wire::TalismanPath::default(); 4];
    for (path, target) in talisman.iter_mut().enumerate() {
        let level = balances.talisman_levels[path];
        *target = full_wire::TalismanPath {
            level: u16::try_from(level).ok()?,
            bonus: u16::try_from(task_board::talisman_bonus(path, level)).ok()?,
            next_cost: u16::try_from(
                task_board::talisman_upgrade_cost_for_path(path, level).unwrap_or(0),
            )
            .ok()?,
        };
    }
    let shop = rules::SHOP_OFFERS
        .iter()
        .map(|item| {
            let at_limit =
                item.kind == rules::TaskBoardShopKind::Promotion && data.promotion_points >= 50;
            Some(full_wire::ShopOffer {
                index: item.id,
                kind: match item.kind {
                    rules::TaskBoardShopKind::Item => 0,
                    rules::TaskBoardShopKind::Mount => 1,
                    rules::TaskBoardShopKind::Outfit => 2,
                    rules::TaskBoardShopKind::ItemDouble => 3,
                    rules::TaskBoardShopKind::Promotion => 4,
                },
                price: if at_limit {
                    0
                } else {
                    rules::shop_price(item, data.promotion_points).ok()?
                },
                primary: u32::from(item.appearance_or_item),
                secondary: u32::from(item.second_item.unwrap_or(0)),
                addon: item.addon,
                bought: at_limit || data.shop_purchases.contains(&item.id),
            })
        })
        .collect::<Option<Vec<_>>>()?;
    Some(full_wire::Reply {
        refusal: refusal(commit.refusal.as_ref()),
        period: u64::try_from(view.period).ok()?,
        points: u64::try_from(balances.points).ok()?,
        bounty_points: u64::try_from(balances.bounty_points).ok()?,
        soulseals: u64::try_from(balances.soulseals).ok()?,
        weekly,
        bounty: full_wire::Bounty {
            state: match bounty.state {
                rules::BountyStatus::None => 0,
                rules::BountyStatus::Selection => 1,
                rules::BountyStatus::Active => 2,
                rules::BountyStatus::Completed => 3,
            },
            difficulty: bounty.difficulty,
            selected: active.map_or(Some(0), |task| task.slot.checked_add(1))?,
            current: active.map_or(0, |task| task.current),
            rerolls: view.reroll_tokens,
            daily_reroll_at: u64::try_from(bounty.next_free_reroll_unix).ok()?,
            offers: bounty.offers.iter().map(offer).collect(),
            active: active.map(offer),
            lists: bounty
                .lists
                .iter()
                .map(|slot| full_wire::ListSlot {
                    unlocked: slot.unlocked,
                    preferred: slot.preferred.clone().unwrap_or_default(),
                    unwanted: slot.unwanted.clone().unwrap_or_default(),
                })
                .collect(),
        },
        talisman,
        shop,
    })
}
