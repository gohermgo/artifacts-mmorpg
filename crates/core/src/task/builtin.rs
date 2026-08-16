use super::{Command, LineParser};

const MOVE_TO_COPPER_ORE: &str = "move -x 2 -y 0";

const MOVE_TO_FORGE: &str = "move -x 1 -y 5";
const MOVE_TO_BANK: &str = "move -x 4 -y 1";

const MOVE_TO_GEAR_WORKSHOP: &str = "move -x 3 -y 1";

struct Move {
    x: i32,
    y: i32,
}

impl Move {
    const fn to_copper_ore() -> Move {
        Move { x: 2, y: 0 }
    }
    const fn to_ash_tree() -> Move {
        // just one of the options
        Move { x: -1, y: 0 }
    }
    const fn to_forge() -> Move {
        Move { x: 1, y: 5 }
    }
    const fn to_bank() -> Move {
        Move { x: 4, y: 1 }
    }
    const fn to_weapon_workshop() -> Move {
        Move { x: 2, y: 1 }
    }
    const fn to_gear_workshop() -> Move {
        Move { x: 3, y: 1 }
    }
    const fn to_gudgeon_spot() -> Move {
        Move { x: 4, y: 2 }
    }
}

impl core::fmt::Display for Move {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "move -x {x} -y {y}", x = self.x, y = self.y)
    }
}

struct BankDepositItem {
    code: &'static str,
    quantity: u32,
}

impl core::fmt::Display for BankDepositItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "bank-deposit-item -code {code} -quantity {quantity}",
            code = self.code,
            quantity = self.quantity
        )
    }
}

struct BankWithdrawItem {
    code: &'static str,
    quantity: u32,
}

impl core::fmt::Display for BankWithdrawItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "bank-withdraw-item -code {code} -quantity {quantity}",
            code = self.code,
            quantity = self.quantity
        )
    }
}

struct Craft<C> {
    code: C,
    quantity: u32,
}

impl<C> core::fmt::Display for Craft<C>
where
    C: core::fmt::Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "craft -code {code} -quantity {quantity}",
            code = self.code,
            quantity = self.quantity
        )
    }
}

#[expect(dead_code)]
fn gather_and_craft_copper_helmet(n: u32, bank: bool) -> Vec<String> {
    let mut base = vec![
        MOVE_TO_COPPER_ORE.into(),
        format!("+x{n} gather", n = 10 * (6 * n)),
        MOVE_TO_FORGE.into(),
        format!("craft -code copper_bar -quantity {n}", n = 6 * n),
        MOVE_TO_GEAR_WORKSHOP.into(),
        format!("craft -code copper_helmet -quantity {n}"),
    ];

    if bank {
        base.extend_from_slice(&[
            MOVE_TO_BANK.into(),
            format!("bank-deposit-item -code copper_helmet -quantity {n}"),
        ]);
    }

    base
}

fn move_craft_gear(code: impl core::fmt::Display, quantity: u32) -> Vec<String> {
    [
        Move::to_gear_workshop().to_string(),
        Craft { code, quantity }.to_string(),
    ]
    .into()
}

fn move_craft_weapon(code: impl core::fmt::Display, quantity: u32) -> Vec<String> {
    [
        Move::to_weapon_workshop().to_string(),
        Craft { code, quantity }.to_string(),
    ]
    .into()
}

fn farm_inner(
    // whether to take from bank or not and how to do that (or gather it from resources)
    from_bank: impl IntoIterator<Item = String>,
    from_resource: impl IntoIterator<Item = String>,
    withdraw_from_bank: bool,
    // the steps in the middle, placed appropriately into the buffer
    gather_steps: impl IntoIterator<Item = String>,
    // whether to put into bank or not, and how to do that
    into_bank: impl IntoIterator<Item = String>,
    deposit_into_bank: bool,
    buf: &mut Vec<String>,
) {
    if withdraw_from_bank {
        buf.extend(from_bank);
    } else {
        buf.extend(from_resource);
    }

    buf.extend(gather_steps);

    if deposit_into_bank {
        buf.extend(into_bank);
    }
}

