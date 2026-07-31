use std::collections::{HashMap, HashSet};

use super::{Command, LineParser};

const MOVE_TO_COPPER_ORE: &str = "move -x 2 -y 0";
// fn gather_copper_ore(n: impl core::fmt::Display) -> String {
//     format!("+x{n} gather")
// }

const MOVE_TO_FORGE: &str = "move -x 1 -y 5";
fn craft_copper_bars(n: impl core::fmt::Display) -> String {
    format!("craft -code copper_bar -quantity {n}")
}

const MOVE_TO_BANK: &str = "move -x 4 -y 1";
fn farm_copper(n: u32, withdraw_ore_from_bank: bool, deposit_bars_into_bank: bool) -> Vec<String> {
    let mut base = vec![];

    if withdraw_ore_from_bank {
        base.extend_from_slice(&[
            MOVE_TO_BANK.into(),
            format!(
                "bank-withdraw-item -code copper_bar -quantity {n}",
                n = 10 * n
            ),
        ]);
    } else {
        base.extend_from_slice(&[
            MOVE_TO_COPPER_ORE.into(),
            format!("+x{n} gather", n = 10 * n),
        ])
    }

    base.extend_from_slice(&[MOVE_TO_FORGE.into(), craft_copper_bars(n)]);

    if deposit_bars_into_bank {
        base.extend_from_slice(&[
            MOVE_TO_BANK.into(),
            format!("bank-deposit-item -code copper_bar -quantity {n}"),
        ]);
    }

    base
}

const MOVE_TO_WEAPON_WORKSHOP: &str = "move -x 2 -y 1";
const MOVE_TO_GEAR_WORKSHOP: &str = "move -x 3 -y 1";

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

fn move_craft_gear(
    code: impl core::fmt::Display,
    quantity: impl core::fmt::Display,
) -> Vec<String> {
    [
        MOVE_TO_GEAR_WORKSHOP.into(),
        format!("craft -code {code} -quantity {quantity}"),
    ]
    .into()
}

fn move_craft_weapon(
    code: impl core::fmt::Display,
    quantity: impl core::fmt::Display,
) -> Vec<String> {
    [
        MOVE_TO_WEAPON_WORKSHOP.into(),
        format!("craft -code {code} -quantity {quantity}"),
    ]
    .into()
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskStep {
    Cmd(Command),
    Invoke { name: String, repeat: u32 },
}

pub struct Task {
    pub name: Box<str>,
    pub steps: Box<[TaskStep]>,
}

impl Task {
    pub fn commands(name: impl AsRef<str>, cit: impl Iterator<Item = Command>) -> Task {
        Task {
            name: name.as_ref().into(),
            steps: cit.map(TaskStep::Cmd).collect(),
        }
    }
    /// Flattens out `self`, loosing the task name in the process however
    pub fn into_commands(self) -> Vec<Command> {
        let Task { steps, name } = self;
        tracing::info!("flattening task {name}");
        let mut visited = HashSet::default();
        steps.into_iter().fold(Vec::new(), |mut buf, step| {
            match step {
                TaskStep::Cmd(command) => buf.push(command),
                TaskStep::Invoke { name, repeat } => {
                    if let Some(commands_in_task_def) =
                        resolve_invokeable(&name, repeat, &HashMap::default(), &mut visited)
                    {
                        buf.extend(commands_in_task_def);
                    }
                }
            }
            buf
        })
    }
}

pub fn resolve_invokeable(
    invokeable_name: &str,
    repeat_count: u32,
    user_defined_invokeables: &HashMap<String, Vec<TaskStep>>,
    visited: &mut HashSet<String>,
) -> Option<Vec<Command>> {
    if !visited.insert(invokeable_name.to_string()) {
        tracing::error!("Cycle hit on {invokeable_name}");
        return None;
    }

    let out = resolve_invokeable_builtin(invokeable_name, repeat_count).or_else(|| {
        user_defined_invokeables
            .get(invokeable_name)
            .and_then(|invokable_steps| {
                let mut out = vec![];
                for step in invokable_steps {
                    match step {
                        TaskStep::Cmd(command) => out.push(command.clone()),
                        TaskStep::Invoke { name, repeat } => {
                            // WARN: recursion here!
                            //       if it turns out the label doesn't exist, we will
                            //       get stack overflow at some point i think...
                            //
                            //       this essentially means some user invokeable
                            //       references some other task!
                            let resolved_in_user_task = resolve_invokeable(
                                name,
                                *repeat,
                                user_defined_invokeables,
                                visited,
                            )?;
                            out.extend(resolved_in_user_task);
                        }
                    }
                }
                Some(out)
            })
    });

    visited.remove(invokeable_name);

    out
}

fn resolve_gather(label: &str, line_buf: &mut Vec<String>) {
    // copper gathering
    //
    // gather implies non-crafting, so raw resources
    tracing::info!("RESOLVING GATHER from {label}");
    match label.rsplit_once('_') {
        Some(("g_copper_ore", digit)) if let Ok(gather_n) = digit.parse::<u32>() => line_buf
            .extend([
                MOVE_TO_COPPER_ORE.into(),
                format!("+x{gather_n} gather"),
                MOVE_TO_BANK.into(),
                format!("bank-deposit-item -code copper_ore -quantity {gather_n}"),
            ]),
        Some(("g_copper", digit)) if let Ok(farm_n) = digit.parse::<u32>() => {
            line_buf.extend(farm_copper(farm_n, false, true))
        }
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

            line_buf.extend(
                farm_copper(6 * craft_quantity, true, false)
                    .into_iter()
                    .chain(move_craft_step),
            )
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

            line_buf.extend(
                farm_copper(6 * craft_quantity, false, false)
                    .into_iter()
                    .chain(move_craft_step),
            );
        }
        _ => {}
    }
}

