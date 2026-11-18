//! QuarryPlus for Horizon — the Quarry machine and its carried content.
//!
//! [`runtime`] is the port's pure logic (the dig area and its order, the FE cost of a
//! break, the mining tick and the drop plan) and carries its own tests; this file is
//! only the game glue.
//!
//! Upstream keeps a quarry's work as an `Area` (frame bounds), a `targetPos` iterator and
//! a `PowerEntity` energy buffer. A plain quarry mines one layer at a time from the top
//! down, one block per action, and each break costs `hardness * 25 FE` (no enchantments)
//! with a 10,000 FE buffer. The buffer is the game's default energy domain for a mod block
//! (`Registrar::block_energy`), saved in the block entity's `energy` int; the nine-slot
//! inventory is its default item domain (`Registrar::block_container`); the blocks come
//! from the `assets/` and `data/` this package carries.
mod runtime;

use mod_api::{
    self, BlockEntityPart, BlockEntityRef, BlockEntityTickEvent, BlockEntityTickReply,
    BlockEntityTicksEvent, BlockPart, Command, CommandArgumentValue, Compound, Flow, Hooks,
    HostBlockStates, Identifier, ItemKey, ItemStack, LevelRef, ModContainer, ModEnergyBuffer,
    RegisterCommandsEvent, Registrar, RegistrationContext, RunModCommandEvent, ServerHost, Tag,
    Text,
};

use runtime::{
    Area, DEFAULT_DEPTH, DEFAULT_RADIUS, MAX_ENERGY, MINE_TICKS, Mining, Stack as DropStack, Tick,
    break_cost, drop_plan,
};

/// The block and its item.
const QUARRY: &str = "quarryplus:quarry";
/// The block entity type this mod declares for the block, in its own namespace.
const QUARRY_TYPE: &str = "quarry";
/// The pickaxe drops are rolled with (upstream uses a netherite pickaxe).
const TOOL: &str = "minecraft:netherite_pickaxe";
/// The internal inventory, shown as a vanilla 3 x 3 grid.
const INVENTORY_SLOTS: usize = 9;
/// The vanilla menu that shows the nine slots when the block is used.
const MENU: &str = "minecraft:generic_3x3";
/// Fallback when the item registry does not answer (vanilla stacks to 64).
const FALLBACK_MAX_STACK: u32 = 64;

/// One integer of the saved compound, by its Java key. `Compound` is a map keyed by Java's
/// lossless UTF-16 strings (`JavaString`), which the SDK does not export by name, so the lookup
/// spells the key the way Java does and compares the stored units.
fn int(nbt: &Compound, name: &str) -> Option<i32> {
    let units: Vec<u16> = name.encode_utf16().collect();
    match nbt.iter().find(|(key, _)| key.0 == units) {
        Some((_, Tag::Int(value))) => Some(*value),
        _ => None,
    }
}

fn put(nbt: &mut Compound, name: &str, value: i32) {
    nbt.insert(name.into(), Tag::Int(value));
}

/// The saved shaft bounds, or `None` when the compound has none (a fresh block).
fn read_area(nbt: &Compound) -> Option<Area> {
    let min = [int(nbt, "minX")?, int(nbt, "minY")?, int(nbt, "minZ")?];
    let max = [int(nbt, "maxX")?, int(nbt, "maxY")?, int(nbt, "maxZ")?];
    Area::new(min, max).ok()
}

fn write_area(nbt: &mut Compound, area: &Area) {
    put(nbt, "minX", area.min[0]);
    put(nbt, "minY", area.min[1]);
    put(nbt, "minZ", area.min[2]);
    put(nbt, "maxX", area.max[0]);
    put(nbt, "maxY", area.max[1]);
    put(nbt, "maxZ", area.max[2]);
}

/// The saved mining state. The FE amount is the engine's `energy` int, the same field the
/// block's energy domain keeps its charge in.
fn read_mining(nbt: &Compound) -> Mining {
    Mining {
        energy: int(nbt, "energy").unwrap_or(0).clamp(0, MAX_ENERGY),
        progress: int(nbt, "progress").unwrap_or(0).clamp(0, MINE_TICKS - 1),
        cursor: int(nbt, "cursor").unwrap_or(0).max(0) as usize,
        mined: int(nbt, "mined").unwrap_or(0).max(0) as u32,
        skipped: int(nbt, "skipped").unwrap_or(0).max(0) as u32,
        finished: int(nbt, "finished").unwrap_or(0) != 0,
    }
}

