// Declare `/quarryplus info|energy|area`.
fn observe_register(commands: &mut Vec<NativeCommand>) {
    crate::game::quarryplus_state::on_register_commands(commands);
}