fn farm_ash(n: u32, withdraw_wood_from_bank: bool, deposit_planks_into_bank: bool) -> Vec<String> {
    let mut base = vec![];
    farm_inner(
        [
            Move::to_bank().to_string(),
            BankWithdrawItem {
                code: "ash_wood",
                quantity: 10 * n,
            }
            .to_string(),
        ],
        [
            Move::to_ash_tree().to_string(),
            format!("+x{n} gather", n = 10 * n),
        ],
        withdraw_wood_from_bank,
        [
            Move::to_bank().to_string(),
            BankDepositItem {
                code: "ash_plank",
                quantity: n,
            }
            .to_string(),
        ],
        [
            Move::to_forge().to_string(),
            Craft {
                code: "copper_bar",
                quantity: n,
            }
            .to_string(),
        ],
        deposit_planks_into_bank,
        &mut base,
    );

    base
}

fn farm_copper(
    n: u32,
    withdraw_ore_from_bank: bool,
    deposit_bars_into_bank: bool,
    buf: &mut Vec<String>,
) {
    // let mut base = vec![];

    farm_inner(
        [
            Move::to_bank().to_string(),
            BankWithdrawItem {
                code: "copper_ore",
                quantity: 10 * n,
            }
            .to_string(),
        ],
        [
            Move::to_copper_ore().to_string(),
            format!("+x{n} gather", n = 10 * n),
        ],
        withdraw_ore_from_bank,
        [
            Move::to_forge().to_string(),
            Craft {
                code: "copper_bar",
                quantity: n,
            }
            .to_string(),
        ],
        [
            Move::to_bank().to_string(),
            BankDepositItem {
                code: "copper_bar",
                quantity: n,
            }
            .to_string(),
        ],
        deposit_bars_into_bank,
        buf,
    );

    // if withdraw_ore_from_bank {
    //     base.extend_from_slice(&[
    //         Move::to_bank().to_string(),
    //         BankWithdrawItem {
    //             code: "copper_ore",
    //             quantity: 10 * n,
    //         }
    //         .to_string(),
    //     ]);
    // } else {
    //     base.extend_from_slice(&[
    //         Move::to_copper_ore().to_string(),
    //         format!("+x{n} gather", n = 10 * n),
    //     ])
    // }

    // base.extend_from_slice(&[
    //     Move::to_forge().to_string(),
    //     Craft {
    //         code: "copper_bar",
    //         quantity: n,
    //     }
    //     .to_string(),
    // ]);

    // if deposit_bars_into_bank {
    //     base.extend_from_slice(&[
    //         Move::to_bank().to_string(),
    //         BankDepositItem {
    //             code: "copper_bar",
    //             quantity: n,
    //         }
    //         .to_string(),
    //     ]);
    // }

    // base
}

fn resolve_gather(label: &str, line_buf: &mut Vec<String>) {
    // copper gathering
    //
    // gather implies non-crafting, so raw resources
    tracing::info!("RESOLVING GATHER from {label}");
    match label.rsplit_once('_') {
        Some(("g_copper_ore", digit)) if let Ok(gather_n) = digit.parse::<u32>() => line_buf
            .extend([
                Move::to_copper_ore().to_string(),
                format!("+x{gather_n} gather"),
                Move::to_bank().to_string(),
                BankDepositItem {
                    code: "copper_ore",
                    quantity: gather_n,
                }
                .to_string(),
            ]),
        Some(("g_copper", digit)) if let Ok(farm_n) = digit.parse::<u32>() => {
            farm_copper(farm_n, false, true, line_buf);
        }
        Some(("g_gudgeon", digit)) if let Ok(farm_n) = digit.parse::<u32>() => line_buf.extend([
            Move::to_gudgeon_spot().to_string(),
            format!("+x{farm_n} gather"),
            Move::to_bank().to_string(),
            BankDepositItem {
                code: "gudgeon",
                quantity: farm_n,
            }
            .to_string(),
        ]),
        _ => {}
    }
}

fn is_weapon_craftable(item_code: &str) -> bool {
    matches!(item_code, "pickaxe" | "axe" | "dagger")
}

