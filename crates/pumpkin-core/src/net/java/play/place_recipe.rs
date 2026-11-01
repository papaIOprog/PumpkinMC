#[allow(clippy::wildcard_imports)]
use super::*;

use crate::net::java::recipe_helper::{compute_biggest_craftable, take_n_ingredient};
use crate::server::recipe::DynamicRecipe;
use pumpkin_data::screen::WindowType;
use pumpkin_inventory::screen_handler::ScreenHandler;
use pumpkin_protocol::codec::recipe::{
    OwnedCookingRecipeType, OwnedCraftingRecipe, OwnedRecipeIngredient,
};
use std::sync::{Arc, Mutex};

impl JavaClient {
    pub fn handle_place_recipe(
        &self,
        server: &Arc<Server>,
        player: &Arc<Player>,
        packet: &SPlaceRecipe,
    ) {
        let use_max = packet.use_max_items;

        let mut click_event = crate::plugin::api::events::player::player_recipe_book_click::PlayerRecipeBookClickEvent::new(
            player.clone(),
            packet.recipe_id.clone(),
            use_max,
        );
        server
            .plugin_manager
            .fire_blocking(server, &mut click_event);
        if click_event.cancelled {
            return;
        }

        // Vanilla `handlePlaceRecipe`: the packet's container must be the open
        // menu, and `ServerPlaceRecipe#recipeClicked` only places recipes the
        // player's book knows.
        player.update_last_action_time();
        if player.gamemode.load() == pumpkin_util::GameMode::Spectator {
            return;
        }
        let screen_handler_arc = player
            .current_screen_handler
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        let (sync_id, window_type) = {
            let handler = screen_handler_arc
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            (handler.get_behaviour().sync_id, handler.window_type())
        };
        if i32::from(sync_id) != i32::from(packet.container_id) {
            return;
        }

        let known = player
            .recipe_book
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .known
            .contains(&packet.recipe_id);
        if !known {
            return;
        }
        let Some(recipe) = server.recipe_manager.by_key(&packet.recipe_id) else {
            return;
        };

        match window_type {
            // `None` is the player's own 2x2 crafting grid.
            Some(WindowType::Crafting) | None => {
                Self::place_crafting_recipe(player, &recipe, use_max, &screen_handler_arc);
            }
            Some(WindowType::Furnace | WindowType::BlastFurnace | WindowType::Smoker) => {
                Self::place_cooking_recipe(player, &recipe, use_max, &screen_handler_arc);
            }
            _ => {}
        }
    }

