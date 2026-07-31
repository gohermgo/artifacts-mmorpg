use super::*;

use artifacts_api::{
    BankGoldTransactionResponseSchema, BankItemTransactionResponseSchema,
    CharacterFightResponseSchema, CharacterMovementResponseSchema, CharacterRestResponseSchema,
    EquipmentResponseSchema, RecyclingResponseSchema, SkillResponseSchema, UseItemResponseSchema,
};

fn command_argument_map(
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

/// sends out move-request, is blocking
fn action_move_command_handler(
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
fn action_fight_command_handler(
    api_key: &str,
    character_name: &str,
) -> Result<ActionResponseDataSchema, ActionRequestError> {
    send_action_request::<CharacterFightResponseSchema>(api_key, character_name, "fight", &[])
        .map(|res| ActionResponseDataSchema::Fight(res.data))
}

/// blocking request
fn action_rest_command_handler(
    api_key: &str,
    character_name: &str,
) -> Result<ActionResponseDataSchema, ActionRequestError> {
    send_action_request::<CharacterRestResponseSchema>(api_key, character_name, "rest", &[])
        .map(|res| ActionResponseDataSchema::Rest(res.data))
}

/// blocking request
fn action_gather_command_handler(
    api_key: &str,
    character_name: &str,
) -> Result<ActionResponseDataSchema, ActionRequestError> {
    send_action_request::<SkillResponseSchema>(api_key, character_name, "gathering", &[])
        .map(|res| ActionResponseDataSchema::Gather(res.data))
}

/// sends out unequip-request, is blocking
///
/// commands are currently only supported as a naive single set
fn action_unequip_command_handler(
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
fn action_craft_command_handler(
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
fn action_equip_command_handler(
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
fn action_recycle_command_handler(
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
fn action_use_command_handler(
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

fn action_bank_deposit_gold_command_handler(
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

fn action_bank_withdraw_gold_command_handler(
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

fn action_bank_deposit_item_command_handler(
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

fn action_bank_withdraw_item_command_handler(
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