fn write_mining(nbt: &mut Compound, mining: &Mining) {
    put(nbt, "energy", mining.energy);
    put(nbt, "progress", mining.progress);
    put(nbt, "cursor", mining.cursor as i32);
    put(nbt, "mined", mining.mined as i32);
    put(nbt, "skipped", mining.skipped as i32);
    put(nbt, "finished", i32::from(mining.finished));
}

/// The three vanilla blocks a quarry treats as nothing to mine (the engine's `isAir`,
/// which the SDK does not publish).
fn is_air(id: &str) -> bool {
    matches!(
        id,
        "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air"
    )
}

/// The energy domain: 10,000 FE, every face accepts a fill, none gives (upstream's quarry
/// only consumes FE). The amount lives in the entity's `energy` int.
fn buffer() -> ModEnergyBuffer {
    ModEnergyBuffer {
        capacity: MAX_ENERGY,
        max_insert: MAX_ENERGY,
        max_extract: 0,
        insert_faces: [true; 6],
        extract_faces: [false; 6],
    }
}

/// The item domain: nine slots, shown as a vanilla 3 x 3 grid. Slice 1 has no external
/// automation, so no face reaches a slot for a hopper.
fn container() -> ModContainer {
    ModContainer {
        slots: INVENTORY_SLOTS,
        menu: Some(MENU.to_owned()),
        ..ModContainer::default()
    }
}

/// `/quarryplus info`'s one-line summary of a quarry.
fn info_line(area: &Area, mining: &Mining, inventory: &str) -> String {
    let state = if mining.finished { "done" } else { "working" };
    format!(
        "Quarry: {}/{} FE, mined {} of {}, {state}, inventory {inventory}",
        mining.energy,
        MAX_ENERGY,
        mining.mined,
        area.len()
    )
}

/// The inventory as `item xN` pairs, in slot order; `empty` when it holds nothing.
fn inventory_summary(items: &[Option<ItemStack>], host: &dyn mod_api::Host) -> String {
    let stacks: Vec<String> = items
        .iter()
        .flatten()
        .filter(|stack| stack.count() > 0)
        .map(|stack| format!("{} x{}", stack.item().id(host), stack.count()))
        .collect();
    if stacks.is_empty() {
        "empty".to_owned()
    } else {
        stacks.join(", ")
    }
}

/// The block entity's nine slots, read one by one (the SDK publishes no whole-inventory
/// read for a saved handle).
fn slots_of(host: &dyn mod_api::Host, entity: BlockEntityRef) -> Vec<Option<ItemStack>> {
    (0..INVENTORY_SLOTS as u32)
        .map(|slot| entity.item_slot(host, slot).ok().flatten())
        .collect()
}

/// An item id as the registry key the game answers for it.
fn item_key(host: &dyn mod_api::Host, id: &str) -> Option<ItemKey> {
    host.item_key(&Identifier::parse(id).ok()?)
}

/// The authoritative level the server ticks now: the one a block entity's tick belongs to (a
/// parked level's block entities are not ticked, and a client of a server ticks no level).
fn in_place_level(host: &mut dyn ServerHost) -> Option<LevelRef> {
    host.levels()
        .list_levels()
        .into_iter()
        .find(|level| level.in_place(host))
}

/// The quarry block entity at `pos`, if the block there is ours.
fn quarry_at(
    host: &dyn mod_api::Host,
    level: LevelRef,
    pos: runtime::Pos,
) -> Option<BlockEntityRef> {
    let block = level.block(host, pos)?;
    (block.id(host) == QUARRY).then(|| host.block_entity(level, pos))?
}

/// What one target asks of the runtime: `None` when the level cannot answer (the quarry
/// waits), `Some(cost)` otherwise, where `cost` is `None` for air, a fluid, a waterlogged
/// block or an unbreakable one.
fn target_cost(
    host: &dyn mod_api::Host,
    level: LevelRef,
    pos: runtime::Pos,
) -> Option<Option<i32>> {
    let block = level.block(host, pos)?;
    if is_air(&block.id(host)) {
        return Some(None);
    }
    if level
        .fluid_state(host, pos)
        .is_some_and(|fluid| !fluid.is_empty())
    {
        return Some(None);
    }
    let hardness = level
        .block_state(host, pos)
        .and_then(|state| host.block_states().destroy_speed(state));
    Some(hardness.and_then(break_cost))
}

