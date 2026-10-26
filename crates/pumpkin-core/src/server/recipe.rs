use std::sync::{Arc, RwLock};

use pumpkin_inventory::crafting::recipe_provider::RecipeProvider;
pub use pumpkin_protocol::codec::recipe::DynamicRecipe;
use pumpkin_protocol::codec::recipe::{
    OwnedCookingRecipe, OwnedCookingRecipeType, OwnedCraftingRecipe, OwnedRecipeIngredient,
    OwnedRecipeResult, OwnedSmithingTransformRecipe, OwnedSmithingTrimRecipe, OwnedSpecialRecipe,
    OwnedStonecuttingRecipe, SpecialRecipeType,
};

pub struct RecipeManager {
    dynamic_recipes: RwLock<Vec<DynamicRecipe>>,
    /// Cached `ClientboundUpdateRecipesPacket` payload, rebuilt whenever the
    /// recipe set changes.
    encoded_client_recipes: RwLock<Option<Arc<[u8]>>>,
}

impl Default for RecipeManager {
    fn default() -> Self {
        Self::new()
    }
}

impl RecipeManager {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            dynamic_recipes: RwLock::new(Vec::new()),
            encoded_client_recipes: RwLock::new(None),
        }
    }

    pub fn add_recipe(&self, recipe: DynamicRecipe) {
        let mut recipes = self
            .dynamic_recipes
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        recipes.push(recipe);
        self.invalidate_encoded();
    }

    pub fn add_recipes(&self, new_recipes: impl IntoIterator<Item = DynamicRecipe>) {
        let mut recipes = self
            .dynamic_recipes
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        recipes.extend(new_recipes);
        self.invalidate_encoded();
    }

    pub fn set_recipes(&self, new_recipes: Vec<DynamicRecipe>) {
        let mut recipes = self
            .dynamic_recipes
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *recipes = new_recipes;
        self.invalidate_encoded();
    }

    pub fn clear(&self) {
        let mut recipes = self
            .dynamic_recipes
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        recipes.clear();
        self.invalidate_encoded();
    }

    pub fn get_dynamic_recipes_internal(&self) -> Vec<DynamicRecipe> {
        self.dynamic_recipes
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    fn invalidate_encoded(&self) {
        *self
            .encoded_client_recipes
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = None;
    }

    /// The `ClientboundUpdateRecipesPacket` payload for the current recipe
    /// set, built once and shared by every joining player.
    #[must_use]
    pub fn encoded_client_recipes(&self) -> Arc<[u8]> {
        {
            let cached = self
                .encoded_client_recipes
                .read()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Some(payload) = cached.as_ref() {
                return payload.clone();
            }
        }

        let payload: Arc<[u8]> = Arc::from(pumpkin_protocol::codec::recipe::encode_update_recipes(
            &self.get_dynamic_recipes_internal(),
            &pumpkin_data::packet::CURRENT_MC_VERSION,
        ));
        *self
            .encoded_client_recipes
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(payload.clone());
        payload
    }
}

impl RecipeProvider for RecipeManager {
    fn get_dynamic_recipes(&self) -> Vec<DynamicRecipe> {
        self.dynamic_recipes
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }
}

