// The quarry's nine-slot internal inventory (`native_block_entity_container`,
// API 16/17), shown as a vanilla 3 x 3 grid.
fn observe_block_entity_container(
    block: &str,
    container: &mut Option<horizon_client::containers::ModContainer>,
) {
    crate::game::quarryplus_state::on_block_entity_container(block, container);
}
