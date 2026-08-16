use std::{collections::VecDeque, time::Instant};

use artifacts_api::{ActionType, CooldownSchema, ItemSlot, MapLayer, MapSchema};
use chrono::{DateTime, Utc};

#[derive(Debug)]
pub struct CooldownTimestamps {
    /// The start of the cooldown
    started_at: DateTime<Utc>,
    /// The expiration of the cooldown
    expiration: DateTime<Utc>,
}

impl CooldownTimestamps {
    pub fn new(
        started_at: Box<str>,
        expiration: Box<str>,
    ) -> chrono::ParseResult<CooldownTimestamps> {
        let started_at = DateTime::parse_from_rfc3339(&started_at)?;
        let expiration = DateTime::parse_from_rfc3339(&expiration)?;

        Ok(CooldownTimestamps {
            started_at: started_at.to_utc(),
            expiration: expiration.to_utc(),
        })
    }
    pub fn start(&self) -> &DateTime<Utc> {
        &self.started_at
    }
    pub fn expiry(&self) -> &DateTime<Utc> {
        &self.expiration
    }
}

#[derive(Debug)]
pub struct ActiveCooldown {
    pub command_name: Box<str>,
    /// The moment at which this cooldown-tracker last witnessed
    /// a cooldown start, for consistency
    pub client_side_instant: Instant,
    /// The timestamps recorded from the [schema](CooldownSchema)
    pub timestamps: CooldownTimestamps,
    /// The reason of the cooldown
    pub reason: ActionType,
}

impl ActiveCooldown {
    fn new(
        command_name: Box<str>,
        started_at: Box<str>,
        expiration: Box<str>,
        reason: ActionType,
    ) -> chrono::ParseResult<ActiveCooldown> {
        let timestamps = CooldownTimestamps::new(started_at, expiration)?;

        Ok(ActiveCooldown {
            command_name,
            client_side_instant: Instant::now(),
            timestamps,
            reason,
        })
    }

    pub fn now(&self) -> DateTime<Utc> {
        self.timestamps.started_at + self.client_side_instant.elapsed()
    }

    pub fn is_expired(&self) -> bool {
        self.now() >= self.timestamps.expiration
    }

    pub fn elapsed(&self) -> chrono::TimeDelta {
        self.now().signed_duration_since(self.timestamps.started_at)
    }
    pub fn remaining(&self) -> chrono::TimeDelta {
        self.timestamps.expiration.signed_duration_since(self.now())
    }
    pub fn total(&self) -> chrono::TimeDelta {
        self.timestamps
            .expiration
            .signed_duration_since(self.timestamps.started_at)
    }
}

#[derive(Debug, Default)]
pub struct ActiveHealth {
    pub max_value: u32,
    pub current_value: u32,
}

#[derive(Debug, Default)]
pub struct HealthTracker {
    /// The active health-values if any
    pub active_health: Option<ActiveHealth>,
}

impl HealthTracker {
    #[tracing::instrument(level = "info")]
    pub fn update_from_action_response_data_schema(
        &mut self,
        character_name: &str,
        response_schema: &network::ActionResponseDataSchema,
    ) {
        if let Some(character_schema) = response_schema.character_schema(character_name) {
            self.active_health = Some(ActiveHealth {
                max_value: character_schema.max_hp as u32,
                current_value: character_schema.hp as u32,
            });
        }
    }
}

#[derive(Debug, Default)]
pub struct CooldownTracker {
    /// The active cooldown if any
    pub active_cooldown: Option<ActiveCooldown>,
}

impl CooldownTracker {
    pub fn update_from_cooldown_schema(
        &mut self,
        command_name: Box<str>,
        CooldownSchema {
            started_at,
            expiration,
            reason,
            ..
        }: &CooldownSchema,
    ) -> chrono::ParseResult<()> {
        let active_cooldown = ActiveCooldown::new(
            command_name,
            started_at.clone(),
            expiration.clone(),
            reason.clone(),
        )?;

        self.active_cooldown = Some(active_cooldown);

        Ok(())
    }
    pub fn is_active(&self) -> bool {
        self.active_cooldown
            .as_ref()
            .is_some_and(|cd| !cd.is_expired())
    }
    pub fn elapsed(&self) -> Option<chrono::TimeDelta> {
        self.active_cooldown.as_ref().map(|cd| cd.elapsed())
    }
    pub fn remaining(&self) -> Option<chrono::TimeDelta> {
        self.active_cooldown.as_ref().map(|cd| cd.remaining())
    }
    pub fn total(&self) -> Option<chrono::TimeDelta> {
        self.active_cooldown.as_ref().map(|cd| cd.total())
    }
}