/// The vanilla recipe set as [`DynamicRecipe`]s, seeded from the
/// `pumpkin-data` statics so datapack packs extend it instead of starting
/// from an empty book.
#[must_use]
pub fn vanilla_recipes() -> Vec<DynamicRecipe> {
    use pumpkin_data::recipes::{
        RECIPES_COOKING, RECIPES_CRAFTING, RECIPES_CRAFTING_IDS, RECIPES_SMITHING_TRANSFORM,
        RECIPES_SMITHING_TRANSFORM_IDS, RECIPES_SMITHING_TRIM, RECIPES_SMITHING_TRIM_IDS,
        RECIPES_SPECIAL, RECIPES_STONECUTTING, RECIPES_STONECUTTING_IDS,
    };

    let mut recipes = vanilla_crafting_recipes();
    recipes.extend(vanilla_cooking_recipes());

    for (recipe, id) in RECIPES_STONECUTTING.iter().zip(RECIPES_STONECUTTING_IDS) {
        recipes.push(DynamicRecipe::Stonecutting(OwnedStonecuttingRecipe {
            recipe_id: (*id).to_string(),
            group: recipe.group.map(ToString::to_string),
            ingredient: owned_ingredient(&recipe.ingredient),
            result: owned_result(recipe.result.id, recipe.result.count),
        }));
    }

    for (recipe, id) in RECIPES_SMITHING_TRANSFORM
        .iter()
        .zip(RECIPES_SMITHING_TRANSFORM_IDS)
    {
        recipes.push(DynamicRecipe::SmithingTransform(
            OwnedSmithingTransformRecipe {
                recipe_id: (*id).to_string(),
                template: owned_ingredient(&recipe.template),
                base: owned_ingredient(&recipe.base),
                addition: owned_ingredient(&recipe.addition),
                result: owned_result(recipe.result.id, recipe.result.count),
            },
        ));
    }

    for (recipe, id) in RECIPES_SMITHING_TRIM.iter().zip(RECIPES_SMITHING_TRIM_IDS) {
        recipes.push(DynamicRecipe::SmithingTrim(OwnedSmithingTrimRecipe {
            recipe_id: (*id).to_string(),
            template: owned_ingredient(&recipe.template),
            base: owned_ingredient(&recipe.base),
            addition: owned_ingredient(&recipe.addition),
        }));
    }

    for (id, category) in RECIPES_SPECIAL {
        let Some(kind) = special_kind_from_id(id) else {
            continue;
        };
        recipes.push(DynamicRecipe::Special(OwnedSpecialRecipe {
            recipe_id: (*id).to_string(),
            kind,
            category: category.clone(),
        }));
    }

    recipes
}

fn owned_ingredient(value: &pumpkin_data::recipes::RecipeIngredientTypes) -> OwnedRecipeIngredient {
    use pumpkin_data::recipes::RecipeIngredientTypes;
    match value {
        RecipeIngredientTypes::Simple(id) => OwnedRecipeIngredient::Simple((*id).to_string()),
        RecipeIngredientTypes::Tagged(tag) => OwnedRecipeIngredient::Tagged((*tag).to_string()),
        RecipeIngredientTypes::OneOf(ids) => {
            OwnedRecipeIngredient::OneOf(ids.iter().map(|id| (*id).to_string()).collect::<Vec<_>>())
        }
    }
}

fn owned_result(id: &str, count: u8) -> OwnedRecipeResult {
    OwnedRecipeResult {
        item_id: id.to_string(),
        count,
    }
}

fn vanilla_crafting_recipes() -> Vec<DynamicRecipe> {
    use pumpkin_data::recipes::{CraftingRecipeTypes, RECIPES_CRAFTING, RECIPES_CRAFTING_IDS};

    let mut recipes = Vec::new();
    for (recipe, id) in RECIPES_CRAFTING.iter().zip(RECIPES_CRAFTING_IDS) {
        match recipe {
            CraftingRecipeTypes::CraftingShaped {
                category,
                group,
                show_notification,
                key,
                pattern,
                result: recipe_result,
            } => recipes.push(DynamicRecipe::Crafting(OwnedCraftingRecipe::Shaped {
                recipe_id: Some((*id).to_string()),
                category: category.clone(),
                group: group.map(ToString::to_string),
                show_notification: *show_notification,
                key: key
                    .iter()
                    .map(|(ch, value)| (*ch, owned_ingredient(value)))
                    .collect::<Vec<_>>(),
                pattern: pattern
                    .iter()
                    .map(|row| (*row).to_string())
                    .collect::<Vec<_>>(),
                result: owned_result(recipe_result.id, recipe_result.count),
            })),
            CraftingRecipeTypes::CraftingShapeless {
                category,
                group,
                ingredients,
                result: recipe_result,
            } => recipes.push(DynamicRecipe::Crafting(OwnedCraftingRecipe::Shapeless {
                recipe_id: Some((*id).to_string()),
                category: category.clone(),
                group: group.map(ToString::to_string),
                ingredients: ingredients.iter().map(owned_ingredient).collect::<Vec<_>>(),
                result: owned_result(recipe_result.id, recipe_result.count),
            })),
            CraftingRecipeTypes::CraftingDecoratedPot { category } => {
                recipes.push(DynamicRecipe::Special(OwnedSpecialRecipe {
                    recipe_id: (*id).to_string(),
                    kind: SpecialRecipeType::DecoratedPot,
                    category: category.clone(),
                }));
            }
            // No 1.21.1 serializer exists for these; the vanilla 1.21.1 data
            // pack contains none.
            CraftingRecipeTypes::CraftingTransmute { .. }
            | CraftingRecipeTypes::CraftingSpecial => {}
        }
    }
    recipes
}

