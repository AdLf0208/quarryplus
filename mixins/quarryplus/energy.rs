// The quarry's 10,000 FE buffer (`native_block_entity_energy`, API 142).
fn observe_block_entity_energy(
    block: &str,
    energy: &mut Option<horizon_client::containers::ModEnergyBuffer>,
) {
    crate::game::quarryplus_state::on_block_entity_energy(block, energy);
}
