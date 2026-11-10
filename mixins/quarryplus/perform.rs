// Run the `/quarryplus` commands a player types in chat.
fn observe_perform(
    game: &mut crate::game::Game<'_>,
    player: &NativeServerPlayer,
    command: &str,
    ran: &mut bool,
) {
    crate::game::quarryplus_state::on_perform_command(game, player, command, ran);
}
