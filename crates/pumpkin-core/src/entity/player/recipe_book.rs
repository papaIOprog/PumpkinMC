use pumpkin_nbt::compound::NbtCompound;
use pumpkin_nbt::tag::NbtTag;
use pumpkin_protocol::codec::recipe::DynamicRecipe;
use pumpkin_protocol::java::client::play::RecipeBookSettings;
use rustc_hash::FxHashSet;

/// The per-player recipe book, vanilla `ServerRecipeBook`.
///
/// Recipes are stored by their string key (`minecraft:oak_planks`); keys the
/// `RecipeManager` does not know are dropped on load, like vanilla.
#[derive(Default)]
pub struct RecipeBook {
    pub known: FxHashSet<String>,
    /// Recipes the client flashes as newly unlocked (`toBeDisplayed`).
    pub highlight: FxHashSet<String>,
    pub settings: RecipeBookSettings,
}

impl RecipeBook {
    /// Vanilla `ServerRecipeBook#addRecipes` without the packet send: returns
    /// the keys that were newly learned. Special recipes never unlock.
    pub fn add_recipes<'a>(
        &mut self,
        recipes: impl Iterator<Item = (&'a str, bool)>,
    ) -> Vec<&'a str> {
        let mut added = Vec::new();
        for (id, is_special) in recipes {
            if is_special || self.known.contains(id) {
                continue;
            }
            self.known.insert(id.to_string());
            self.highlight.insert(id.to_string());
            added.push(id);
        }
        added
    }

    /// Vanilla `ServerRecipeBook#removeRecipes` without the packet send:
    /// returns the keys that were actually known.
    pub fn remove_recipes<'a>(&mut self, ids: impl Iterator<Item = &'a str>) -> Vec<&'a str> {
        let mut removed = Vec::new();
        for id in ids {
            if self.known.remove(id) {
                self.highlight.remove(id);
                removed.push(id);
            }
        }
        removed
    }

    pub fn write_nbt(&self, nbt: &mut NbtCompound) {
        let mut book = NbtCompound::new();
        let settings = &self.settings;
        // Vanilla `RecipeBookSettings.TAG_FIELDS`.
        for (open_key, filtering_key, open, filtering) in [
            (
                "isGuiOpen",
                "isFilteringCraftable",
                settings.crafting_open,
                settings.crafting_filtering,
            ),
            (
                "isFurnaceGuiOpen",
                "isFurnaceFilteringCraftable",
                settings.furnace_open,
                settings.furnace_filtering,
            ),
            (
                "isBlastingFurnaceGuiOpen",
                "isBlastingFurnaceFilteringCraftable",
                settings.blast_furnace_open,
                settings.blast_furnace_filtering,
            ),
            (
                "isSmokerGuiOpen",
                "isSmokerFilteringCraftable",
                settings.smoker_open,
                settings.smoker_filtering,
            ),
        ] {
            book.put_bool(open_key, open);
            book.put_bool(filtering_key, filtering);
        }
        book.put(
            "recipes",
            NbtTag::List(
                self.known
                    .iter()
                    .map(|id| NbtTag::String(id.as_str().into()))
                    .collect(),
            ),
        );
        book.put(
            "toBeDisplayed",
            NbtTag::List(
                self.highlight
                    .iter()
                    .map(|id| NbtTag::String(id.as_str().into()))
                    .collect(),
            ),
        );
        nbt.put_compound("recipeBook", book);
    }

    pub fn read_nbt(&mut self, nbt: &NbtCompound, recipes: &[DynamicRecipe]) {
        let Some(book) = nbt.get_compound("recipeBook") else {
            return;
        };
        self.settings = RecipeBookSettings {
            crafting_open: book.get_bool("isGuiOpen").unwrap_or(false),
            crafting_filtering: book.get_bool("isFilteringCraftable").unwrap_or(false),
            furnace_open: book.get_bool("isFurnaceGuiOpen").unwrap_or(false),
            furnace_filtering: book
                .get_bool("isFurnaceFilteringCraftable")
                .unwrap_or(false),
            blast_furnace_open: book.get_bool("isBlastingFurnaceGuiOpen").unwrap_or(false),
            blast_furnace_filtering: book
                .get_bool("isBlastingFurnaceFilteringCraftable")
                .unwrap_or(false),
            smoker_open: book.get_bool("isSmokerGuiOpen").unwrap_or(false),
            smoker_filtering: book.get_bool("isSmokerFilteringCraftable").unwrap_or(false),
        };
        for (target, key) in [
            (&mut self.known, "recipes"),
            (&mut self.highlight, "toBeDisplayed"),
        ] {
            let Some(list) = book.get_list(key) else {
                continue;
            };
            for tag in list {
                if let NbtTag::String(id) = tag
                    && recipes
                        .iter()
                        .any(|recipe| recipe.recipe_id() == Some(&*id))
                {
                    target.insert(id.to_string());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pumpkin_data::recipes::RecipeCategoryTypes;
    use pumpkin_protocol::codec::recipe::{
        OwnedCraftingRecipe, OwnedRecipeResult, OwnedSpecialRecipe, SpecialRecipeType,
    };

    fn special(id: &str) -> DynamicRecipe {
        DynamicRecipe::Special(OwnedSpecialRecipe {
            recipe_id: id.to_string(),
            kind: SpecialRecipeType::RepairItem,
            category: RecipeCategoryTypes::Misc,
        })
    }

    fn shaped(id: &str) -> DynamicRecipe {
        DynamicRecipe::Crafting(OwnedCraftingRecipe::Shapeless {
            recipe_id: Some(id.to_string()),
            category: RecipeCategoryTypes::Misc,
            group: None,
            ingredients: Vec::new(),
            result: OwnedRecipeResult {
                item_id: "minecraft:stick".to_string(),
                count: 4,
            },
        })
    }

    #[test]
    fn add_skips_known_and_special_recipes() {
        let mut book = RecipeBook::default();
        let added = book.add_recipes(
            [
                ("minecraft:stick", false),
                ("minecraft:stick", false),
                ("minecraft:repair_item", true),
            ]
            .into_iter(),
        );
        assert_eq!(added, ["minecraft:stick"]);
        assert!(book.highlight.contains("minecraft:stick"));
    }

    #[test]
    fn remove_drops_the_highlight_too() {
        let mut book = RecipeBook::default();
        book.add_recipes(std::iter::once(("minecraft:stick", false)));
        let removed = book.remove_recipes(["minecraft:stick", "minecraft:unknown"].into_iter());
        assert_eq!(removed, ["minecraft:stick"]);
        assert!(!book.highlight.contains("minecraft:stick"));
        assert!(book.known.is_empty());
    }

    #[test]
    fn nbt_round_trip_keeps_only_known_recipes() {
        let recipes = vec![shaped("minecraft:stick"), special("minecraft:repair_item")];
        let mut book = RecipeBook::default();
        book.add_recipes(std::iter::once(("minecraft:stick", false)));
        book.settings.crafting_open = true;
        book.settings.smoker_filtering = true;

        let mut nbt = NbtCompound::new();
        book.write_nbt(&mut nbt);

        let mut loaded = RecipeBook::default();
        loaded.read_nbt(&nbt, &recipes);
        assert_eq!(loaded.known, book.known);
        assert_eq!(loaded.highlight, book.highlight);
        assert!(loaded.settings.crafting_open);
        assert!(loaded.settings.smoker_filtering);
        assert!(!loaded.settings.furnace_open);

        // A stale key from a removed datapack recipe is dropped on load.
        let mut stale = NbtCompound::new();
        stale.put(
            "recipes",
            NbtTag::List(vec![NbtTag::String("minecraft:gone".into())]),
        );
        let mut root = NbtCompound::new();
        root.put_compound("recipeBook", stale);
        let mut filtered = RecipeBook::default();
        filtered.read_nbt(&root, &recipes);
        assert!(filtered.known.is_empty());
    }
}