#[derive(Debug)]
pub struct PlayerPosition {
    /// Position X of the player on the map
    pub x: i64,
    /// Position Y of the player on the map
    pub y: i64,
}

#[derive(Debug)]
pub struct ActivePosition {
    pub position: PlayerPosition,
    pub layer: MapLayer,
    pub map_id: i64,
}

impl ActivePosition {
    pub fn new(
        (position_x, position_y): (i64, i64),
        layer: MapLayer,
        map_id: i64,
    ) -> ActivePosition {
        ActivePosition {
            position: PlayerPosition {
                x: position_x,
                y: position_y,
            },
            layer,
            map_id,
        }
    }
}

#[derive(Debug, Default)]
pub struct PositionTracker {
    pub active_position: Option<ActivePosition>,
}

impl PositionTracker {
    pub fn update_position(&mut self, position: (i64, i64), layer: MapLayer, map_id: i64) {
        self.active_position = Some(ActivePosition::new(position, layer, map_id));
    }
    pub fn update_position_from_map_schema(
        &mut self,
        MapSchema {
            map_id,
            x,
            y,
            layer,
            ..
        }: &MapSchema,
    ) {
        self.update_position((*x, *y), layer.clone(), *map_id);
    }
}

#[derive(Debug, Default)]
pub struct PlayerTracker {
    pub health: HealthTracker,
    pub position: PositionTracker,
    pub cooldown: CooldownTracker,
}

impl PlayerTracker {
    pub fn update_from_response_schema(
        &mut self,
        character_name: &str,
        response_schema: &network::ActionResponseDataSchema,
    ) -> anyhow::Result<()> {
        if let Some(cooldown_schema) = response_schema.cooldown_schema() {
            self.cooldown.update_from_cooldown_schema(
                response_schema.command_name().into(),
                cooldown_schema,
            )?;
        }

        if let Some(map_schema) = response_schema.map_schema() {
            self.position.update_position_from_map_schema(map_schema);
        }

        self.health
            .update_from_action_response_data_schema(character_name, response_schema);

        Ok(())
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CommandModifier {
    Repeat(u32),
}

#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Command {
    pub name: String,
    pub modifier: Option<CommandModifier>,
    pub arguments: Box<[(String, String)]>,
}

impl Command {
    pub fn into_unmodified(self) -> core::iter::RepeatN<UnmodifiedCommand> {
        let count = self.repeat_count() as usize;
        core::iter::repeat_n(
            UnmodifiedCommand {
                name: self.name,
                arguments: self.arguments,
            },
            count,
        )
    }
}

/// Indicates that the responsibility of
/// repeats etc. is already taken care of
#[derive(Clone, Debug, Default)]
pub struct UnmodifiedCommand {
    pub name: String,
    pub arguments: Box<[(String, String)]>,
}

impl Command {
    pub fn repeated(self, times: u32) -> Command {
        Command {
            modifier: Some(CommandModifier::Repeat(times)),
            ..self
        }
    }
    pub fn r#move(x: i64, y: i64) -> Command {
        Command {
            name: "move".into(),
            arguments: Box::from_iter([("x".into(), x.to_string()), ("y".into(), y.to_string())]),
            ..Default::default()
        }
    }

    pub fn repeat_count(&self) -> u32 {
        match self.modifier.as_ref() {
            Some(CommandModifier::Repeat(value)) => *value,
            _ => 1,
        }
    }
}

// below is more application networking logic i guess

#[expect(dead_code)]
fn apply_headers(
    req: ureq::RequestBuilder<ureq::typestate::WithBody>,
    api_key: &str,
) -> ureq::RequestBuilder<ureq::typestate::WithBody> {
    req.header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", api_key))
}

pub mod network;

pub struct ErrorData {
    pub message: Box<str>,
    pub data: Option<serde_json::Value>,
}

/// code-field is omitted from variants
pub enum ActionMoveError {
    /// Code 404
    MapNotFound(ErrorData),
}

pub mod error_codes {
    pub type ErrorCodePrimitive = u16;

    macro_rules! define_code {
        ($name:ident = $val:literal) => {
            pub const $name: ErrorCodePrimitive = $val;
        };
        ($hd:ident = $val:literal, $($succ:ident = $succ_val:literal),+) => {
            define_code!($hd = $val);
            $(define_code!($succ = $succ_val);)+
        }
    }

