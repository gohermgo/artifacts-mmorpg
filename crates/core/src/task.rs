use std::collections::{HashMap, HashSet};

use super::{Command, LineParser};

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

    let out = builtin::resolve_invokeable_builtin(invokeable_name, repeat_count).or_else(|| {
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

mod builtin;

pub fn expand_set(
    task_set: impl IntoIterator<Item = Task>,
) -> impl Iterator<Item = super::UnmodifiedCommand> {
    task_set
        .into_iter()
        .flat_map(Task::into_commands)
        .flat_map(Command::into_unmodified)
}
