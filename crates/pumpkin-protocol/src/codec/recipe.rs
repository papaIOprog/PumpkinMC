use pumpkin_data::recipes::RecipeCategoryTypes;

use pumpkin_data::item::Item;
use pumpkin_data::item_stack::ItemStack;
use pumpkin_data::tag::Taggable;
use pumpkin_util::identifier::Identifier;
use pumpkin_util::version::JavaMinecraftVersion;

use crate::VarInt;
use crate::codec::item_stack_seralizer::ItemStackSerializer;
use crate::ser::{NetworkWriteExt, WritingError};

#[derive(Clone, Debug)]
pub enum OwnedRecipeIngredient {
    Simple(String),
    Tagged(String),
    OneOf(Vec<String>),
}

impl OwnedRecipeIngredient {
    #[must_use]
    pub fn match_item(&self, item: &Item) -> bool {
        match self {
            Self::Simple(id) => {
                let name = format!("minecraft:{}", item.registry_key);
                name == *id
            }
            Self::Tagged(tag) => item.is_tagged_with(tag).unwrap_or(false),
            Self::OneOf(ids) => {
                let name = format!("minecraft:{}", item.registry_key);
                ids.contains(&name)
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct OwnedRecipeResult {
    pub item_id: String,
    pub count: u8,
    // TODO: Add components/enchantments if needed for the display result
}

#[derive(Clone, Debug)]
pub enum OwnedCraftingRecipe {
    Shaped {
        recipe_id: Option<String>,
        category: RecipeCategoryTypes,
        group: Option<String>,
        show_notification: bool,
        key: Vec<(char, OwnedRecipeIngredient)>,
        pattern: Vec<String>,
        result: OwnedRecipeResult,
    },
    Shapeless {
        recipe_id: Option<String>,
        category: RecipeCategoryTypes,
        group: Option<String>,
        ingredients: Vec<OwnedRecipeIngredient>,
        result: OwnedRecipeResult,
    },
}

#[derive(Clone, Debug)]
pub struct OwnedCookingRecipe {
    pub recipe_id: String,
    pub category: RecipeCategoryTypes,
    pub group: Option<String>,
    pub ingredient: OwnedRecipeIngredient,
    pub cooking_time: i32,
    pub experience: f32,
    pub result: OwnedRecipeResult,
}

#[derive(Clone, Debug)]
pub enum OwnedCookingRecipeType {
    Blasting(OwnedCookingRecipe),
    Smelting(OwnedCookingRecipe),
    Smoking(OwnedCookingRecipe),
    CampfireCooking(OwnedCookingRecipe),
}

#[derive(Clone, Debug)]
pub struct OwnedBrewingRecipe {
    pub recipe_id: String,
    pub input_item: String,
    pub input_potion: Option<String>,
    pub reagent: String,
    pub output_item: String,
    pub output_potion: Option<String>,
}

#[derive(Clone, Debug)]
pub struct OwnedStonecuttingRecipe {
    pub recipe_id: String,
    pub group: Option<String>,
    pub ingredient: OwnedRecipeIngredient,
    pub result: OwnedRecipeResult,
}

#[derive(Clone, Debug)]
pub struct OwnedSmithingTransformRecipe {
    pub recipe_id: String,
    pub template: OwnedRecipeIngredient,
    pub base: OwnedRecipeIngredient,
    pub addition: OwnedRecipeIngredient,
    pub result: OwnedRecipeResult,
}

#[derive(Clone, Debug)]
pub struct OwnedSmithingTrimRecipe {
    pub recipe_id: String,
    pub template: OwnedRecipeIngredient,
    pub base: OwnedRecipeIngredient,
    pub addition: OwnedRecipeIngredient,
}

/// The `crafting_special_*` serializers plus `crafting_decorated_pot`, whose
/// network payload is only the crafting book category.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpecialRecipeType {
    ArmorDye,
    BookCloning,
    MapCloning,
    MapExtending,
    FireworkRocket,
    FireworkStar,
    FireworkStarFade,
    TippedArrow,
    BannerDuplicate,
    ShieldDecoration,
    ShulkerBoxColoring,
    SuspiciousStew,
    RepairItem,
    DecoratedPot,
}

#[derive(Clone, Debug)]
pub struct OwnedSpecialRecipe {
    pub recipe_id: String,
    pub kind: SpecialRecipeType,
    pub category: RecipeCategoryTypes,
}

#[derive(Clone, Debug)]
pub enum DynamicRecipe {
    Crafting(OwnedCraftingRecipe),
    Cooking(OwnedCookingRecipeType),
    /// Pumpkin-internal brewing mixes; never sent to the vanilla client
    /// because `brewing` is not a client recipe serializer.
    Brewing(OwnedBrewingRecipe),
    Stonecutting(OwnedStonecuttingRecipe),
    SmithingTransform(OwnedSmithingTransformRecipe),
    SmithingTrim(OwnedSmithingTrimRecipe),
    Special(OwnedSpecialRecipe),
}

impl DynamicRecipe {
    /// The recipe's key (`RecipeHolder#id`), `None` for id-less crafting
    /// recipes.
    #[must_use]
    pub fn recipe_id(&self) -> Option<&str> {
        match self {
            Self::Crafting(
                OwnedCraftingRecipe::Shaped { recipe_id, .. }
                | OwnedCraftingRecipe::Shapeless { recipe_id, .. },
            ) => recipe_id.as_deref(),
            Self::Cooking(
                OwnedCookingRecipeType::Blasting(recipe)
                | OwnedCookingRecipeType::Smelting(recipe)
                | OwnedCookingRecipeType::Smoking(recipe)
                | OwnedCookingRecipeType::CampfireCooking(recipe),
            ) => Some(&recipe.recipe_id),
            Self::Brewing(recipe) => Some(&recipe.recipe_id),
            Self::Stonecutting(recipe) => Some(&recipe.recipe_id),
            Self::SmithingTransform(recipe) => Some(&recipe.recipe_id),
            Self::SmithingTrim(recipe) => Some(&recipe.recipe_id),
            Self::Special(recipe) => Some(&recipe.recipe_id),
        }
    }

    /// Vanilla `Recipe#isSpecial`: no recipe book entry and never unlocked.
    #[must_use]
    pub const fn is_special(&self) -> bool {
        matches!(self, Self::Special(_))
    }
}

/// Numeric ids of the vanilla `recipe_serializer` registry in registration
/// order; `Recipe#STREAM_CODEC` dispatches on them via `writeById`.
const SERIALIZER_CRAFTING_SHAPED: i32 = 0;
const SERIALIZER_CRAFTING_SHAPELESS: i32 = 1;
const SERIALIZER_SPECIAL_ARMOR_DYE: i32 = 2;
const SERIALIZER_SPECIAL_BOOK_CLONING: i32 = 3;
const SERIALIZER_SPECIAL_MAP_CLONING: i32 = 4;
const SERIALIZER_SPECIAL_MAP_EXTENDING: i32 = 5;
const SERIALIZER_SPECIAL_FIREWORK_ROCKET: i32 = 6;
const SERIALIZER_SPECIAL_FIREWORK_STAR: i32 = 7;
const SERIALIZER_SPECIAL_FIREWORK_STAR_FADE: i32 = 8;
const SERIALIZER_SPECIAL_TIPPED_ARROW: i32 = 9;
const SERIALIZER_SPECIAL_BANNER_DUPLICATE: i32 = 10;
const SERIALIZER_SPECIAL_SHIELD_DECORATION: i32 = 11;
const SERIALIZER_SPECIAL_SHULKER_BOX_COLORING: i32 = 12;
const SERIALIZER_SPECIAL_SUSPICIOUS_STEW: i32 = 13;
const SERIALIZER_SPECIAL_REPAIR_ITEM: i32 = 14;
const SERIALIZER_SMELTING: i32 = 15;
const SERIALIZER_BLASTING: i32 = 16;
const SERIALIZER_SMOKING: i32 = 17;
const SERIALIZER_CAMPFIRE_COOKING: i32 = 18;
const SERIALIZER_STONECUTTING: i32 = 19;
const SERIALIZER_SMITHING_TRANSFORM: i32 = 20;
const SERIALIZER_SMITHING_TRIM: i32 = 21;
const SERIALIZER_CRAFTING_DECORATED_POT: i32 = 22;

/// Crafting book category ordinals in `CraftingBookCategory` wire order.
const CRAFTING_CATEGORY_BUILDING: i32 = 0;
const CRAFTING_CATEGORY_REDSTONE: i32 = 1;
const CRAFTING_CATEGORY_EQUIPMENT: i32 = 2;
const CRAFTING_CATEGORY_MISC: i32 = 3;

/// Cooking book category ordinals in `CookingBookCategory` wire order.
const COOKING_CATEGORY_FOOD: i32 = 0;
const COOKING_CATEGORY_BLOCKS: i32 = 1;
const COOKING_CATEGORY_MISC: i32 = 2;

const fn crafting_category_id(category: &RecipeCategoryTypes) -> i32 {
    match category {
        RecipeCategoryTypes::Building => CRAFTING_CATEGORY_BUILDING,
        RecipeCategoryTypes::Restone => CRAFTING_CATEGORY_REDSTONE,
        RecipeCategoryTypes::Equipment => CRAFTING_CATEGORY_EQUIPMENT,
        // Food/Blocks are cooking-only categories and fall back to misc here.
        RecipeCategoryTypes::Misc | RecipeCategoryTypes::Food | RecipeCategoryTypes::Blocks => {
            CRAFTING_CATEGORY_MISC
        }
    }
}

const fn cooking_category_id(category: &RecipeCategoryTypes) -> i32 {
    match category {
        RecipeCategoryTypes::Food => COOKING_CATEGORY_FOOD,
        RecipeCategoryTypes::Blocks => COOKING_CATEGORY_BLOCKS,
        _ => COOKING_CATEGORY_MISC,
    }
}

const fn special_serializer_id(kind: SpecialRecipeType) -> i32 {
    match kind {
        SpecialRecipeType::ArmorDye => SERIALIZER_SPECIAL_ARMOR_DYE,
        SpecialRecipeType::BookCloning => SERIALIZER_SPECIAL_BOOK_CLONING,
        SpecialRecipeType::MapCloning => SERIALIZER_SPECIAL_MAP_CLONING,
        SpecialRecipeType::MapExtending => SERIALIZER_SPECIAL_MAP_EXTENDING,
        SpecialRecipeType::FireworkRocket => SERIALIZER_SPECIAL_FIREWORK_ROCKET,
        SpecialRecipeType::FireworkStar => SERIALIZER_SPECIAL_FIREWORK_STAR,
        SpecialRecipeType::FireworkStarFade => SERIALIZER_SPECIAL_FIREWORK_STAR_FADE,
        SpecialRecipeType::TippedArrow => SERIALIZER_SPECIAL_TIPPED_ARROW,
        SpecialRecipeType::BannerDuplicate => SERIALIZER_SPECIAL_BANNER_DUPLICATE,
        SpecialRecipeType::ShieldDecoration => SERIALIZER_SPECIAL_SHIELD_DECORATION,
        SpecialRecipeType::ShulkerBoxColoring => SERIALIZER_SPECIAL_SHULKER_BOX_COLORING,
        SpecialRecipeType::SuspiciousStew => SERIALIZER_SPECIAL_SUSPICIOUS_STEW,
        SpecialRecipeType::RepairItem => SERIALIZER_SPECIAL_REPAIR_ITEM,
        SpecialRecipeType::DecoratedPot => SERIALIZER_CRAFTING_DECORATED_POT,
    }
}

/// Mirrors the payload of `ClientboundUpdateRecipesPacket` for 1.20.5-1.21.1:
/// a varint recipe count, then `RecipeHolder.STREAM_CODEC` per recipe (recipe
/// id, serializer registry id, serializer-specific data).
#[must_use]
pub fn encode_update_recipes(recipes: &[DynamicRecipe], version: &JavaMinecraftVersion) -> Vec<u8> {
    let mut count_writer = CountingWriter::default();
    for recipe in recipes {
        if let Some(encoded) = encode_recipe(recipe, *version) {
            count_writer.count += 1;
            let _ = count_writer.write_string(&encoded.id.to_string());
            let _ = count_writer.write_var_int(&VarInt(encoded.serializer_id));
            let _ = count_writer.write_slice(&encoded.payload);
        }
    }

    let mut out = Vec::with_capacity(count_writer.buffer.len() + 5);
    let _ = out.write_var_int(&VarInt(count_writer.count));
    let _ = out.write_slice(&count_writer.buffer);
    out
}

struct EncodedRecipe {
    id: Identifier,
    serializer_id: i32,
    payload: Vec<u8>,
}

#[allow(clippy::too_many_arguments)]
fn encode_shaped(
    recipe_id: &str,
    category: &RecipeCategoryTypes,
    group: Option<&str>,
    show_notification: bool,
    key: &[(char, OwnedRecipeIngredient)],
    pattern: &[String],
    result: &OwnedRecipeResult,
    version: JavaMinecraftVersion,
) -> Option<EncodedRecipe> {
    let id = Identifier::parse(recipe_id).ok()?;
    let mut payload = Vec::new();
    payload.write_string(group.unwrap_or("")).ok()?;
    payload
        .write_var_int(&VarInt(crafting_category_id(category)))
        .ok()?;

    // ShapedRecipePattern#toNetwork: width, height, then one ingredient per
    // pattern cell (including empty ones).
    let width = pattern.first().map_or(0, |row| row.chars().count()) as i32;
    let height = pattern.len() as i32;
    payload.write_var_int(&VarInt(width)).ok()?;
    payload.write_var_int(&VarInt(height)).ok()?;
    for row in pattern {
        for cell in row.chars() {
            if let Some((_, ingredient)) = key.iter().find(|(ch, _)| *ch == cell) {
                encode_ingredient(&mut payload, ingredient, version).ok()?;
            } else {
                payload.write_var_int(&VarInt(0)).ok()?;
            }
        }
    }

    encode_item_stack(&mut payload, result, version)?;
    payload.write_bool(show_notification).ok()?;
    Some(EncodedRecipe {
        id,
        serializer_id: SERIALIZER_CRAFTING_SHAPED,
        payload,
    })
}

fn encode_shapeless(
    recipe_id: &str,
    category: &RecipeCategoryTypes,
    group: Option<&str>,
    ingredients: &[OwnedRecipeIngredient],
    result: &OwnedRecipeResult,
    version: JavaMinecraftVersion,
) -> Option<EncodedRecipe> {
    let id = Identifier::parse(recipe_id).ok()?;
    let mut payload = Vec::new();
    payload.write_string(group.unwrap_or("")).ok()?;
    payload
        .write_var_int(&VarInt(crafting_category_id(category)))
        .ok()?;
    payload
        .write_var_int(&VarInt(ingredients.len() as i32))
        .ok()?;
    for ingredient in ingredients {
        encode_ingredient(&mut payload, ingredient, version).ok()?;
    }
    encode_item_stack(&mut payload, result, version)?;
    Some(EncodedRecipe {
        id,
        serializer_id: SERIALIZER_CRAFTING_SHAPELESS,
        payload,
    })
}

fn encode_cooking(
    cooking: &OwnedCookingRecipeType,
    version: JavaMinecraftVersion,
) -> Option<EncodedRecipe> {
    let (recipe, serializer_id) = match cooking {
        OwnedCookingRecipeType::Smelting(recipe) => (recipe, SERIALIZER_SMELTING),
        OwnedCookingRecipeType::Blasting(recipe) => (recipe, SERIALIZER_BLASTING),
        OwnedCookingRecipeType::Smoking(recipe) => (recipe, SERIALIZER_SMOKING),
        OwnedCookingRecipeType::CampfireCooking(recipe) => (recipe, SERIALIZER_CAMPFIRE_COOKING),
    };
    let id = Identifier::parse(recipe.recipe_id.as_str()).ok()?;
    let mut payload = Vec::new();
    payload
        .write_string(recipe.group.as_deref().unwrap_or(""))
        .ok()?;
    payload
        .write_var_int(&VarInt(cooking_category_id(&recipe.category)))
        .ok()?;
    encode_ingredient(&mut payload, &recipe.ingredient, version).ok()?;
    encode_item_stack(&mut payload, &recipe.result, version)?;
    payload.write_f32_be(recipe.experience).ok()?;
    payload.write_var_int(&VarInt(recipe.cooking_time)).ok()?;
    Some(EncodedRecipe {
        id,
        serializer_id,
        payload,
    })
}

/// Returns `None` for recipes without a client-side serializer (brewing) or
/// with an id the vanilla client would reject.
fn encode_recipe(recipe: &DynamicRecipe, version: JavaMinecraftVersion) -> Option<EncodedRecipe> {
    match recipe {
        DynamicRecipe::Brewing(_) => None,
        DynamicRecipe::Crafting(OwnedCraftingRecipe::Shaped {
            recipe_id,
            category,
            group,
            show_notification,
            key,
            pattern,
            result,
        }) => encode_shaped(
            recipe_id.as_deref()?,
            category,
            group.as_deref(),
            *show_notification,
            key,
            pattern,
            result,
            version,
        ),
        DynamicRecipe::Crafting(OwnedCraftingRecipe::Shapeless {
            recipe_id,
            category,
            group,
            ingredients,
            result,
        }) => encode_shapeless(
            recipe_id.as_deref()?,
            category,
            group.as_deref(),
            ingredients,
            result,
            version,
        ),
        DynamicRecipe::Cooking(cooking) => encode_cooking(cooking, version),
        DynamicRecipe::Stonecutting(recipe) => {
            let id = Identifier::parse(recipe.recipe_id.as_str()).ok()?;
            let mut payload = Vec::new();
            payload
                .write_string(recipe.group.as_deref().unwrap_or(""))
                .ok()?;
            encode_ingredient(&mut payload, &recipe.ingredient, version).ok()?;
            encode_item_stack(&mut payload, &recipe.result, version)?;
            Some(EncodedRecipe {
                id,
                serializer_id: SERIALIZER_STONECUTTING,
                payload,
            })
        }
        DynamicRecipe::SmithingTransform(recipe) => {
            let id = Identifier::parse(recipe.recipe_id.as_str()).ok()?;
            let mut payload = Vec::new();
            encode_ingredient(&mut payload, &recipe.template, version).ok()?;
            encode_ingredient(&mut payload, &recipe.base, version).ok()?;
            encode_ingredient(&mut payload, &recipe.addition, version).ok()?;
            encode_item_stack(&mut payload, &recipe.result, version)?;
            Some(EncodedRecipe {
                id,
                serializer_id: SERIALIZER_SMITHING_TRANSFORM,
                payload,
            })
        }
        DynamicRecipe::SmithingTrim(recipe) => {
            let id = Identifier::parse(recipe.recipe_id.as_str()).ok()?;
            let mut payload = Vec::new();
            encode_ingredient(&mut payload, &recipe.template, version).ok()?;
            encode_ingredient(&mut payload, &recipe.base, version).ok()?;
            encode_ingredient(&mut payload, &recipe.addition, version).ok()?;
            Some(EncodedRecipe {
                id,
                serializer_id: SERIALIZER_SMITHING_TRIM,
                payload,
            })
        }
        DynamicRecipe::Special(recipe) => {
            let id = Identifier::parse(recipe.recipe_id.as_str()).ok()?;
            let mut payload = Vec::new();
            payload
                .write_var_int(&VarInt(crafting_category_id(&recipe.category)))
                .ok()?;
            Some(EncodedRecipe {
                id,
                serializer_id: special_serializer_id(recipe.kind),
                payload,
            })
        }
    }
}

/// Mirrors `Ingredient.CONTENTS_STREAM_CODEC`: a varint-length list of item
/// stacks, with tag ingredients resolved to their items server-side.
fn encode_ingredient(
    payload: &mut Vec<u8>,
    ingredient: &OwnedRecipeIngredient,
    version: JavaMinecraftVersion,
) -> Result<(), WritingError> {
    let mut stacks = Vec::new();
    match ingredient {
        OwnedRecipeIngredient::Simple(id) => {
            if let Some(item) = item_from_key(id) {
                stacks.push(item);
            }
        }
        OwnedRecipeIngredient::Tagged(tag) => {
            let tag = tag.strip_prefix('#').unwrap_or(tag);
            let tag = tag.strip_prefix("minecraft:").unwrap_or(tag);
            for key in pumpkin_data::tag::get_tag_values(pumpkin_data::tag::RegistryKey::Item, tag)
                .into_iter()
                .flatten()
            {
                if let Some(item) = Item::from_registry_key(key) {
                    stacks.push(item);
                }
            }
        }
        OwnedRecipeIngredient::OneOf(ids) => {
            for id in ids {
                if let Some(item) = item_from_key(id) {
                    stacks.push(item);
                }
            }
        }
    }

    payload.write_var_int(&VarInt(stacks.len() as i32))?;
    for item in stacks {
        ItemStackSerializer::from(ItemStack::new(1, item)).write_with_version(payload, &version)?;
    }
    Ok(())
}

fn item_from_key(id: &str) -> Option<&'static Item> {
    Item::from_registry_key(id.strip_prefix("minecraft:").unwrap_or(id))
}

fn encode_item_stack(
    payload: &mut Vec<u8>,
    result: &OwnedRecipeResult,
    version: JavaMinecraftVersion,
) -> Option<()> {
    let item = item_from_key(&result.item_id)?;
    // The strict result codec rejects empty stacks, so drop recipes whose
    // result item is unknown to this version.
    ItemStackSerializer::from(ItemStack::new(result.count, item))
        .write_with_version(payload, &version)
        .ok()
}

/// Writes into an inner buffer while counting the recipes that were written,
/// so the packet's leading count can be patched in afterwards.
#[derive(Default)]
struct CountingWriter {
    buffer: Vec<u8>,
    count: i32,
}

impl std::io::Write for CountingWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.buffer.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod update_recipes_tests {
    use super::*;
    use crate::VarInt as ReadVarInt;
    use crate::ser::NetworkReadExt;
    use pumpkin_data::recipes::RecipeCategoryTypes;
    use pumpkin_util::version::JavaMinecraftVersion;

    #[test]
    fn encodes_a_shapeless_recipe_like_vanilla() {
        let recipe = DynamicRecipe::Crafting(OwnedCraftingRecipe::Shapeless {
            recipe_id: Some("minecraft:oak_planks_from_oak_log".to_string()),
            category: RecipeCategoryTypes::Building,
            group: None,
            ingredients: vec![OwnedRecipeIngredient::Simple(
                "minecraft:oak_log".to_string(),
            )],
            result: OwnedRecipeResult {
                item_id: "minecraft:oak_planks".to_string(),
                count: 4,
            },
        });

        let bytes = encode_update_recipes(&[recipe], &JavaMinecraftVersion::V_1_21);
        let mut cursor = std::io::Cursor::new(&bytes);

        // recipe count
        assert_eq!(cursor.get_var_int().unwrap(), ReadVarInt(1));
        // recipe id
        assert_eq!(
            cursor.get_str().unwrap().as_ref(),
            "minecraft:oak_planks_from_oak_log"
        );
        // serializer id: crafting_shapeless
        assert_eq!(cursor.get_var_int().unwrap(), ReadVarInt(1));
        // group, crafting category building
        assert_eq!(cursor.get_str().unwrap().as_ref(), "");
        assert_eq!(cursor.get_var_int().unwrap(), ReadVarInt(0));
        // one ingredient holding one stack: oak_log with count 1
        assert_eq!(cursor.get_var_int().unwrap(), ReadVarInt(1));
        assert_eq!(cursor.get_var_int().unwrap(), ReadVarInt(1));
        assert_eq!(cursor.get_var_int().unwrap(), ReadVarInt(1));
        assert_eq!(
            cursor.get_var_int().unwrap(),
            ReadVarInt(i32::from(Item::OAK_LOG.id))
        );
        // no component patch
        assert_eq!(cursor.get_var_int().unwrap(), ReadVarInt(0));
        assert_eq!(cursor.get_var_int().unwrap(), ReadVarInt(0));
        // result: 4 oak planks, then no components
        assert_eq!(cursor.get_var_int().unwrap(), ReadVarInt(4));
        assert_eq!(
            cursor.get_var_int().unwrap(),
            ReadVarInt(i32::from(Item::OAK_PLANKS.id))
        );
        assert_eq!(cursor.get_var_int().unwrap(), ReadVarInt(0));
        assert_eq!(cursor.get_var_int().unwrap(), ReadVarInt(0));
        assert_eq!(cursor.position() as usize, bytes.len());
    }

    #[test]
    fn skips_brewing_and_unknown_items() {
        let brewing = DynamicRecipe::Brewing(OwnedBrewingRecipe {
            recipe_id: "minecraft:brewing/potion".to_string(),
            input_item: "minecraft:potion".to_string(),
            input_potion: None,
            reagent: "minecraft:nether_wart".to_string(),
            output_item: "minecraft:potion".to_string(),
            output_potion: None,
        });
        let bad_result = DynamicRecipe::Crafting(OwnedCraftingRecipe::Shapeless {
            recipe_id: Some("minecraft:bad".to_string()),
            category: RecipeCategoryTypes::Misc,
            group: None,
            ingredients: Vec::new(),
            result: OwnedRecipeResult {
                item_id: "minecraft:not_an_item".to_string(),
                count: 1,
            },
        });

        let bytes = encode_update_recipes(&[brewing, bad_result], &JavaMinecraftVersion::V_1_21);
        let mut cursor = std::io::Cursor::new(&bytes);
        assert_eq!(cursor.get_var_int().unwrap(), crate::VarInt(0));
    }
}