    define_code! {
        // relates more to an item being missing etc. as part of use-commands, or maps in terms
        // of move-commands
        TARGET_NOT_FOUND = 404,
        INVALID_PAYLOAD = 422,
        INSUFFICIENT_GOLD_IN_BANK = 460,
        ITEMS_OR_GOLD_ALREADY_IN_TRANSACTION = 461,
        MISSING_REQUIRED_ITEMS = 478,
        ACTION_ALREADY_IN_PROGRESS = 486,
        NOT_ENOUGH_GOLD = 492,
        CHARACTER_NOT_FOUND = 498,
        CHARACTER_IN_COOLDOWN = 499,
        // relates more to being on the wrong map-tile
        INTERACTION_TARGET_NOT_FOUND = 598
    }
}

/// Refers to any type that
/// without using any of it's own data (functions have no receiver)
///
/// Can be used to refer to some static list of field-names expected
/// in any body payload.
///
/// Can also be used to refer to some static list of required field-names
/// for the body payload.
pub trait RequestObjectFields {
    fn field_names() -> &'static [&'static str];
    fn required_field_names() -> Option<&'static [&'static str]>;
}

pub fn is_valid_input_arg_for_request_object<Object>(input: &str) -> bool
where
    Object: RequestObjectFields,
{
    Object::field_names()
        .iter()
        .any(|valid_arg_name| input.trim_start_matches('-').starts_with(valid_arg_name))
}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct MoveRequestCoordinates {
    pub x: i64,
    pub y: i64,
}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct MoveRequestBody {
    #[serde(flatten)]
    pub coordinates: Option<MoveRequestCoordinates>,
    pub map_id: Option<u64>,
}

impl RequestObjectFields for MoveRequestBody {
    fn field_names() -> &'static [&'static str] {
        &["x", "y", "map_id"]
    }
    fn required_field_names() -> Option<&'static [&'static str]> {
        None
    }
}

/// Contained in the outer [body](EquipRequestBody)
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct EquipRequestBodyEntry {
    pub code: Box<str>,
    pub slot: ItemSlot,
    pub quantity: Option<u64>,
}

impl RequestObjectFields for EquipRequestBodyEntry {
    fn field_names() -> &'static [&'static str] {
        &["code", "slot", "quantity"]
    }
    fn required_field_names() -> Option<&'static [&'static str]> {
        Some(&["code", "slot"])
    }
}

impl EquipRequestBodyEntry {}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct EquipRequestBody(pub Box<[EquipRequestBodyEntry]>);

/// assumes the input is a valid argument, blind to which commands or state in general
/// and uses the `take_next_value`-argument to ask for the next (assumes one fragment only in input)
pub fn parse_request_argument(
    input: &str,
    mut take_next_value: impl FnMut() -> Option<String>,
) -> Option<(String, String)> {
    // make sure it is a `-` or `+` prefixed input
    if !(input.starts_with('-') | input.starts_with('+')) {
        return None;
    }
    // trim only `-`, indicating that this is indeed the key of an argument
    let input = input.trim_start_matches('-');
    // the `var=key` case
    //
    // here we do not touch the next value
    if let Some((key, val)) = input.split_once('=') {
        Some((key.into(), val.into()))
    }
    // fallback, where we use the function provided
    else {
        take_next_value().map(|argument_value| (input.into(), argument_value))
    }
}

pub fn parse_request_modifier(input: &str) -> Option<String> {
    if !input.starts_with('+') {
        return None;
    }

    let input = input.trim_start_matches('+');

    if input.contains(' ') {
        let (val, _) = input.split_once(' ')?;
        Some(val.into())
    }
    // assume the whole thing is a modifier
    else {
        Some(input.into())
    }
}

/// Not yet sorted or contextualized, but
/// completely opaque
#[derive(Debug)]
pub struct OpaqueRequest {
    #[allow(dead_code)]
    command_name: String,
    #[allow(dead_code)]
    command_arguments: Option<Box<[(String, String)]>>,
    #[allow(dead_code)]
    command_modifier: Option<String>,
}

/// Returns the remainder (the point at which we could not parse more of
/// the input-line)
pub fn parse_next_opaque(line: &str) -> Result<(OpaqueRequest, Option<&str>), &str> {
    if !line.is_empty() && !line.contains(' ') {
        return Ok((
            OpaqueRequest {
                command_name: line.into(),
                command_arguments: None,
                command_modifier: None,
            },
            None,
        ));
    }
    let (command_name, mut tail) = line.split_once(' ').ok_or(line)?;

    let mut buf = vec![];
    let mut prev_tail = None;
    let mut command_modifier = None;

    while !tail.is_empty() {
        // first we try to match it as a modifier
        if let Some(modifier) = parse_request_modifier(tail) {
            // get the tail by trimming the parsed term off
            tail = tail.trim_start_matches('+').trim_start_matches(&modifier);
            command_modifier = Some(modifier);
            // skip argument parsing for this iteration
            continue;
        };

        // continue and try it as an argument

        match parse_request_argument(tail, || {
            let (arg, after_arg) = tail.split_once(' ')?;

            prev_tail = Some(tail);
            tail = after_arg;

            Some(arg.into())
        }) {
            Some(arguments) => buf.push(arguments),
            None => {
                break;
            }
        }
    }

    let command_arguments = if buf.is_empty() {
        None
    } else {
        Some(buf.into_boxed_slice())
    };

    Ok((
        OpaqueRequest {
            command_name: command_name.into(),
            command_arguments,
            command_modifier,
        },
        prev_tail,
    ))
}

