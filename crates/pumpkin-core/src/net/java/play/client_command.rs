#[allow(clippy::wildcard_imports)]
use super::*;

impl JavaClient {
    pub fn handle_client_status(&self, player: &Arc<Player>, client_status: &SClientCommand) {
        player.update_last_action_time();
        match client_status.action_id.0 {
            SClientCommand::PERFORM_RESPAWN => {
                // Perform respawn
                if player.living_entity.health.load() > 0.0 {
                    return;
                }
                let player_c = player.clone();
                let Some(server) = player.world().server.upgrade() else {
                    return;
                };
                server.spawn_task(async move {
                    player_c
                        .world()
                        .clone()
                        .respawn_player(&player_c, false)
                        .await;

                    {
                        let screen_handler = player_c
                            .current_screen_handler
                            .lock()
                            .unwrap_or_else(std::sync::PoisonError::into_inner);
                        let mut screen_handler = screen_handler
                            .lock()
                            .unwrap_or_else(std::sync::PoisonError::into_inner);
                        screen_handler.sync_state();
                    };

                    // Restore abilities based on gamemode after respawn
                    {
                        let mut abilities = player_c
                            .abilities
                            .lock()
                            .unwrap_or_else(std::sync::PoisonError::into_inner);
                        abilities.set_for_gamemode(player_c.gamemode.load());
                    };
                    player_c.send_abilities_update();
                });
            }
            SClientCommand::REQUEST_STATS => {
                // Request stats
                player.send_stats();
            }
            _ => {
                self.try_kick(&TextComponent::text("Invalid client status"));
            }
        }
    }
}