/// One server tick of a quarry: inspect the next unvisited target, skip air/fluids and
/// unbreakable blocks, advance the four-tick break budget, and destroy a block once with
/// collected drops.
fn quarry_tick(host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) {
    let pos = event.pos;
    let Some(level) = in_place_level(host) else {
        return;
    };
    let mut changed = false;
    if event.entity.items.len() < INVENTORY_SLOTS {
        event.entity.items.resize(INVENTORY_SLOTS, None);
        changed = true;
    }
    let area = match read_area(&event.entity.nbt) {
        Some(area) => area,
        None => {
            let area = Area::default_at(pos, DEFAULT_RADIUS, DEFAULT_DEPTH);
            write_area(&mut event.entity.nbt, &area);
            changed = true;
            area
        }
    };
    let before = read_mining(&event.entity.nbt);
    let mut machine = before;
    if area.is_empty() || machine.cursor > area.len() {
        machine.cursor = area.len();
        machine.finished = true;
    }
    loop {
        if machine.finished {
            break;
        }
        let Some(target) = area.target(machine.cursor) else {
            machine.finished = true;
            break;
        };
        // Wait for a chunk that is not loaded rather than skipping a block the quarry will
        // be able to mine when the chunk comes back.
        if !level.is_loaded(&*host, target) {
            break;
        }
        let Some(cost) = target_cost(&*host, level, target) else {
            break;
        };
        match runtime::tick(&mut machine, &area, cost) {
            Tick::Working | Tick::NoEnergy | Tick::Done => break,
            Tick::Skipped => continue,
            Tick::Broke { pos: at, .. } => {
                let tool = item_key(&*host, TOOL).map(ItemStack::of);
                let drops = level
                    .destroy_block(&mut *host, at, true, tool.as_ref(), true)
                    .ok()
                    .flatten()
                    .unwrap_or_default();
                for drop in drops {
                    let _ = store_drop(&*host, &mut event.entity.items, drop);
                }
                changed = true;
                break;
            }
        }
    }
    if machine != before {
        write_mining(&mut event.entity.nbt, &machine);
        changed = true;
    }
    // Ticking alone must not dirty the chunk: the tick is `Changed` only when it did.
    if !changed {
        *event.reply = BlockEntityTickReply::Unchanged;
    }
}

/// Merge one rolled drop into the quarry's slots: same item and damage into partial stacks
/// first, then empty slots, at most the item's stack size. A full inventory keeps the
/// leftover out (slice 1 has no overflow buffer).
fn store_drop(host: &dyn mod_api::Host, items: &mut [Option<ItemStack>], drop: ItemStack) -> u32 {
    let max_stack = host
        .max_stack_size(drop.item())
        .unwrap_or(FALLBACK_MAX_STACK)
        .max(1);
    let existing: Vec<Option<DropStack>> = items
        .iter()
        .map(|slot| {
            slot.as_ref()
                .map(|stack| DropStack::new(&stack.item().id(host), stack.count(), stack.damage()))
        })
        .collect();
    let plan = drop_plan(
        &existing,
        &DropStack::new(&drop.item().id(host), drop.count(), drop.damage()),
        max_stack,
    );
    let mut placed = 0;
    for (slot, amount) in plan {
        let Some(current) = items.get_mut(slot) else {
            continue;
        };
        match current {
            Some(stack) => {
                let grown = stack.count().saturating_add(amount);
                let _ = stack.set_count(grown);
            }
            None => {
                let mut copy = drop.clone();
                let _ = copy.set_count(amount);
                *current = Some(copy);
            }
        }
        placed += amount;
    }
    drop.count().saturating_sub(placed)
}

/// The command's success feedback, routed to its originating source (the player who typed
/// it, the console, a command block) as the command service does.
fn feedback(host: &mut dyn ServerHost, line: &str) {
    let _ = host.commands().feedback(Text::literal(line));
}