fn vanilla_cooking_recipes() -> Vec<DynamicRecipe> {
    use pumpkin_data::recipes::{CookingRecipe, CookingRecipeType, RECIPES_COOKING};

    let owned = |recipe: &CookingRecipe| OwnedCookingRecipe {
        recipe_id: recipe.recipe_id.to_string(),
        category: recipe.category.clone(),
        group: recipe.group.map(ToString::to_string),
        ingredient: owned_ingredient(&recipe.ingredient),
        cooking_time: recipe.cookingtime,
        experience: recipe.experience,
        result: owned_result(recipe.result.id, recipe.result.count),
    };

    RECIPES_COOKING
        .iter()
        .map(|cooking| {
            DynamicRecipe::Cooking(match cooking {
                CookingRecipeType::Blasting(recipe) => {
                    OwnedCookingRecipeType::Blasting(owned(recipe))
                }
                CookingRecipeType::Smelting(recipe) => {
                    OwnedCookingRecipeType::Smelting(owned(recipe))
                }
                CookingRecipeType::Smoking(recipe) => {
                    OwnedCookingRecipeType::Smoking(owned(recipe))
                }
                CookingRecipeType::CampfireCooking(recipe) => {
                    OwnedCookingRecipeType::CampfireCooking(owned(recipe))
                }
            })
        })
        .collect()
}

fn special_kind_from_id(id: &str) -> Option<SpecialRecipeType> {
    Some(match id.strip_prefix("minecraft:")? {
        "armor_dye" => SpecialRecipeType::ArmorDye,
        "banner_duplicate" => SpecialRecipeType::BannerDuplicate,
        "book_cloning" => SpecialRecipeType::BookCloning,
        "map_cloning" => SpecialRecipeType::MapCloning,
        "map_extending" => SpecialRecipeType::MapExtending,
        "firework_rocket" => SpecialRecipeType::FireworkRocket,
        "firework_star" => SpecialRecipeType::FireworkStar,
        "firework_star_fade" => SpecialRecipeType::FireworkStarFade,
        "tipped_arrow" => SpecialRecipeType::TippedArrow,
        "shield_decoration" => SpecialRecipeType::ShieldDecoration,
        "shulker_box_coloring" => SpecialRecipeType::ShulkerBoxColoring,
        "suspicious_stew" => SpecialRecipeType::SuspiciousStew,
        "repair_item" => SpecialRecipeType::RepairItem,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use pumpkin_protocol::ser::NetworkReadExt;

    #[test]
    fn vanilla_recipes_seed_every_recipe_kind() {
        let recipes = vanilla_recipes();
        assert!(recipes.len() > 1200, "got {} recipes", recipes.len());
        assert!(recipes.iter().any(|r| matches!(
            r,
            DynamicRecipe::Crafting(
                pumpkin_protocol::codec::recipe::OwnedCraftingRecipe::Shaped { .. }
            )
        )));
        assert!(
            recipes
                .iter()
                .any(|r| matches!(r, DynamicRecipe::Special(_)))
        );

        let encoded = pumpkin_protocol::codec::recipe::encode_update_recipes(
            &recipes,
            &pumpkin_data::packet::CURRENT_MC_VERSION,
        );
        let mut cursor: &[u8] = &encoded;
        let count = cursor.get_var_int().unwrap().0;
        assert!(count > 1200, "encoded {count} recipes");
    }
}