fn resolve_invokeable_builtin(
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
        "g_copper_6" => cmd_line_buf.extend(farm_copper(6, false, true)),
        // gather + craft
        "gc_copper_pickaxe" => {
            cmd_line_buf.extend(
                farm_copper(6, false, false)
                    .into_iter()
                    .chain(move_craft_weapon("copper_pickaxe", 1)),
            );
        }
        "gc_copper_helmet" => {
            cmd_line_buf.extend(
                farm_copper(6, false, false)
                    .into_iter()
                    .chain(move_craft_gear("copper_helmet", 1)),
            );
        }
        // craft
        "c_copper_pickaxe" => {
            cmd_line_buf.extend(
                farm_copper(6, true, false)
                    .into_iter()
                    .chain(move_craft_weapon("copper_pickaxe", 1)),
            );
        }
        "c_copper_helmet" => {
            cmd_line_buf.extend(
                farm_copper(6, true, false)
                    .into_iter()
                    .chain(move_craft_gear("copper_helmet", 1)),
            );
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
    tracing::info!("LINE PARSER CONSTRUCTED");
    let mut line_parser = LineParser::new(&mut lines);
    // while let Some(step_cmd) = line_parser.parse_next_command() {
    //     step_buf.push(TaskStep::Cmd(step_cmd));
    // }

    // Some(Task {
    //     name: invokeable_name.as_ref().into(),
    //     steps: core::iter::repeat_n(step_buf, repeat_count as usize)
    //         .flatten()
    //         .collect(),
    // })
    let command_vec: Vec<Command> =
        core::iter::from_fn(|| line_parser.parse_next_command()).collect();
    Some(
        core::iter::repeat_n(command_vec, repeat_count as usize)
            .flatten()
            .collect(),
    )
}

pub fn expand_set(
    task_set: impl IntoIterator<Item = Task>,
) -> impl Iterator<Item = super::UnmodifiedCommand> {
    task_set
        .into_iter()
        .flat_map(Task::into_commands)
        .flat_map(Command::into_unmodified)
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