/// The quarry: its block entity (ticking, nine slots) and its commands.
struct Quarry;

// A block entity Part is also the block's Part (the same type answers both), so the trait
// bound needs an implementation; every `BlockPart` method has its default here — this
// slice adds no block behaviour of its own.
#[mod_api::horizon_part]
impl BlockPart for Quarry {}

#[mod_api::horizon_part]
impl BlockEntityPart for Quarry {
    /// The quarry ticks on the server (`EntityBlock.getTicker`).
    fn block_entity_ticks(&self, _host: &dyn mod_api::Host, event: &mut BlockEntityTicksEvent<'_>) {
        event.ticks = true;
    }

    fn block_entity_tick(&self, host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) {
        quarry_tick(host, event);
    }
}

#[derive(Default)]
struct Mod;

#[mod_api::horizon_mod]
impl Hooks for Mod {
    /// Declare the quarry's block entity, held by its block, with the FE buffer and the nine
    /// slots the game's default capabilities give it.
    fn register(&self, r: &mut dyn Registrar, _: &RegistrationContext) {
        let block = r.block_key(QUARRY);
        let kind = r.block_entity_type(QUARRY_TYPE).blocks(&[QUARRY]).build();
        let _ = r.block_entity_part(&kind, Quarry);
        r.block_energy(&block, buffer());
        r.block_container(&block, container());
    }

    /// `/quarryplus info|energy|area`.
    fn register_commands(&self, _host: &mut dyn ServerHost, event: &mut RegisterCommandsEvent<'_>) {
        event
            .commands
            .push(Command::new("quarryplus info <pos:block_pos>"));
        event.commands.push(Command::op(
            "quarryplus energy <amount:integer> <pos:block_pos>",
            2,
        ));
        event.commands.push(Command::op(
            "quarryplus area <pos:block_pos> <radius:integer> <depth:integer>",
            2,
        ));
    }