#[derive(Debug)]
pub enum CommandArgs {
    List(&'static [&'static str]),
    Empty,
}

fn command_args_list(command_name: &str) -> Option<CommandArgs> {
    let list = match command_name {
        "move" => CommandArgs::List(&["x", "y", "world_id"]),
        "unequip" => CommandArgs::List(&["slot", "quantity"]),
        "craft" => CommandArgs::List(&["code", "quantity"]),
        "equip" => CommandArgs::List(&["code", "slot", "quantity"]),
        "recycle" => CommandArgs::List(&["code", "quantity", "enhanced"]),
        "use" => CommandArgs::List(&["code", "quantity"]),
        "fight" | "rest" | "gather" => CommandArgs::Empty,
        // BANK-commands
        "bank-deposit-gold" | "bank-withdraw-gold" => CommandArgs::List(&["quantity"]),
        "bank-deposit-item" | "bank-withdraw-item" => CommandArgs::List(&["code", "quantity"]),
        // NPC-commands
        "npc-buy" | "npc-sell" => CommandArgs::List(&["code", "quantity"]),

        _otherwise => {
            return None;
        }
    };

    Some(list)
}

/// Parses line-by-line
///
/// This is a suitable entry-point for the [Args](std::env::Args) struct for example for usage in CLI-contexts
#[derive(Debug)]
pub struct LineParser<'a, I>
where
    I: Iterator<Item = String>,
{
    it: &'a mut I,
    cached_command_name: Option<String>,
}

impl<'a, I> LineParser<'a, I>
where
    I: Iterator<Item = String>,
{
    pub fn new(it: &'a mut I) -> LineParser<'a, I> {
        LineParser {
            it,
            cached_command_name: None,
        }
    }
    #[tracing::instrument(level = "debug", skip(self, is_arg_valid), ret)]
    fn parse_next_arg(
        &mut self,
        is_arg_valid: impl FnOnce(&str) -> bool,
    ) -> Option<(String, String)> {
        // let first_val = self.fst.take().or_else(|| self.args.next())?;
        let first_val = self.it.next()?;
        tracing::info!("parsing next argument with first-value {first_val:?}");
        if !is_arg_valid(&first_val) {
            tracing::warn!(
                "{first_val} not a valid arg for current command! setting to remainder slot!"
            );
            self.cached_command_name = Some(first_val);
            None
        } else {
            parse_request_argument(&first_val, || self.it.next())
        }
    }
    fn args_from_valid_list(
        &mut self,
        valid_list: &'static [&'static str],
    ) -> impl FnMut() -> Option<(String, String)> {
        || {
            self.parse_next_arg(|input| {
                valid_list
                    .iter()
                    .any(|valid_arg_name| input.trim_start_matches('-').starts_with(valid_arg_name))
            })
        }
    }
    /// handles the translation of the cli-language into commands
    ///
    /// _thus_ for the process of creating a tui-application,
    /// this should be replicated in some way through a user-event
    /// system.
    ///
    /// hopefully simply of course.
    #[tracing::instrument(level = "trace", skip(self), ret)]
    pub fn parse_next_command(&mut self) -> Option<Command> {
        let cached_value = if let Some(cached_fst) = self.cached_command_name.take() {
            cached_fst
        } else {
            // tracing::info!("next branch");
            self.it.next()?
        };

        let (modifier, name): (Option<CommandModifier>, String) = if cached_value.starts_with('+')
            && cached_value.trim_start_matches('+').starts_with('x')
        {
            (
                cached_value
                    .trim_start_matches('+')
                    .trim_start_matches('x')
                    .parse::<u32>()
                    .ok()
                    .map(CommandModifier::Repeat),
                self.it.next()?,
            )
        } else {
            (None, cached_value)
        };

        tracing::debug!("parsing command {name} (modifier={modifier:?})");

        let Some(args_list) = command_args_list(&name) else {
            tracing::warn!("unimplemented command {name}");
            self.cached_command_name = Some(name);
            return None;
        };

        let arguments = match args_list {
            CommandArgs::List(list) => {
                Box::from_iter(core::iter::from_fn(self.args_from_valid_list(list)))
            }
            CommandArgs::Empty => Box::default(),
        };

        Some(Command {
            name,
            modifier,
            arguments,
        })
    }
}

