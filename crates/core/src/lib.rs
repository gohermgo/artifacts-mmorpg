use std::time::Instant;

use artifacts_api::{
    ActionType, BankGoldTransactionResponseSchema, BankGoldTransactionSchema,
    BankItemTransactionResponseSchema, BankItemTransactionSchema, CharacterFightDataSchema,
    CharacterFightResponseSchema, CharacterMovementDataSchema, CharacterMovementResponseSchema,
    CharacterRestDataSchema, CharacterRestResponseSchema, CharacterSchema, CooldownSchema,
    EquipmentResponseSchema, EquipmentTransactionSchema, ItemSlot, MapLayer, MapSchema,
    NpcMerchantTransactionSchema, RecyclingDataSchema, RecyclingResponseSchema, SkillDataSchema,
    SkillResponseSchema, UseItemResponseSchema, UseItemSchema,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

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
    pub fn update_from_action_response_data_schema(
        &mut self,
        character_name: &str,
        response_schema: &ActionResponseDataSchema,
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
        response_schema: &ActionResponseDataSchema,
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

#[derive(Copy, Clone, Debug)]
pub enum CommandModifier {
    Repeat(u32),
}

#[derive(Clone, Debug, Default)]
pub struct Command {
    pub name: String,
    pub modifier: Option<CommandModifier>,
    pub arguments: Box<[(String, String)]>,
}

/// Indicates that the responsibility of
/// repeats etc. is already taken care of
#[derive(Clone, Debug, Default)]
pub struct UnmodifiedCommand {
    pub name: String,
    pub arguments: Box<[(String, String)]>,
}

unsafe impl Send for Command {}
unsafe impl Sync for Command {}

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
}

pub fn command_argument_map(
    command_arguments: &[(String, String)],
) -> serde_json::Map<String, serde_json::Value> {
    use core::str::FromStr;

    command_arguments
        .iter()
        .fold(serde_json::Map::default(), |mut acc, (k, v)| {
            let v = match k.as_ref() {
                "code" | "slot" => serde_json::Value::String(v.clone()),
                _ => serde_json::Value::Number(serde_json::Number::from_str(v).unwrap()),
            };
            let res = acc.insert(k.clone(), v);
            debug_assert!(res.is_none(), "duplicate key in arg_map!");
            acc
        })
}

#[derive(Debug)]
pub enum BankActionResponseDataSchema {
    DepositGold(BankGoldTransactionSchema),
    DepositItem(BankItemTransactionSchema),
    WithdrawItem(BankItemTransactionSchema),
    WithdrawGold(BankGoldTransactionSchema),
}

impl BankActionResponseDataSchema {
    pub fn command_name(&self) -> &'static str {
        match self {
            BankActionResponseDataSchema::DepositGold(_) => "bank-deposit-gold",
            BankActionResponseDataSchema::DepositItem(_) => "bank-deposit-item",
            BankActionResponseDataSchema::WithdrawItem(_) => "bank-withdraw-item",
            BankActionResponseDataSchema::WithdrawGold(_) => "bank-withdraw-gold",
        }
    }
    pub fn cooldown_schema(&self) -> &CooldownSchema {
        match self {
            BankActionResponseDataSchema::DepositGold(gold_transaction_schema)
            | BankActionResponseDataSchema::WithdrawGold(gold_transaction_schema) => {
                &gold_transaction_schema.cooldown
            }
            BankActionResponseDataSchema::DepositItem(item_transaction_schema)
            | BankActionResponseDataSchema::WithdrawItem(item_transaction_schema) => {
                &item_transaction_schema.cooldown
            }
        }
    }
    pub fn character_schema(&self) -> &CharacterSchema {
        match self {
            BankActionResponseDataSchema::DepositGold(gold_transaction_schema)
            | BankActionResponseDataSchema::WithdrawGold(gold_transaction_schema) => {
                &gold_transaction_schema.character
            }
            BankActionResponseDataSchema::DepositItem(item_transaction_schema)
            | BankActionResponseDataSchema::WithdrawItem(item_transaction_schema) => {
                &item_transaction_schema.character
            }
        }
    }
}