    /// Run the `/quarryplus` commands a player types.
    fn run_mod_command(
        &self,
        host: &mut dyn ServerHost,
        event: &mut RunModCommandEvent<'_>,
    ) -> Flow {
        let invocation = event.command;
        let literals: Vec<&str> = invocation.literals.iter().map(String::as_str).collect();
        let Some(CommandArgumentValue::BlockPos(pos)) = invocation.argument("pos") else {
            return Flow::Continue;
        };
        let pos = *pos;
        let Some(level) = host
            .levels()
            .list_levels()
            .into_iter()
            .find(|level| level.dimension() == invocation.dimension)
        else {
            return Flow::Continue;
        };
        match literals.as_slice() {
            ["quarryplus", "info"] => {
                let Some(entity) = quarry_at(&*host, level, pos) else {
                    feedback(host, "No Quarry there.");
                    return Flow::Cancel;
                };
                let Some(data) = entity.data(&*host) else {
                    feedback(host, "No Quarry there.");
                    return Flow::Cancel;
                };
                let area = read_area(&data)
                    .unwrap_or_else(|| Area::default_at(pos, DEFAULT_RADIUS, DEFAULT_DEPTH));
                let mining = read_mining(&data);
                let inventory = inventory_summary(&slots_of(&*host, entity), &*host);
                feedback(host, &info_line(&area, &mining, &inventory));
            }
            ["quarryplus", "energy"] => {
                let Some(CommandArgumentValue::Integer(amount)) = invocation.argument("amount")
                else {
                    feedback(host, "The energy must be a whole number.");
                    return Flow::Cancel;
                };
                let Some(entity) = quarry_at(&*host, level, pos) else {
                    feedback(host, "No Quarry there.");
                    return Flow::Cancel;
                };
                let energy = (*amount).clamp(0, MAX_ENERGY);
                let Some(mut data) = entity.data(&*host) else {
                    feedback(host, "No Quarry there.");
                    return Flow::Cancel;
                };
                put(&mut data, "energy", energy);
                if entity.replace_data(host, data).is_err() {
                    feedback(host, "No Quarry there.");
                    return Flow::Cancel;
                }
                feedback(
                    host,
                    &format!("Quarry energy set to {energy}/{MAX_ENERGY} FE."),
                );
            }
            ["quarryplus", "area"] => {
                let (
                    Some(CommandArgumentValue::Integer(radius)),
                    Some(CommandArgumentValue::Integer(depth)),
                ) = (invocation.argument("radius"), invocation.argument("depth"))
                else {
                    feedback(host, "The radius and depth must be whole numbers.");
                    return Flow::Cancel;
                };
                let Some(entity) = quarry_at(&*host, level, pos) else {
                    feedback(host, "No Quarry there.");
                    return Flow::Cancel;
                };
                let Some(mut data) = entity.data(&*host) else {
                    feedback(host, "No Quarry there.");
                    return Flow::Cancel;
                };
                let area = Area::default_at(pos, *radius, *depth);
                let mining = read_mining(&data);
                write_area(&mut data, &area);
                write_mining(
                    &mut data,
                    &Mining {
                        energy: mining.energy,
                        ..Mining::default()
                    },
                );
                if entity.replace_data(host, data).is_err() {
                    feedback(host, "No Quarry there.");
                    return Flow::Cancel;
                }
                feedback(host, &format!("Quarry area set to {} blocks.", area.len()));
            }
            _ => return Flow::Continue,
        }
        Flow::Cancel
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_mining() -> Mining {
        Mining {
            energy: 3_210,
            progress: 2,
            cursor: 19,
            mined: 5,
            skipped: 1,
            finished: false,
        }
    }

    #[test]
    fn the_mining_state_round_trips_through_the_saved_compound() {
        let mining = sample_mining();
        let mut nbt = Compound::new();
        write_mining(&mut nbt, &mining);
        assert_eq!(read_mining(&nbt), mining);
    }

    #[test]
    fn a_hand_written_compound_is_clamped_to_the_slice() {
        let mut nbt = Compound::new();
        put(&mut nbt, "energy", MAX_ENERGY + 1);
        put(&mut nbt, "progress", MINE_TICKS + 3);
        put(&mut nbt, "cursor", -4);
        let mining = read_mining(&nbt);
        assert_eq!(mining.energy, MAX_ENERGY);
        assert_eq!(mining.progress, MINE_TICKS - 1);
        assert_eq!(mining.cursor, 0);
    }

    #[test]
    fn the_area_round_trips_through_the_saved_compound() {
        let area = Area::default_at([4, 70, -9], 2, 5);
        let mut nbt = Compound::new();
        write_area(&mut nbt, &area);
        assert_eq!(read_area(&nbt), Some(area));
        assert_eq!(read_area(&Compound::new()), None);
    }

    #[test]
    fn the_three_vanilla_air_blocks_are_the_only_air() {
        assert!(is_air("minecraft:air"));
        assert!(is_air("minecraft:cave_air"));
        assert!(is_air("minecraft:void_air"));
        assert!(!is_air("minecraft:stone"));
        assert!(!is_air("quarryplus:quarry"));
    }

    #[test]
    fn the_buffer_takes_energy_on_every_face_and_gives_none() {
        let energy = buffer();
        assert_eq!(energy.capacity, MAX_ENERGY);
        assert_eq!(energy.max_extract, 0);
        assert_eq!(energy.insert_faces, [true; 6]);
        assert_eq!(energy.extract_faces, [false; 6]);
    }

    #[test]
    fn the_inventory_is_nine_slots_shown_as_a_three_by_three_menu() {
        let container = container();
        assert_eq!(container.slots, INVENTORY_SLOTS);
        assert_eq!(container.menu.as_deref(), Some(MENU));
        // Slice 1 has no external automation: no face reaches a slot.
        assert!(container.faces.iter().all(Vec::is_empty));
        assert!(container.inputs.is_empty());
        assert!(container.outputs.is_empty());
    }

    #[test]
    fn the_info_line_reports_energy_progress_state_and_inventory() {
        let area = Area::default_at([0, 100, 0], 1, 3);
        let done = Mining {
            energy: 1,
            mined: 27,
            finished: true,
            ..Mining::default()
        };
        assert_eq!(
            info_line(&area, &done, "minecraft:cobblestone x27"),
            "Quarry: 1/10000 FE, mined 27 of 27, done, inventory minecraft:cobblestone x27"
        );
        let working = Mining::default();
        assert!(
            info_line(&area, &working, "empty")
                .ends_with("mined 0 of 27, working, inventory empty")
        );
    }
}
