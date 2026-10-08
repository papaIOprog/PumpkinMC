#[allow(clippy::wildcard_imports)]
use super::*;

impl JavaClient {
    pub fn handle_player_command(
        &self,
        player: &Arc<Player>,
        command: &SPlayerCommand,
        server: &Arc<Server>,
    ) {
        if command.entity_id != player.entity_id().into() {
            return;
        }
        if !player.has_client_loaded() {
            return;
        }
        player.update_last_action_time();

        let entity = &player.get_entity();
        match command.action {
            Action::StartSprinting => {
                if !entity.is_sprinting() {
                    send_cancellable_blocking! {{
                        server;
                        PlayerToggleSprintEvent::new(player.clone(), true);
                        'after: {
                            player.set_sprinting(event.is_sprinting);
                            player.update_player_pose();
                        }
                    }}
                }
            }
            Action::StopSprinting => {
                if entity.is_sprinting() {
                    send_cancellable_blocking! {{
                        server;
                        PlayerToggleSprintEvent::new(player.clone(), false);
                        'after: {
                            player.set_sprinting(event.is_sprinting);
                            player.update_player_pose();
                        }
                    }}
                }
            }
            Action::LeaveBed => player.wake_up(),

            Action::StartHorseJump | Action::StopHorseJump | Action::OpenVehicleInventory => {
                debug!("todo");
            }
            Action::StartFlyingElytra => {
                let fall_flying = entity.check_fall_flying();
                if entity.is_fall_flying() != fall_flying {
                    let mut event = crate::plugin::api::events::entity::entity_toggle_glide::EntityToggleGlideEvent::new(
                        entity.entity_id,
                        fall_flying,
                    );
                    server.plugin_manager.fire_blocking(server, &mut event);
                    if !event.cancelled {
                        entity.set_fall_flying(event.is_gliding);
                    }
                }
            }
            // 1.21.1 sends sneak state through the player command packet
            Action::StartSneaking | Action::StopSneaking => {
                let sneak = matches!(command.action, Action::StartSneaking);
                if sneak
                    && player.gamemode.load() == GameMode::Spectator
                    && player.camera_target_id.load().is_some()
                {
                    player.camera_target_id.store(None);
                    player.try_send_client_packet(&CSetCamera::new(player.entity_id().into()));
                }

                if player.get_entity().is_sneaking() != sneak {
                    send_cancellable_blocking! {{
                        server;
                        PlayerToggleSneakEvent::new(player.clone(), sneak);
                        'after: {
                            player.get_entity().set_sneaking(event.is_sneaking);
                            if event.is_sneaking {
                                let vehicle = player
                                    .get_entity()
                                    .vehicle
                                    .lock()
                                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                                    .clone();
                                if let Some(vehicle) = vehicle {
                                    vehicle.get_entity().remove_passenger(player.entity_id());
                                }
                            }
                        }
                    }}
                } else if sneak {
                    let vehicle = player
                        .get_entity()
                        .vehicle
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .clone();
                    if let Some(vehicle) = vehicle {
                        vehicle.get_entity().remove_passenger(player.entity_id());
                    }
                }
            }
        }
    }
}