#[derive(Debug)]
pub enum NpcActionResponseDataSchema {
    BuyItem(NpcMerchantTransactionSchema),
    SellItem(NpcMerchantTransactionSchema),
}

impl NpcActionResponseDataSchema {
    pub fn command_name(&self) -> &'static str {
        match self {
            NpcActionResponseDataSchema::BuyItem(_) => "npc-buy",
            NpcActionResponseDataSchema::SellItem(_) => "npc-sell",
        }
    }
    pub fn cooldown_schema(&self) -> &CooldownSchema {
        match self {
            NpcActionResponseDataSchema::BuyItem(npc_merchant_transaction_schema)
            | NpcActionResponseDataSchema::SellItem(npc_merchant_transaction_schema) => {
                &npc_merchant_transaction_schema.cooldown
            }
        }
    }
    pub fn character_schema(&self) -> &CharacterSchema {
        match self {
            NpcActionResponseDataSchema::BuyItem(npc_merchant_transaction_schema)
            | NpcActionResponseDataSchema::SellItem(npc_merchant_transaction_schema) => {
                &npc_merchant_transaction_schema.character
            }
        }
    }
}

#[derive(Debug)]
pub enum ActionResponseDataSchema {
    Move(CharacterMovementDataSchema),
    Fight(CharacterFightDataSchema),
    Rest(CharacterRestDataSchema),
    Gather(SkillDataSchema),
    Unequip(EquipmentTransactionSchema),
    Craft(SkillDataSchema),
    Equip(EquipmentTransactionSchema),
    Recycle(RecyclingDataSchema),
    UseItem(UseItemSchema),
    BankAction(BankActionResponseDataSchema),
    NpcAction(NpcActionResponseDataSchema),
}

impl ActionResponseDataSchema {
    pub fn command_name(&self) -> &'static str {
        match self {
            ActionResponseDataSchema::Move(_) => "move",
            ActionResponseDataSchema::Fight(_) => "fight",
            ActionResponseDataSchema::Rest(_) => "rest",
            ActionResponseDataSchema::Gather(_) => "gather",
            ActionResponseDataSchema::Unequip(_) => "unequip",
            ActionResponseDataSchema::Craft(_) => "craft",
            ActionResponseDataSchema::Equip(_) => "equip",
            ActionResponseDataSchema::Recycle(_) => "recycle",
            ActionResponseDataSchema::UseItem(_) => "use",
            ActionResponseDataSchema::BankAction(bank_action_response_data_schema) => {
                bank_action_response_data_schema.command_name()
            }
            ActionResponseDataSchema::NpcAction(npc_action_response_data_schema) => {
                npc_action_response_data_schema.command_name()
            }
        }
    }
    pub fn cooldown_schema(&self) -> Option<&CooldownSchema> {
        match self {
            ActionResponseDataSchema::Move(character_movement_data_schema) => {
                Some(&character_movement_data_schema.cooldown)
            }
            ActionResponseDataSchema::Fight(character_fight_data_schema) => {
                Some(&character_fight_data_schema.cooldown)
            }
            ActionResponseDataSchema::Rest(character_rest_data_schema) => {
                Some(&character_rest_data_schema.cooldown)
            }
            ActionResponseDataSchema::Gather(skill_data_schema)
            | ActionResponseDataSchema::Craft(skill_data_schema) => {
                Some(&skill_data_schema.cooldown)
            }
            ActionResponseDataSchema::Unequip(equipment_transaction_schema)
            | ActionResponseDataSchema::Equip(equipment_transaction_schema) => {
                Some(&equipment_transaction_schema.cooldown)
            }
            ActionResponseDataSchema::Recycle(recycling_data_schema) => {
                Some(&recycling_data_schema.cooldown)
            }
            ActionResponseDataSchema::UseItem(use_item_schema) => Some(&use_item_schema.cooldown),
            ActionResponseDataSchema::BankAction(bank_action_response_data_schema) => {
                Some(bank_action_response_data_schema.cooldown_schema())
            }
            ActionResponseDataSchema::NpcAction(npc_action_response_data_schema) => {
                Some(npc_action_response_data_schema.cooldown_schema())
            }
        }
    }
    pub fn map_schema(&self) -> Option<&MapSchema> {
        if let ActionResponseDataSchema::Move(character_movement_data_schema) = self {
            Some(&character_movement_data_schema.destination)
        } else {
            None
        }
    }
    /// The character-name is only used in the case that we do not have a single character.
    ///
    /// For now, that only implies the [Fight](ActionResponseDataSchema::Fight) case,
    /// as it could have been a multifight
    pub fn character_schema(&self, character_name: &str) -> Option<&CharacterSchema> {
        match self {
            ActionResponseDataSchema::Move(_) => None,
            ActionResponseDataSchema::Fight(character_fight_data_schema) => {
                character_fight_data_schema
                    .characters
                    .iter()
                    .find(|character_schema| character_schema.name.as_ref() == character_name)
            }
            ActionResponseDataSchema::Rest(character_rest_data_schema) => {
                Some(&character_rest_data_schema.character)
            }
            ActionResponseDataSchema::Gather(skill_data_schema)
            | ActionResponseDataSchema::Craft(skill_data_schema) => {
                Some(&skill_data_schema.character)
            }
            ActionResponseDataSchema::Unequip(equipment_transaction_schema)
            | ActionResponseDataSchema::Equip(equipment_transaction_schema) => {
                Some(&equipment_transaction_schema.character)
            }
            ActionResponseDataSchema::Recycle(recycling_data_schema) => {
                Some(&recycling_data_schema.character)
            }
            ActionResponseDataSchema::UseItem(use_item_schema) => Some(&use_item_schema.character),
            ActionResponseDataSchema::BankAction(bank_action_response_data_schema) => {
                Some(bank_action_response_data_schema.character_schema())
            }
            ActionResponseDataSchema::NpcAction(npc_action_response_data_schema) => {
                Some(npc_action_response_data_schema.character_schema())
            }
        }
    }
}

