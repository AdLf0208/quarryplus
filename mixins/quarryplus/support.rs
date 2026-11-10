// Game-side glue for QuarryPlus slice 1 (injected into `crate::game` as
// `quarryplus_state`; it needs the game, so it is not part of the standalone
// library).
//
// Upstream's quarry is a `QuarryEntity` state machine over a marker/frame
// `Area`, a `PowerEntity` FE buffer and a `MachineStorage` item map. This slice
// keeps the pure area/order/cost/tick/drop arithmetic in `runtime.rs` and does
// the world work here: the block entity's saved `energy` (API 142), its
// 9-slot container (API 16/17), and one break per four ticks through
// `native_machine_destroy_block` (API @API:machine_breaking@), whose rolled
// drops are merged into the block entity's slots.

use horizon_client::blocks::Fluid;
use horizon_client::containers::{BlockEntity, ModBlockEntity, ModContainer, ModEnergyBuffer};
use horizon_client::items::ItemStack;
use horizon_client::text::Text;
use horizon_world::nbt::{Compound, JavaString, Tag};

use crate::game::native_server::{NativeCommand, NativeServerPlayer};
use crate::game::quarryplus::{
    Area, DEFAULT_DEPTH, DEFAULT_RADIUS, MAX_ENERGY, MINE_TICKS, Mining, Stack, Tick, break_cost,
    drop_plan,
};

/// The block and its item.
const QUARRY: &str = "quarryplus:quarry";
/// The pickaxe drops are rolled with (upstream uses a netherite pickaxe).
const TOOL: &str = "minecraft:netherite_pickaxe";
/// The internal inventory shown as a vanilla 3 x 3 grid.
const INVENTORY_SLOTS: usize = 9;
/// Fallback when the item registry does not answer (vanilla stacks to 64).
const FALLBACK_MAX_STACK: u32 = 64;

fn key(name: &str) -> JavaString {
    JavaString::from(name)
}

fn int(nbt: &Compound, name: &str) -> Option<i32> {
    match nbt.get(&key(name)) {
        Some(Tag::Int(value)) => Some(*value),
        _ => None,
    }
}

fn read_area(nbt: &Compound) -> Option<Area> {
    let min = [int(nbt, "minX")?, int(nbt, "minY")?, int(nbt, "minZ")?];
    let max = [int(nbt, "maxX")?, int(nbt, "maxY")?, int(nbt, "maxZ")?];
    Area::new(min, max).ok()
}

fn write_area(nbt: &mut Compound, area: &Area) {
    nbt.insert(key("minX"), Tag::Int(area.min[0]));
    nbt.insert(key("minY"), Tag::Int(area.min[1]));
    nbt.insert(key("minZ"), Tag::Int(area.min[2]));
    nbt.insert(key("maxX"), Tag::Int(area.max[0]));
    nbt.insert(key("maxY"), Tag::Int(area.max[1]));
    nbt.insert(key("maxZ"), Tag::Int(area.max[2]));
}

/// The saved mining state. The FE amount is the engine's `energy` int, the
/// same field `native_block_entity_energy` reads.
fn read_mining(nbt: &Compound) -> Mining {
    Mining {
        energy: int(nbt, "energy").unwrap_or(0).clamp(0, MAX_ENERGY),
        progress: int(nbt, "progress")
            .unwrap_or(0)
            .clamp(0, MINE_TICKS - 1),
        cursor: int(nbt, "cursor").unwrap_or(0).max(0) as usize,
        mined: int(nbt, "mined").unwrap_or(0).max(0) as u32,
        skipped: int(nbt, "skipped").unwrap_or(0).max(0) as u32,
        finished: int(nbt, "finished").unwrap_or(0) != 0,
    }
}

fn write_mining(nbt: &mut Compound, mining: &Mining) {
    nbt.insert(key("energy"), Tag::Int(mining.energy));
    nbt.insert(key("progress"), Tag::Int(mining.progress));
    nbt.insert(key("cursor"), Tag::Int(mining.cursor as i32));
    nbt.insert(key("mined"), Tag::Int(mining.mined as i32));
    nbt.insert(key("skipped"), Tag::Int(mining.skipped as i32));
    nbt.insert(key("finished"), Tag::Int(i32::from(mining.finished)));
}

fn mark_modified(game: &mut crate::game::Game<'_>, pos: [i32; 3]) {
    game.world.modified.insert((pos[0] >> 4, pos[2] >> 4));
}

/// The quarry block entity at `pos`, if it is ours.
fn quarry_at<'a>(game: &'a crate::game::Game<'_>, pos: [i32; 3]) -> Option<&'a ModBlockEntity> {
    match game.world.block_entities.get(&(pos[0], pos[1], pos[2])) {
        Some(BlockEntity::Mod(entity)) if entity.id == QUARRY => Some(entity),
        _ => None,
    }
}