    /// The crafting table and the player's 2x2 grid share vanilla's
    /// `ServerPlaceRecipe` flow: return the grid to the inventory, then fill
    /// it from the inventory again.
    #[allow(clippy::too_many_lines)]
    fn place_crafting_recipe(
        player: &Arc<Player>,
        recipe: &DynamicRecipe,
        use_max: bool,
        screen_handler_arc: &Arc<Mutex<dyn ScreenHandler>>,
    ) {
        let DynamicRecipe::Crafting(crafting) = recipe else {
            return;
        };

        let (grid_width, crafting_inv) = {
            let handler = screen_handler_arc
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let grid_width: usize = match handler.window_type() {
                Some(WindowType::Crafting) => 3,
                None => 2, // player inventory 2x2
                _ => return,
            };
            (grid_width, handler.get_behaviour().slots[1].get_inventory())
        };

        let grid_size = grid_width * grid_width;
        let mut ingredient_slots: Vec<Option<&OwnedRecipeIngredient>> = vec![None; grid_size];

        match crafting {
            OwnedCraftingRecipe::Shaped { pattern, key, .. } => {
                for (row, row_str) in pattern.iter().enumerate() {
                    for (col, ch) in row_str.chars().enumerate() {
                        if ch != ' '
                            && let Some((_, ing)) = key.iter().find(|(k, _)| *k == ch)
                            && row * grid_width + col < grid_size
                        {
                            ingredient_slots[row * grid_width + col] = Some(ing);
                        }
                    }
                }
            }
            OwnedCraftingRecipe::Shapeless { ingredients, .. } => {
                for (i, ing) in ingredients.iter().enumerate().take(grid_size) {
                    ingredient_slots[i] = Some(ing);
                }
            }
        }

        // Check if this exact recipe is already placed (determines stacking vs fresh fill).
        let recipe_matches = {
            let mut ok = true;
            for (idx, ing) in ingredient_slots.iter().enumerate() {
                let stack = crafting_inv.get_stack(idx);
                match ing {
                    None => {
                        if !stack.is_empty() {
                            ok = false;
                            break;
                        }
                    }
                    Some(ingredient) => {
                        if stack.is_empty() || !ingredient.match_item(stack.item) {
                            ok = false;
                            break;
                        }
                    }
                }
            }
            ok
        };

        // Read minimum count from occupied slots before clearing (needed for stacking).
        let current_min = if recipe_matches && !use_max {
            let mut min = u8::MAX;
            for (idx, ing) in ingredient_slots.iter().enumerate() {
                if ing.is_some() {
                    let stack = crafting_inv.get_stack(idx);
                    if !stack.is_empty() {
                        min = min.min(stack.item_count);
                    }
                }
            }
            if min == u8::MAX { 0 } else { min }
        } else {
            0
        };

        // Always clear the grid first, returning items to inventory.
        for i in 0..grid_size {
            let stack = crafting_inv.remove_stack(i);
            if !stack.is_empty() {
                player.inventory.offer(stack, false, player.as_ref());
            }
        }

        // Determine how many of each ingredient to place per slot.
        let active_ingredients: Vec<&OwnedRecipeIngredient> =
            ingredient_slots.iter().flatten().copied().collect();
        let amount_to_craft = if use_max {
            compute_biggest_craftable(&active_ingredients, &player.inventory)
        } else if recipe_matches {
            current_min.saturating_add(1)
        } else {
            1
        };

        if amount_to_craft == 0 {
            screen_handler_arc
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .send_content_updates();
            return;
        }

        // Fill each grid slot with exactly `amount_to_craft` matching items.
        for (idx, ing) in ingredient_slots.iter().enumerate() {
            let Some(ingredient) = ing else { continue };
            let taken = take_n_ingredient(&player.inventory, ingredient, amount_to_craft);
            if !taken.is_empty() {
                crafting_inv.set_stack(idx, taken);
            }
        }

        screen_handler_arc
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .send_content_updates();
    }

    /// Furnace-family menus have a 1x1 grid: vanilla places the cooking
    /// recipe's ingredient into the input slot.
    fn place_cooking_recipe(
        player: &Arc<Player>,
        recipe: &DynamicRecipe,
        use_max: bool,
        screen_handler_arc: &Arc<Mutex<dyn ScreenHandler>>,
    ) {
        use pumpkin_inventory::furnace_like::furnace_like_screen_handler::FurnaceLikeScreenHandler;

        let ingredient = match recipe {
            DynamicRecipe::Cooking(
                OwnedCookingRecipeType::Blasting(recipe)
                | OwnedCookingRecipeType::Smelting(recipe)
                | OwnedCookingRecipeType::Smoking(recipe)
                | OwnedCookingRecipeType::CampfireCooking(recipe),
            ) => &recipe.ingredient,
            _ => return,
        };

        let furnace_inv = {
            let handler = screen_handler_arc
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            handler.get_behaviour().slots[FurnaceLikeScreenHandler::INGREDIENT_SLOT].get_inventory()
        };

        // Return the current input to the inventory before refilling.
        let stack = furnace_inv.remove_stack(FurnaceLikeScreenHandler::INGREDIENT_SLOT);
        if !stack.is_empty() {
            player.inventory.offer(stack, false, player.as_ref());
        }

        let amount = if use_max {
            compute_biggest_craftable(&[ingredient], &player.inventory)
        } else {
            1
        };
        if amount == 0 {
            screen_handler_arc
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .send_content_updates();
            return;
        }
        let taken = take_n_ingredient(&player.inventory, ingredient, amount);
        if !taken.is_empty() {
            furnace_inv.set_stack(FurnaceLikeScreenHandler::INGREDIENT_SLOT, taken);
        }

        screen_handler_arc
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .send_content_updates();
    }
}
