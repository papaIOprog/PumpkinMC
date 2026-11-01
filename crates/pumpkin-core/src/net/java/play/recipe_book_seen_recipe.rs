#[allow(clippy::wildcard_imports)]
use super::*;

impl JavaClient {
    pub fn handle_recipe_book_seen_recipe(
        &self,
        server: &Arc<Server>,
        player: &Arc<Player>,
        packet: &SRecipeBookSeenRecipe,
    ) {
        let mut event = crate::plugin::api::events::player::player_recipe_discover::PlayerRecipeDiscoverEvent::new(
            player.clone(),
            packet.recipe_id.clone(),
        );
        server.plugin_manager.fire_blocking(server, &mut event);
        player.remove_recipe_highlight(&packet.recipe_id);
    }
}
