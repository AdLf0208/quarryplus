// One server tick of a quarry: advance the dig cursor and break a block.
fn observe_block_entity_tick(
    game: &mut crate::game::Game<'_>,
    pos: [i32; 3],
    entity: &mut horizon_client::containers::ModBlockEntity,
) {
    crate::game::quarryplus_state::on_block_entity_tick(game, pos, entity);
}