/// The block declares a block entity (`native_block_entity`, API 15/129).
pub fn on_block_entity(block: &str, entity: &mut Option<(String, Compound)>) {
    if block == QUARRY {
        *entity = Some((QUARRY.to_owned(), Compound::new()));
    }
}

/// The quarry ticks on the server (`native_block_entity_ticks`, API 15).
pub fn on_block_entity_ticks(block: &str, ticks: &mut bool) {
    if block == QUARRY {
        *ticks = true;
    }
}

/// The FE buffer (`native_block_entity_energy`, API 142): 10,000 FE, all
/// faces accept, none give (upstream's quarry only consumes FE).
pub fn on_block_entity_energy(block: &str, energy: &mut Option<ModEnergyBuffer>) {
    if block == QUARRY {
        *energy = Some(ModEnergyBuffer {
            capacity: MAX_ENERGY,
            max_insert: MAX_ENERGY,
            max_extract: 0,
            insert_faces: [true; 6],
            extract_faces: [false; 6],
        });
    }
}

/// The internal inventory (`native_block_entity_container`, API 16/17): nine
/// slots shown as a vanilla 3 x 3 grid. Slice 1 has no external automation, so
/// no face is reachable by hoppers.
pub fn on_block_entity_container(block: &str, container: &mut Option<ModContainer>) {
    if block == QUARRY {
        *container = Some(ModContainer {
            slots: INVENTORY_SLOTS,
            menu: Some("minecraft:generic_3x3".to_owned()),
            ..ModContainer::default()
        });
    }
}

/// One server tick of a quarry (`native_block_entity_tick`, API 17): inspect
/// the next unvisited target, skip air/fluids/unbreakable blocks, advance the
/// four-tick break budget, and destroy a block once with collected drops.
#[allow(private_interfaces, reason = "the injected bundle is this crate's own module")]
pub fn on_block_entity_tick(
    game: &mut crate::game::Game<'_>,
    pos: [i32; 3],
    entity: &mut ModBlockEntity,
) {
    if entity.id != QUARRY {
        return;
    }
    if entity.items.len() < INVENTORY_SLOTS {
        entity.items.resize(INVENTORY_SLOTS, None);
    }
    let area = read_area(&entity.nbt).unwrap_or_else(|| {
        let area = Area::default_at(pos, DEFAULT_RADIUS, DEFAULT_DEPTH);
        write_area(&mut entity.nbt, &area);
        area
    });
    let before = read_mining(&entity.nbt);
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
        // Wait for a chunk that is not loaded rather than skipping a block
        // the quarry will be able to mine when the chunk comes back.
        if !game.world.loaded(target[0], target[2]) {
            break;
        }
        let cost = {
            let state = game.world.block(target[0], target[1], target[2]);
            if game.res.facts().is_air(state) {
                None
            } else {
                let facts = game.res.facts().get(state);
                if matches!(facts.fluid, Fluid::None) {
                    break_cost(facts.hardness)
                } else {
                    None
                }
            }
        };
        match crate::game::quarryplus::tick(&mut machine, &area, cost) {
            Tick::Working | Tick::NoEnergy | Tick::Done => break,
            Tick::Skipped => continue,
            Tick::Broke { pos: at, .. } => {
                let tool = ItemStack::new(TOOL, 1);
                let drops = game
                    .native_machine_destroy_block(at, Some(&tool), true)
                    .unwrap_or_default();
                for drop in drops {
                    store_drop(game, &mut entity.items, drop);
                }
                break;
            }
        }
    }
    write_mining(&mut entity.nbt, &machine);
    if machine != before {
        mark_modified(game, pos);
    }
}

/// Merge one rolled drop into the quarry's slots: same item and damage into
/// partial stacks first, then empty slots, at most the item's stack size.
/// A full inventory keeps the leftover out (slice 1 has no overflow buffer).
fn store_drop(
    game: &crate::game::Game<'_>,
    items: &mut [Option<ItemStack>],
    drop: ItemStack,
) -> u32 {
    let max_stack = game
        .native_max_stack_size(&drop.item)
        .unwrap_or(FALLBACK_MAX_STACK)
        .max(1);
    let existing: Vec<Option<Stack>> = items
        .iter()
        .map(|slot| {
            slot.as_ref()
                .map(|stack| Stack::new(&stack.item, stack.count, stack.damage))
        })
        .collect();
    let plan = drop_plan(
        &existing,
        &Stack::new(&drop.item, drop.count, drop.damage),
        max_stack,
    );
    let mut placed = 0;
    for (slot, amount) in plan {
        if let Some(current) = items.get_mut(slot) {
            match current {
                Some(stack) => stack.count = stack.count.saturating_add(amount),
                None => {
                    *current = Some(ItemStack {
                        item: drop.item.clone(),
                        count: amount,
                        damage: drop.damage,
                        extra: drop.extra.clone(),
                    });
                }
            }
            placed += amount;
        }
    }
    drop.count.saturating_sub(placed)
}