// below is more application networking logic i guess

fn apply_headers(
    req: ureq::RequestBuilder<ureq::typestate::WithBody>,
    api_key: &str,
) -> ureq::RequestBuilder<ureq::typestate::WithBody> {
    req.header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", api_key))
}

#[derive(Debug, Error)]
pub enum ActionRequestError {
    #[error("unrecognized command: {0}")]
    UnrecognizedCommand(String),

    #[error("something with the api: {0:?}")]
    ApiError(CodedErrorObject),

    #[error("failed to construct request: {0}")]
    UreqHttp(#[from] ureq::http::Error),

    #[error("request error: {0}")]
    Ureq(#[from] ureq::Error),

    #[error("serde error: {0}")]
    Serde(#[from] serde_json::Error),
}

fn action_request_uri(
    character_name: &str,
    action_name: &str,
) -> Result<ureq::http::Uri, ActionRequestError> {
    ureq::http::Uri::builder()
        .scheme("https")
        .authority("api.artifactsmmo.com")
        .path_and_query(format!("/my/{character_name}/action/{action_name}",))
        .build()
        .map_err(Into::into)
}

fn action_post_request_builder(
    api_key: &str,
    character_name: &str,
    action_name: &str,
) -> Result<ureq::RequestBuilder<ureq::typestate::WithBody>, ActionRequestError> {
    action_request_uri(character_name, action_name)
        .inspect_err(|e| tracing::error!(?e))
        .map(ureq::post)
        .map(|req| {
            // apply the required headers, along with the api-key
            req.header("Accept", "application/json")
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {api_key}"))
                .config()
                // make sure to config the request to not
                // eat the error payload!
                .http_status_as_error(false)
                .build()
        })
}

fn send_action_request_inner(
    api_key: &str,
    character_name: &str,
    action_name: &str,
    data: &[u8],
) -> Result<ureq::http::Response<ureq::Body>, ActionRequestError> {
    action_post_request_builder(api_key, character_name, action_name)
        .and_then(|req| req.send(data).map_err(Into::into))
}

#[tracing::instrument(level = "info", skip(api_key))]
pub fn send_action_request<Response>(
    api_key: &str,
    character_name: &str,
    action_name: &str,
    data: &[u8],
) -> Result<Response, ActionRequestError>
where
    Response: serde::de::DeserializeOwned,
{
    send_action_request_inner(api_key, character_name, action_name, data).and_then(|res| {
        let status = res.status();
        tracing::info!("status is {status}");
        let rdr = res.into_body().into_reader();
        if status.is_client_error() | status.is_server_error() {
            /// Simple wrapper type that the API returns in this case, no need
            /// to pollute local code to have this functionality
            #[derive(Debug, Default, Deserialize, Serialize)]
            struct CodedErrorObjectResponse {
                error: CodedErrorObject,
            }

            let output: CodedErrorObjectResponse = serde_json::from_reader(rdr)?;
            Err(ActionRequestError::ApiError(output.error))
        } else {
            tracing::info!("status is good!");
            let output: Response = serde_json::from_reader(rdr)?;
            Ok(output)
        }
    })
}

/// This should be mapped to individual error-types but
/// represents an all purpose error type for most of the
/// api-calls
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct CodedErrorObject {
    pub code: u64,
    pub message: Box<str>,
    pub data: Option<serde_json::Value>,
}

pub struct ErrorData {
    pub message: Box<str>,
    pub data: Option<serde_json::Value>,
}

/// code-field is omitted from variants
pub enum ActionMoveError {
    /// Code 404
    MapNotFound(ErrorData),
}

/// sends out move-request, is blocking
pub fn action_move_command_handler(
    api_key: &str,
    character_name: &str,
    command_arguments: &[(String, String)],
) -> Result<ActionResponseDataSchema, ActionRequestError> {
    serde_json::to_vec(&command_argument_map(command_arguments))
        .map_err(Into::into)
        .and_then(|data| {
            send_action_request::<CharacterMovementResponseSchema>(
                api_key,
                character_name,
                "move",
                &data,
            )
            .map(|res| ActionResponseDataSchema::Move(res.data))
        })
}

/// blocking request
pub fn action_fight_command_handler(
    api_key: &str,
    character_name: &str,
) -> Result<ActionResponseDataSchema, ActionRequestError> {
    send_action_request::<CharacterFightResponseSchema>(api_key, character_name, "fight", &[])
        .map(|res| ActionResponseDataSchema::Fight(res.data))
}

/// blocking request
pub fn action_rest_command_handler(
    api_key: &str,
    character_name: &str,
) -> Result<ActionResponseDataSchema, ActionRequestError> {
    send_action_request::<CharacterRestResponseSchema>(api_key, character_name, "rest", &[])
        .map(|res| ActionResponseDataSchema::Rest(res.data))
}

/// blocking request
pub fn action_gather_command_handler(
    api_key: &str,
    character_name: &str,
) -> Result<ActionResponseDataSchema, ActionRequestError> {
    send_action_request::<SkillResponseSchema>(api_key, character_name, "gathering", &[])
        .map(|res| ActionResponseDataSchema::Gather(res.data))
}

/// sends out unequip-request, is blocking
///
/// commands are currently only supported as a naive single set
pub fn action_unequip_command_handler(
    api_key: &str,
    character_name: &str,
    command_arguments: &[(String, String)],
) -> Result<ActionResponseDataSchema, ActionRequestError> {
    serde_json::to_vec(&[command_argument_map(command_arguments)])
        .map_err(Into::into)
        .and_then(|data| {
            send_action_request::<EquipmentResponseSchema>(
                api_key,
                character_name,
                "unequip",
                &data,
            )
            .map(|res| ActionResponseDataSchema::Unequip(res.data))
        })
}

/// sends out craft-request, is blocking
///
/// commands are currently only supported as a naive single set
pub fn action_craft_command_handler(
    api_key: &str,
    character_name: &str,
    command_arguments: &[(String, String)],
) -> Result<ActionResponseDataSchema, ActionRequestError> {
    serde_json::to_vec(&command_argument_map(command_arguments))
        .map_err(Into::into)
        .and_then(|data| {
            send_action_request::<SkillResponseSchema>(api_key, character_name, "crafting", &data)
                .map(|res| ActionResponseDataSchema::Craft(res.data))
        })
}

/// sends out equip-request, is blocking
///
/// commands are currently only supported as a naive single set
pub fn action_equip_command_handler(
    api_key: &str,
    character_name: &str,
    command_arguments: &[(String, String)],
) -> Result<ActionResponseDataSchema, ActionRequestError> {
    serde_json::to_vec(&[command_argument_map(command_arguments)])
        .map_err(Into::into)
        .and_then(|data| {
            send_action_request::<EquipmentResponseSchema>(api_key, character_name, "equip", &data)
                .map(|res| ActionResponseDataSchema::Equip(res.data))
        })
}

/// sends out recycle-request, is blocking
///
/// commands are currently only supported as a naive single set
pub fn action_recycle_command_handler(
    api_key: &str,
    character_name: &str,
    command_arguments: &[(String, String)],
) -> Result<ActionResponseDataSchema, ActionRequestError> {
    serde_json::to_vec(&command_argument_map(command_arguments))
        .map_err(Into::into)
        .and_then(|data| {
            send_action_request::<RecyclingResponseSchema>(
                api_key,
                character_name,
                "recycle",
                &data,
            )
            .map(|res| ActionResponseDataSchema::Recycle(res.data))
        })
}

/// sends out recycle-request, is blocking
///
/// commands are currently only supported as a naive single set
pub fn action_use_command_handler(
    api_key: &str,
    character_name: &str,
    command_arguments: &[(String, String)],
) -> Result<ActionResponseDataSchema, ActionRequestError> {
    serde_json::to_vec(&command_argument_map(command_arguments))
        .map_err(Into::into)
        .and_then(|data| {
            send_action_request::<UseItemResponseSchema>(api_key, character_name, "use", &data)
                .map(|res| ActionResponseDataSchema::UseItem(res.data))
        })
}

pub fn action_bank_deposit_gold_command_handler(
    api_key: &str,
    character_name: &str,
    command_arguments: &[(String, String)],
) -> Result<ActionResponseDataSchema, ActionRequestError> {
    serde_json::to_vec(&[command_argument_map(command_arguments)])
        .map_err(Into::into)
        .and_then(|data| {
            send_action_request::<BankGoldTransactionResponseSchema>(
                api_key,
                character_name,
                "bank/deposit/gold",
                &data,
            )
            .map(|res| {
                ActionResponseDataSchema::BankAction(BankActionResponseDataSchema::DepositGold(
                    res.data,
                ))
            })
        })
}

pub fn action_bank_withdraw_gold_command_handler(
    api_key: &str,
    character_name: &str,
    command_arguments: &[(String, String)],
) -> Result<ActionResponseDataSchema, ActionRequestError> {
    serde_json::to_vec(&[command_argument_map(command_arguments)])
        .map_err(Into::into)
        .and_then(|data| {
            send_action_request::<BankGoldTransactionResponseSchema>(
                api_key,
                character_name,
                "bank/withdraw/gold",
                &data,
            )
            .map(|res| {
                ActionResponseDataSchema::BankAction(BankActionResponseDataSchema::WithdrawGold(
                    res.data,
                ))
            })
        })
}

pub fn action_bank_deposit_item_command_handler(
    api_key: &str,
    character_name: &str,
    command_arguments: &[(String, String)],
) -> Result<ActionResponseDataSchema, ActionRequestError> {
    serde_json::to_vec(&[command_argument_map(command_arguments)])
        .map_err(Into::into)
        .and_then(|data| {
            send_action_request::<BankItemTransactionResponseSchema>(
                api_key,
                character_name,
                "bank/deposit/item",
                &data,
            )
            .map(|res| {
                ActionResponseDataSchema::BankAction(BankActionResponseDataSchema::DepositItem(
                    res.data,
                ))
            })
        })
}

pub fn action_bank_withdraw_item_command_handler(
    api_key: &str,
    character_name: &str,
    command_arguments: &[(String, String)],
) -> Result<ActionResponseDataSchema, ActionRequestError> {
    serde_json::to_vec(&[command_argument_map(command_arguments)])
        .map_err(Into::into)
        .and_then(|data| {
            send_action_request::<BankItemTransactionResponseSchema>(
                api_key,
                character_name,
                "bank/withdraw/item",
                &data,
            )
            .map(|res| {
                ActionResponseDataSchema::BankAction(BankActionResponseDataSchema::WithdrawItem(
                    res.data,
                ))
            })
        })
}

pub fn command_handler_router(
    api_key: &str,
    character_name: &str,
    command_name: &str,
    command_arguments: &[(String, String)],
) -> Result<ActionResponseDataSchema, ActionRequestError> {
    tracing::info!("routing command {command_name}");
    match command_name {
        "move" => action_move_command_handler(api_key, character_name, command_arguments),
        "fight" => action_fight_command_handler(api_key, character_name),
        "rest" => action_rest_command_handler(api_key, character_name),
        "gather" => action_gather_command_handler(api_key, character_name),
        "unequip" => action_unequip_command_handler(api_key, character_name, command_arguments),
        "craft" => action_craft_command_handler(api_key, character_name, command_arguments),
        "equip" => action_equip_command_handler(api_key, character_name, command_arguments),
        "recycle" => action_recycle_command_handler(api_key, character_name, command_arguments),
        "use" => action_use_command_handler(api_key, character_name, command_arguments),
        "bank-deposit-gold" => {
            action_bank_deposit_gold_command_handler(api_key, character_name, command_arguments)
        }
        "bank-deposit-item" => {
            action_bank_deposit_item_command_handler(api_key, character_name, command_arguments)
        }
        "bank-withdraw-item" => {
            action_bank_withdraw_item_command_handler(api_key, character_name, command_arguments)
        }
        "bank-withdraw-gold" => {
            action_bank_withdraw_gold_command_handler(api_key, character_name, command_arguments)
        }
        otherwise => Err(ActionRequestError::UnrecognizedCommand(otherwise.into())),
    }
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
    command_name: String,
    command_arguments: Option<Box<[(String, String)]>>,
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

#[derive(Debug)]
pub struct ArgParser<'a, I>
where
    I: Iterator<Item = String>,
{
    args: &'a mut I,
    cached_command_name: Option<String>,
}

impl<'a, I> ArgParser<'a, I>
where
    I: Iterator<Item = String>,
{
    pub fn new(args: &'a mut I) -> ArgParser<'a, I> {
        ArgParser {
            args,
            cached_command_name: None,
        }
    }
    #[tracing::instrument(level = "debug", skip(self, is_arg_valid), ret)]
    fn parse_next_arg(
        &mut self,
        is_arg_valid: impl FnOnce(&str) -> bool,
    ) -> Option<(String, String)> {
        // let first_val = self.fst.take().or_else(|| self.args.next())?;
        let first_val = self.args.next()?;
        tracing::info!("parsing next argument with first-value {first_val:?}");
        if !is_arg_valid(&first_val) {
            tracing::warn!(
                "{first_val} not a valid arg for current command! setting to remainder slot!"
            );
            self.cached_command_name = Some(first_val);
            None
        } else {
            parse_request_argument(&first_val, || self.args.next())
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
            self.args.next()?
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
                self.args.next()?,
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
    #[test]
    fn serialize_equip_request_body_entry() {
        let input = "equip -code copper_pickaxe -slot weapon";
        let entry = EquipRequestBodyEntry {
            code: "copper_pickaxe".into(),
            slot: ItemSlot::Weapon,
            quantity: None,
        };
        // is_valid_input_arg_for_request_object(input);
    }
}
