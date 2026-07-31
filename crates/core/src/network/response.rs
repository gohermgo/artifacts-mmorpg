use artifacts_api::{
    BankGoldTransactionSchema, BankItemTransactionSchema, CharacterFightDataSchema,
    CharacterMovementDataSchema, CharacterRestDataSchema, CharacterSchema, CooldownSchema,
    EquipmentTransactionSchema, MapSchema, NpcMerchantTransactionSchema, RecyclingDataSchema,
    SkillDataSchema, UseItemSchema,
};

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

#[derive(Debug)]
pub enum BankActionResponseDataSchema {
    DepositGold(BankGoldTransactionSchema),
    DepositItem(BankItemTransactionSchema),
    WithdrawItem(BankItemTransactionSchema),
    WithdrawGold(BankGoldTransactionSchema),
}

#[derive(Debug)]
pub enum NpcActionResponseDataSchema {
    BuyItem(NpcMerchantTransactionSchema),
    SellItem(NpcMerchantTransactionSchema),
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
