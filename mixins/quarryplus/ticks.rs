// The quarry ticks on the server (`native_block_entity_ticks`, API 15).
fn observe_block_entity_ticks(block: &str, ticks: &mut bool) {
    crate::game::quarryplus_state::on_block_entity_ticks(block, ticks);
}