fn split_invokeable_label_with_item(label: &str) -> Option<(&str, &str, &str)> {
    label.rsplit_once('_').and_then(|(name_and_item, count)| {
        name_and_item
            .rsplit_once('_')
            .map(|(name, item)| (name, item, count))
    })
}

/// Basically the same as `gc` commands, but when `c` prefixed
/// we do not perform the gather-step and instead withdraw ore from the bank
fn resolve_craft(label: &str, line_buf: &mut Vec<String>) {
    tracing::info!("RESOLVING CRAFT from {label}");
    match split_invokeable_label_with_item(label) {
        Some(("c_copper", item_code, count)) if let Ok(craft_quantity) = count.parse::<u32>() => {
            let move_craft_step = if is_weapon_craftable(item_code) {
                move_craft_weapon(item_code, craft_quantity)
            } else {
                move_craft_gear(item_code, craft_quantity)
            };

            farm_copper(6 * craft_quantity, true, false, line_buf);
            line_buf.extend(move_craft_step);
        }
        Some(("c_ash", "plank", count)) if let Ok(craft_quantity) = count.parse::<u32>() => {
            line_buf.extend(farm_ash(craft_quantity, true, true))
        }
        _ => {}
    }
}
fn resolve_gather_craft(label: &str, line_buf: &mut Vec<String>) {
    // split from front, to get craft-name and item-with-count
    // separately
    match split_invokeable_label_with_item(label) {
        Some(("gc_copper", item_code, count)) if let Ok(craft_quantity) = count.parse::<u32>() => {
            let move_craft_step = if is_weapon_craftable(item_code) {
                move_craft_weapon(item_code, craft_quantity)
            } else {
                move_craft_gear(item_code, craft_quantity)
            };

            farm_copper(6 * craft_quantity, false, false, line_buf);
            line_buf.extend(move_craft_step);
        }

        Some(("c_ash", "plank", count)) if let Ok(craft_quantity) = count.parse::<u32>() => {
            line_buf.extend(farm_ash(craft_quantity, false, true))
        }
        _ => {}
    }
}

fn resolve_fight_rest(label: &str, line_buf: &mut Vec<String>) {
    match label.rsplit_once('_') {
        Some(("fr", count_str)) if let Ok(count) = count_str.parse::<u32>() => {
            // just push a single loop of N fights followed by rest for now
            line_buf.push(format!("+x{count} fight rest"))
        }
        _ => {}
    }
}

pub fn resolve_invokeable_builtin(
    invokeable_name: impl AsRef<str>,
    repeat_count: u32,
) -> Option<Vec<Command>> {
    let mut cmd_line_buf = vec![];
    match invokeable_name.as_ref() {
        invokeable_label if invokeable_label.starts_with("gc_") => {
            resolve_gather_craft(invokeable_label, &mut cmd_line_buf)
        }
        invokeable_label if invokeable_label.starts_with("g_") => {
            resolve_gather(invokeable_label, &mut cmd_line_buf);
        }
        invokeable_label if invokeable_label.starts_with("c_") => {
            resolve_craft(invokeable_label, &mut cmd_line_buf);
        }
        invokeable_label if invokeable_label.starts_with("fr_") => {
            resolve_fight_rest(invokeable_label, &mut cmd_line_buf)
        }
        _ => return None,
    };

    // let mut step_buf = vec![];
    let mut lines = cmd_line_buf
        .into_iter()
        .fold(Vec::new(), |mut acc, s| {
            acc.extend(s.split_whitespace().map(<str>::to_string));
            acc
        })
        .into_iter();
    let mut line_parser = LineParser::new(&mut lines);

    let command_vec: Vec<Command> =
        core::iter::from_fn(|| line_parser.parse_next_command()).collect();
    Some(
        core::iter::repeat_n(command_vec, repeat_count as usize)
            .flatten()
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_different_labels() {
        assert_eq!(
            split_invokeable_label_with_item("gc_copper_pickaxe_10"),
            Some(("gc_copper", "pickaxe", "10"))
        )
    }
}