/// `/quarryplus info|energy|area` (`register_commands`, API 8).
pub fn on_register_commands(commands: &mut Vec<NativeCommand>) {
    commands.push(NativeCommand::new("quarryplus info <pos:block_pos>"));
    commands.push(NativeCommand::op(
        "quarryplus energy <amount:integer> <pos:block_pos>",
        2,
    ));
    commands.push(NativeCommand::op(
        "quarryplus area <pos:block_pos> <radius:integer> <depth:integer>",
        2,
    ));
}

/// Run the `/quarryplus` commands a player types in chat.
#[allow(private_interfaces, reason = "the injected bundle is this crate's own module")]
pub fn on_perform_command(
    game: &mut crate::game::Game<'_>,
    _player: &NativeServerPlayer,
    command: &str,
    ran: &mut bool,
) {
    let words: Vec<&str> = command.split_whitespace().collect();
    match words.as_slice() {
        ["quarryplus", "info", x, y, z] => {
            *ran = true;
            let Some(pos) = parse_pos(&[*x, *y, *z]) else {
                game.native_feedback(Text::literal("Usage: /quarryplus info <x> <y> <z>"));
                return;
            };
            let Some(entity) = quarry_at(game, pos) else {
                game.native_feedback(Text::literal("No Quarry there."));
                return;
            };
            let area = read_area(&entity.nbt)
                .unwrap_or_else(|| Area::default_at(pos, DEFAULT_RADIUS, DEFAULT_DEPTH));
            let mining = read_mining(&entity.nbt);
            let state = if mining.finished { "done" } else { "working" };
            game.native_feedback(Text::literal(format!(
                "Quarry: {}/{} FE, mined {} of {}, {state}, inventory {}",
                mining.energy,
                MAX_ENERGY,
                mining.mined,
                area.len(),
                inventory_summary(&entity.items),
            )));
        }
        ["quarryplus", "energy", amount, x, y, z] => {
            *ran = true;
            let Some(pos) = parse_pos(&[*x, *y, *z]) else {
                game.native_feedback(Text::literal(
                    "Usage: /quarryplus energy <amount> <x> <y> <z>",
                ));
                return;
            };
            let Ok(amount) = amount.parse::<i32>() else {
                game.native_feedback(Text::literal("The energy must be a whole number."));
                return;
            };
            let Some(BlockEntity::Mod(entity)) =
                game.world.block_entities.get_mut(&(pos[0], pos[1], pos[2]))
            else {
                game.native_feedback(Text::literal("No Quarry there."));
                return;
            };
            if entity.id != QUARRY {
                game.native_feedback(Text::literal("No Quarry there."));
                return;
            }
            let energy = amount.clamp(0, MAX_ENERGY);
            entity.nbt.insert(key("energy"), Tag::Int(energy));
            mark_modified(game, pos);
            game.native_feedback(Text::literal(format!(
                "Quarry energy set to {energy}/{MAX_ENERGY} FE."
            )));
        }
        ["quarryplus", "area", x, y, z, radius, depth] => {
            *ran = true;
            let Some(pos) = parse_pos(&[*x, *y, *z]) else {
                game.native_feedback(Text::literal(
                    "Usage: /quarryplus area <x> <y> <z> <radius> <depth>",
                ));
                return;
            };
            let (Ok(radius), Ok(depth)) = (radius.parse::<i32>(), depth.parse::<i32>()) else {
                game.native_feedback(Text::literal("The radius and depth must be whole numbers."));
                return;
            };
            let Some(BlockEntity::Mod(entity)) =
                game.world.block_entities.get_mut(&(pos[0], pos[1], pos[2]))
            else {
                game.native_feedback(Text::literal("No Quarry there."));
                return;
            };
            if entity.id != QUARRY {
                game.native_feedback(Text::literal("No Quarry there."));
                return;
            }
            let area = Area::default_at(pos, radius, depth);
            let mining = read_mining(&entity.nbt);
            write_area(&mut entity.nbt, &area);
            write_mining(
                &mut entity.nbt,
                &Mining {
                    energy: mining.energy,
                    ..Mining::default()
                },
            );
            mark_modified(game, pos);
            game.native_feedback(Text::literal(format!(
                "Quarry area set to {} blocks.",
                area.len()
            )));
        }
        _ => {}
    }
}

fn inventory_summary(items: &[Option<ItemStack>]) -> String {
    let stacks: Vec<String> = items
        .iter()
        .flatten()
        .filter(|stack| stack.count > 0)
        .map(|stack| format!("{} x{}", stack.item, stack.count))
        .collect();
    if stacks.is_empty() {
        "empty".to_owned()
    } else {
        stacks.join(", ")
    }
}

fn parse_pos(words: &[&str; 3]) -> Option<[i32; 3]> {
    Some([
        words[0].parse().ok()?,
        words[1].parse().ok()?,
        words[2].parse().ok()?,
    ])
}
