use chrono::{DateTime, Utc};
use core::str::FromStr;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MapLayer {
    Overworld,
    Underground,
    Interior,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionType {
    Movement,
    Fight,
    RaidFight,
    MultiFight,
    Crafting,
    Gathering,
    BuyGe,
    SellGe,
    CreateBuyOrderGe,
    FillBuyOrderGe,
    BuyNpc,
    SellNpc,
    CancelGe,
    DeleteItem,
    DepositItem,
    WithdrawItem,
    DepositGold,
    WithdrawGold,
    Equip,
    Unequip,
    Task,
    Recycling,
    Rest,
    Use,
    BuyBankExpansion,
    GiveItem,
    GiveGold,
    RaidDeposit,
    ChangeSkin,
    Rename,
    Transition,
    ClaimItem,
    SandboxGiveGold,
    SandboxGiveItem,
    SandboxGiveXp,
    SandboxClearCooldown,
    SandboxTeleport,
}

pub struct Character {
    pub name: &'static str,
}

pub enum CharacterActionKind {
    Move,
    Fight,
    Rest,
    Gathering,
}

impl Character {
    pub fn base_url(&self) -> Result<url::Url, url::ParseError> {
        url::Url::from_str(&format!("https://api.artifactsmmo.com/my/{}/", self.name))
    }
    pub fn action_url(
        &self,
        action_kind: CharacterActionKind,
    ) -> Result<url::Url, url::ParseError> {
        self.base_url().and_then(|u| {
            u.join(&format!(
                "action/{}",
                match action_kind {
                    CharacterActionKind::Move => "move",
                    CharacterActionKind::Fight => "fight",
                    CharacterActionKind::Rest => "rest",
                    CharacterActionKind::Gathering => "gathering",
                }
            ))
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterMovementResponseSchema {
    pub data: CharacterMovementDataSchema,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterMovementDataSchema {
    pub cooldown: CooldownSchema,
    pub destination: MapSchema,
    /// Path taken from start to destination (list of coordinates)
    pub path: Box<[[i32; 2]]>,
}
/// Represents the details of a game map, including its location, skin, and available interactions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapSchema {
    /// ID of the map.
    pub map_id: i64,
    /// Name of the map.
    pub name: Box<str>,
    /// Skin of the map.
    pub skin: Box<str>,
    /// Position X of the map.
    pub x: i64,
    /// Position Y of the map.
    pub y: i64,
    /// Layer of the map.
    pub layer: MapLayer,
    /// Access information for the map
    pub access: AccessSchema,
    /// Interactions available on this map.
    pub interactions: InteractionSchema,
}

/// Access information for the map, determining movement rules and any conditions required.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessSchema {
    /// Map access type determining movement and accessibility
    pub r#type: MapAccessType,
    /// Access conditions for the map
    pub conditions: Option<Box<[ConditionSchema]>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MapAccessType {
    Standard,
    Restricted,
    Conditional,
    Blocked,
}

/// A single condition that must be met, such as for map access or transitions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionSchema {
    /// Condition code.
    pub code: Box<str>,
    /// Condition operator.
    pub operator: ConditionOperator,
    /// Condition value.
    pub value: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConditionOperator {
    Eq,
    Ne,
    Gt,
    Lt,
    Cost,
    HasItem,
    AchievementUnlocked,
}

/// Represents the interactions available on a map, including content and any transitions to other maps.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractionSchema {
    /// Content of the map.
    pub content: Option<MapContentSchema>,
    /// Transition to another map.
    pub transition: Option<TransitionSchema>,
}

/// Represents a piece of content found on a map, such as a monster, resource, or NPC.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapContentSchema {
    /// Type of the content.
    pub r#type: MapContentType,
    /// Code of the content.
    pub code: Box<str>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MapContentType {
    Monster,
    Resource,
    Workshop,
    Bank,
    GrandExchange,
    TasksMaster,
    Npc,
    Raid,
}

/// Describes a transition from one map to another, including destination and any conditions required.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionSchema {
    /// ID of the destination map.
    pub map_id: i64,
    /// Position X of the destination.
    pub x: i64,
    /// Position Y of the destination.
    pub y: i64,
    /// Layer of the destination.
    pub layer: MapLayer,
    /// Conditions for the transition.
    pub conditions: Option<Box<[ConditionSchema]>>,
}

/// Represents the cooldown status of a character action, including timing and reason.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CooldownSchema {
    /// The total seconds of the cooldown.
    pub total_seconds: i64,
    /// The remaining seconds of the cooldown.
    pub remaining_seconds: i64,
    /// The start of the cooldown.
    pub started_at: Box<str>,
    /// The expiration of the cooldown.
    pub expiration: Box<str>,
    /// The reason of the cooldown.
    pub reason: ActionType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterFightResponseSchema {
    pub data: CharacterFightDataSchema,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterFightDataSchema {
    /// Cooldown details
    pub cooldown: CooldownSchema,
    /// Character fight details
    pub fight: CharacterFightSchema,
    /// All characters involved
    pub characters: Box<[CharacterSchema]>,
}

/// The result and details of a character's completed fight against a monster.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterFightSchema {
    /// The result of the fight.
    pub result: FightResult,
    /// Numbers of the turns of the combat.
    pub turns: i64,
    /// The code of the monster fought.
    pub opponent: Box<str>,
    /// The fight logs.
    pub logs: Box<[Box<str>]>,
    /// Results for each character.
    pub characters: Box<[CharacterMultiFightResultSchema]>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FightResult {
    Win,
    Loss,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterMultiFightResultSchema {
    /// Name of the character
    pub character_name: Box<str>,
    /// XP gained by this character
    pub xp: i64,
    /// Gold gained by this character
    pub gold: i64,
    /// Items dropped for this character.
    pub drops: Box<[DropSchema]>,
    /// Charater's hp at the end of combat
    pub final_hp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DropSchema {
    /// The code of the item.
    pub code: Box<str>,
    /// The quantity of the item.
    pub quantity: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterRestResponseSchema {
    pub data: CharacterRestDataSchema,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterRestDataSchema {
    pub cooldown: CooldownSchema,
    pub hp_restored: i64,
    pub character: CharacterSchema,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillResponseSchema {
    pub data: SkillDataSchema,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDataSchema {
    pub cooldown: CooldownSchema,
    pub details: SkillInfoSchema,
    pub character: CharacterSchema,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillInfoSchema {
    pub xp: i64,
    pub items: Box<[DropSchema]>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquipmentResponseSchema {
    pub data: EquipmentTransactionSchema,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquipmentTransactionSchema {
    /// Cooldown details
    pub cooldown: CooldownSchema,
    /// Items details
    pub items: Box<[EquipmentItemSchema]>,
    /// Player details
    pub character: CharacterSchema,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquipmentItemSchema {
    /// ItemSlot
    pub slot: ItemSlot,
    /// Item details
    pub item: ItemSchema,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemSlot {
    Weapon,
    Shield,
    Helmet,
    BodyArmor,
    LegArmor,
    Boots,
    #[serde(rename = "ring1")]
    Ring1,
    #[serde(rename = "ring2")]
    Ring2,
    Amulet,
    #[serde(rename = "artifact1")]
    Artifact1,
    #[serde(rename = "artifact2")]
    Artifact2,
    #[serde(rename = "artifact3")]
    Artifact3,
    #[serde(rename = "utility1")]
    Utility1,
    #[serde(rename = "utility2")]
    Utility2,
    Bag,
    Rune,
}

/// Schema for an in-game item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemSchema {
    /// Item name.
    pub name: Box<str>,

    /// Item code. This is the item's unique identifier (ID).
    pub code: Box<str>,

    /// Item level.
    pub level: i64,

    /// Item type.
    pub r#type: Box<str>,

    /// Item subtype.
    pub subtype: Box<str>,

    /// Item description.
    pub description: Box<str>,

    /// Item conditions. If applicable. Conditions for using or equipping the item.
    #[serde(default)]
    pub conditions: Option<Box<[ConditionSchema]>>,

    /// List of object effects. For equipment, it will include item stats.
    #[serde(default)]
    pub effects: Option<Box<[SimpleEffectSchema]>>,

    /// Craft information. If applicable.
    #[serde(default)]
    pub craft: Option<CraftSchema>,

    /// Item tradeable status. A non-tradeable item cannot be exchanged or sold.
    pub tradeable: bool,

    /// Item recyclable status. A recyclable item can be recycled at the matching workshop.
    #[serde(default)]
    pub recyclable: Option<bool>,
}

/// Description of a game effect, tied to items, monsters, etc.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleEffectSchema {
    /// Effect code.
    pub code: Box<str>,

    /// Effect value.
    pub value: i64,

    /// Description of the effect.
    pub description: Box<str>,
}

/// Crafting requirements and output for an item, if it can be crafted.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CraftSchema {
    /// Skill required to craft the item.
    #[serde(default)]
    pub skill: Option<CraftSkill>,

    /// The skill level required to craft the item.
    #[serde(default)]
    pub level: Option<i64>,

    /// List of items required to craft the item.
    #[serde(default)]
    pub items: Option<Box<[SimpleItemSchema]>>,

    /// Quantity of items crafted.
    #[serde(default)]
    pub quantity: Option<i64>,
}

/// Skill code required to craft an item.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CraftSkill {
    Weaponcrafting,
    Gearcrafting,
    Jewelrycrafting,
    Cooking,
    Woodcutting,
    Mining,
    Alchemy,
}

/// A simple item reference with a code and quantity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleItemSchema {
    /// Item code.
    pub code: Box<str>,

    /// Item quantity.
    pub quantity: i64,
}

/// The items were successfully recycled
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecyclingResponseSchema {
    /// Recycling data
    pub data: RecyclingDataSchema,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecyclingDataSchema {
    /// Cooldown details
    pub cooldown: CooldownSchema,
    /// Craft details
    pub details: RecyclingItemsSchema,
    /// Player details
    pub character: CharacterSchema,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecyclingItemsSchema {
    /// Objects received
    pub items: Box<[DropSchema]>,
    /// Whether enhanced recycling was used
    #[serde(default)]
    pub enhanced: Option<bool>,
    /// Gold spent for enhanced recycling
    #[serde(default)]
    pub gold: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UseItemResponseSchema {
    pub data: UseItemSchema,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UseItemSchema {
    /// Cooldown details
    pub cooldown: CooldownSchema,
    /// Item details
    pub item: ItemSchema,
    /// Player details
    pub character: CharacterSchema,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BankItemTransactionResponseSchema {
    pub data: BankItemTransactionSchema,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BankItemTransactionSchema {
    /// Cooldown details
    pub cooldown: CooldownSchema,
    /// Item details
    pub items: Box<[SimpleItemSchema]>,
    /// Items in your banks
    pub bank: Box<[SimpleItemSchema]>,
    /// Character details
    pub character: CharacterSchema,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BankGoldTransactionResponseSchema {
    pub data: BankGoldTransactionSchema,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BankGoldTransactionSchema {
    /// Cooldown details
    pub cooldown: CooldownSchema,
    /// Bank details
    pub bank: GoldSchema,
    /// Character details
    pub character: CharacterSchema,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoldSchema {
    /// Quantity of gold
    pub quantity: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcMerchantTransactionResponseSchema {
    pub data: NpcMerchantTransactionSchema,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcMerchantTransactionSchema {
    /// Cooldown details
    pub cooldown: CooldownSchema,
    /// Transaction details
    pub transaction: NpcItemTransactionSchema,
    /// Character details
    pub character: CharacterSchema,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcItemTransactionSchema {
    /// Item code
    pub code: Box<str>,
    /// Item quantity
    pub quantity: u64,
    /// The currency used for the transaction
    pub currency: Box<str>,
    /// Item price
    pub price: u64,
    /// Total price of the transaction
    pub total_price: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterResponse {
    pub data: CharacterSchema,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterSchema {
    pub name: Box<str>,
    pub account: Box<str>,
    /// Character skin code
    pub skin: Box<str>,
    pub level: i32,
    pub xp: i32,
    /// XP required to level up the character
    pub max_xp: i32,
    pub gold: i32,
    pub speed: i32,

    pub mining_level: i32,
    pub mining_xp: i32,
    pub mining_max_xp: i32,
    pub woodcutting_level: i32,
    pub woodcutting_xp: i32,
    pub woodcutting_max_xp: i32,
    pub fishing_level: i32,
    pub fishing_xp: i32,
    pub fishing_max_xp: i32,
    pub weaponcrafting_level: i32,
    pub weaponcrafting_xp: i32,
    pub weaponcrafting_max_xp: i32,
    pub gearcrafting_level: i32,
    pub gearcrafting_xp: i32,
    pub gearcrafting_max_xp: i32,
    pub jewelrycrafting_level: i32,
    pub jewelrycrafting_xp: i32,
    pub jewelrycrafting_max_xp: i32,
    pub cooking_level: i32,
    pub cooking_xp: i32,
    pub cooking_max_xp: i32,
    pub alchemy_level: i32,
    pub alchemy_xp: i32,
    pub alchemy_max_xp: i32,

    pub hp: i32,
    pub max_hp: i32,
    pub haste: i32,
    pub critical_strike: i32,
    pub wisdom: i32,
    pub prospecting: i32,
    pub initiative: i32,
    pub threat: i32,

    pub attack_fire: i32,
    pub attack_earth: i32,
    pub attack_water: i32,
    pub attack_air: i32,
    pub dmg: i32,
    pub dmg_fire: i32,
    pub dmg_earth: i32,
    pub dmg_water: i32,
    pub dmg_air: i32,
    pub res_fire: i32,
    pub res_earth: i32,
    pub res_water: i32,
    pub res_air: i32,

    /// Active buffs/debuffs currently applied to the character.
    pub effects: Box<[StorageEffectSchema]>,

    pub x: i32,
    pub y: i32,

    /// Map layer the character is standing on (e.g. "interior").
    pub layer: Box<str>,
    pub map_id: i32,

    /// Remaining cooldown in seconds.
    pub cooldown: i32,
    /// Timestamp when the current cooldown ends.
    pub cooldown_expiration: DateTime<Utc>,

    // equipment slots — empty string when unequipped, per API convention
    pub weapon_slot: Box<str>,
    pub rune_slot: Box<str>,
    pub shield_slot: Box<str>,
    pub helmet_slot: Box<str>,
    pub body_armor_slot: Box<str>,
    pub leg_armor_slot: Box<str>,
    pub boots_slot: Box<str>,
    pub ring1_slot: Box<str>,
    pub ring2_slot: Box<str>,
    pub amulet_slot: Box<str>,
    pub artifact1_slot: Box<str>,
    pub artifact2_slot: Box<str>,
    pub artifact3_slot: Box<str>,
    pub utility1_slot: Box<str>,
    pub utility1_slot_quantity: i32,
    pub utility2_slot: Box<str>,
    pub utility2_slot_quantity: i32,
    pub bag_slot: Box<str>,

    /// Code of the currently assigned task, empty if none.
    pub task: Box<str>,
    /// "monsters" | "items" | etc.
    pub task_type: Box<str>,
    pub task_progress: i32,
    pub task_total: i32,

    pub inventory_max_items: i32,
    pub inventory: Box<[InventorySlotSchema]>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageEffectSchema {
    pub code: Box<str>,
    pub value: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventorySlotSchema {
    pub slot: i32,
    pub code: Box<str>,
    pub quantity: i32,
}