fn parse_task_label_line(task_name: impl Into<Box<str>>, task_label_line: String) -> Option<Task> {
    let (count, remainder) = task_label_line
        .split_once(':')
        .expect("malformed task label");

    let count: u32 = if count.is_empty() {
        1
    } else {
        count.parse().expect("malformed count")
    };

    let step = TaskStep::Invoke {
        name: remainder.into(),
        repeat: count,
    };

    Some(Task {
        name: task_name.into(),
        steps: Box::from([step]),
    })
}

/// Parses line-by-line, essentially functioning as a struct similar to [Args](std::env::Args)
///
/// but splits each input-line on whitespaces prior to parsing (from there, it uses the [LineParser])
pub struct CommandParser {
    pub commands: VecDeque<String>,
}

impl CommandParser {
    pub fn new(commands: impl IntoIterator<Item = String>) -> CommandParser {
        CommandParser {
            commands: VecDeque::from_iter(commands),
        }
    }
    pub fn parse_next(&mut self) -> Option<Task> {
        let line = self.commands.pop_front()?;

        // seems to be a task-label!
        if line.contains(':') {
            // here we can alter the syntax a bit by changing how the function works
            parse_task_label_line("test-task", line)
        } else {
            // here we treat it as a regular command line
            let mut line_iter = line.split_whitespace().map(<str>::to_string);

            let mut parser = LineParser::new(&mut line_iter);

            let mut step_buf = vec![];

            while let Some(cmd) = parser.parse_next_command() {
                step_buf.push(TaskStep::Cmd(cmd));
            }

            Some(Task {
                name: "test-task-1".into(),
                steps: step_buf.into_boxed_slice(),
            })
        }
    }
}

pub mod task;

pub use task::{Task, TaskStep};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_empty_request_failure() {
        assert!(parse_next_opaque("").is_err_and(|e| e.is_empty()))
    }

    #[test]
    fn parse_rest_request_success() {
        let (elt, tail) = parse_next_opaque("rest").expect("failed to parse rest");
        assert_eq!(elt.command_name, "rest");
        std::assert_matches!(elt.command_arguments, None);
        std::assert_matches!(tail, None)
    }

    #[test]
    fn parse_equip_request_success() {
        parse_next_opaque("equip -code copper_pickaxe -slot weapon")
            .expect("failed to parse equip");
    }
    // #[test]
    // fn serialize_equip_request_body_entry() {
    //     let input = "equip -code copper_pickaxe -slot weapon";
    //     let entry = EquipRequestBodyEntry {
    //         code: "copper_pickaxe".into(),
    //         slot: ItemSlot::Weapon,
    //         quantity: None,
    //     };
    //     // is_valid_input_arg_for_request_object(input);
    // }

    #[test]
    fn command_parser_action_sequence() {
        let line = "move -x 0 -y 1 +x60 gather";
        let mut p = CommandParser::new(vec![line.into()]);

        let mut out_buf = vec![];

        while let Some(t) = p.parse_next() {
            out_buf.push(t);
        }

        assert_eq!(out_buf.len(), 1);
        assert_eq!(out_buf[0].steps.len(), 2);
        assert_eq!(
            out_buf[0].steps[0],
            TaskStep::Cmd(Command {
                name: "move".into(),
                modifier: None,
                arguments: Box::from_iter([("x".into(), "0".into()), ("y".into(), "1".into())])
            })
        );
        assert_eq!(
            out_buf[0].steps[1],
            TaskStep::Cmd(Command {
                name: "gather".into(),
                modifier: Some(CommandModifier::Repeat(60)),
                arguments: Box::default()
            })
        );
    }

    #[test]
    fn command_parser_task_label() {
        let line = "4:craft_copper_helmet";
        let mut p = CommandParser::new(vec![line.to_owned()]);

        let mut out_buf = vec![];

        while let Some(t) = p.parse_next() {
            out_buf.push(t);
        }

        assert_eq!(out_buf.len(), 1);
        // assert_eq!(out_buf[0].steps.len(), 4);
        assert_eq!(
            out_buf[0].steps[0],
            TaskStep::Invoke {
                name: "craft_copper_helmet".into(),
                repeat: 4
            },
        );
    }
}
