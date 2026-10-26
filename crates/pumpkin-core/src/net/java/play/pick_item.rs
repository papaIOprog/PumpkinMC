#[allow(clippy::wildcard_imports)]
use super::*;

impl JavaClient {
    /// Mirrors `Inventory#pickSlot` and `ServerGamePacketListenerImpl#handlePickItem`:
    /// swap the picked slot with the best hotbar slot, then sync both slots and the
    /// held slot to the client.
    pub fn handle_pick_item(&self, player: &Arc<Player>, pick_item: &SPickItem) {
        let slot = pick_item.slot_to_use.0;
        if !(0..36).contains(&slot) {
            return;
        }
        let slot = slot as usize;

        let hotbar_slot = suitable_hotbar_slot(player);
        let picked = player.inventory.get_slot(slot);
        let hotbar_stack = player.inventory.get_slot(hotbar_slot);

        player.inventory.set_selected_slot(hotbar_slot as u8);
        player.inventory.set_slot(hotbar_slot, picked.clone());
        player.inventory.set_slot(slot, hotbar_stack);

        // Vanilla sends these player inventory slot updates with container id -2.
        let picked_serializer = ItemStackSerializer::from(picked);
        let packet = CSetContainerSlot::new(-2, 0, hotbar_slot as i16, &picked_serializer);
        player.try_send_client_packet(&packet);
        let swapped_serializer = ItemStackSerializer::from(player.inventory.get_slot(slot));
        let packet = CSetContainerSlot::new(-2, 0, slot as i16, &swapped_serializer);
        player.try_send_client_packet(&packet);
        player.try_send_client_packet(&CSetSelectedSlot::new(hotbar_slot as i8));
    }
}

/// Mirrors `Inventory#getSuitableHotbarSlot`: the first empty hotbar slot from
/// the selected one, then the first without enchantments, then the selected one.
fn suitable_hotbar_slot(player: &Player) -> usize {
    let selected = player.inventory.get_selected_slot() as usize;
    for offset in 0..9 {
        let slot = (selected + offset) % 9;
        if player.inventory.get_slot(slot).is_empty() {
            return slot;
        }
    }
    for offset in 0..9 {
        let slot = (selected + offset) % 9;
        if !player.inventory.get_slot(slot).has_enchantments() {
            return slot;
        }
    }
    selected
}
