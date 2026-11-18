// Pure Quarry logic for the QuarryPlus slice 1 (standalone crate lib, injected
// as `crate::game::quarryplus`). No game access lives here: the dig area, its
// top-down order, the FE cost of a break, the mining tick and the drop plan.
//
// Upstream keeps a quarry's work as an `Area` (frame bounds), a `targetPos`
// iterator and a `PowerEntity` energy buffer. A plain quarry mines one layer
// at a time from the top down, one block per action, and each break costs
// `hardness * 25 FE` (no enchantments) with a 10,000 FE buffer. This slice
// keeps the arithmetic and drops pure; the block entity and the world calls
// live in `support.rs`.

pub type Pos = [i32; 3];

/// Upstream `PowerMap.Default.QUARRY.maxEnergy`: the plain quarry's buffer.
pub const MAX_ENERGY: i32 = 10_000;
/// Upstream `PowerMap.Default.QUARRY.breakBlockBase`, in FE per hardness.
pub const BREAK_BLOCK_BASE: i32 = 25;
/// Ticks one block takes in this slice: the head's travel plus the break.
pub const MINE_TICKS: i32 = 4;
/// The default horizontal radius: a 3 x 3 shaft.
pub const DEFAULT_RADIUS: i32 = 1;
/// The default depth: a bounded shaft for slice 1.
pub const DEFAULT_DEPTH: i32 = 8;
/// The largest playable radius (the harness command clamps to it).
pub const MAX_RADIUS: i32 = 64;
/// The largest playable depth (the harness command clamps to it).
pub const MAX_DEPTH: i32 = 256;

/// The inclusive box of blocks a quarry mines. Upstream's `Area` bounds the
/// frame and the iterator digs its interior; this slice keeps the dig volume
/// itself, so the box is the exact set of targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Area {
    pub min: Pos,
    pub max: Pos,
}

/// An area whose minimum bound is above its maximum on one axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AreaError {
    pub min: Pos,
    pub max: Pos,
}

impl std::fmt::Display for AreaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "area min {:?} is not at or below max {:?}",
            self.min, self.max
        )
    }
}

impl std::error::Error for AreaError {}

impl Area {
    /// A box from `min` to `max`, both included. Fails when any axis is
    /// inverted (upstream's `Area` constructor does the same).
    pub fn new(min: Pos, max: Pos) -> Result<Self, AreaError> {
        if (0..3).all(|axis| min[axis] <= max[axis]) {
            Ok(Self { min, max })
        } else {
            Err(AreaError { min, max })
        }
    }

    /// The slice's default shaft: a `2 * radius + 1` square below `origin`,
    /// `depth` blocks tall, ending one block under the quarry. Radius and
    /// depth are clamped to `0..=MAX_RADIUS` and `1..=MAX_DEPTH` so the box is
    /// valid and its length cannot overflow.
    pub fn default_at(origin: Pos, radius: i32, depth: i32) -> Self {
        let radius = radius.clamp(0, MAX_RADIUS);
        let depth = depth.clamp(1, MAX_DEPTH);
        Self {
            min: [origin[0] - radius, origin[1] - depth, origin[2] - radius],
            max: [origin[0] + radius, origin[1] - 1, origin[2] + radius],
        }
    }

    /// The number of blocks in the box.
    pub fn len(&self) -> usize {
        let span = |axis: usize| (self.max[axis] - self.min[axis] + 1) as usize;
        span(0) * span(1) * span(2)
    }

    /// A valid [`Area`] always holds at least one block; the method exists so
    /// `len` follows the collection convention and handles a hand-made box.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[allow(dead_code, reason = "pure area API; exercised by the runtime tests")]
    pub fn contains(&self, pos: Pos) -> bool {
        (0..3).all(|axis| self.min[axis] <= pos[axis] && pos[axis] <= self.max[axis])
    }

    /// Target `index` in dig order: top layer first, then z, then x. `None`
    /// past the end, which is how the mining tick knows the work is done.
    pub fn target(&self, index: usize) -> Option<Pos> {
        if index >= self.len() {
            return None;
        }
        let dx = (self.max[0] - self.min[0] + 1) as usize;
        let dz = (self.max[2] - self.min[2] + 1) as usize;
        let per_layer = dx * dz;
        let layer = index / per_layer;
        let within = index % per_layer;
        Some([
            self.min[0] + (within / dz) as i32,
            self.max[1] - layer as i32,
            self.min[2] + (within % dz) as i32,
        ])
    }
}

/// The FE a plain break costs: upstream's no-enchantment formula
/// `hardness * breakBlockBase`, truncated to whole FE. `None` for an
/// unbreakable block (negative or infinite hardness). Zero hardness and NaN
/// cost nothing, as upstream's `getBreakEnergy` returns 0 for them.
pub fn break_cost(hardness: f32) -> Option<i32> {
    if hardness.is_nan() {
        return Some(0);
    }
    if hardness < 0.0 || hardness.is_infinite() {
        return None;
    }
    Some((f64::from(hardness) * f64::from(BREAK_BLOCK_BASE)) as i32)
}

/// The saved progress of one quarry: its FE buffer, its position in the dig
/// order, the ticks spent on the current block, and the counters the readout
/// prints. The area itself is kept by the caller (it is a placement fact).
/// A machine placed in the world starts with no FE and at the first target.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Mining {
    pub energy: i32,
    pub progress: i32,
    pub cursor: usize,
    pub mined: u32,
    pub skipped: u32,
    pub finished: bool,
}

/// What one mining tick did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tick {
    /// The progress advanced; nothing was broken.
    Working,
    /// The block at `pos` should be destroyed and its drops stored.
    Broke { pos: Pos, cost: i32 },
    /// The target cannot be broken (air, a fluid, unbreakable): skip it.
    Skipped,
    /// The buffer is below the break cost; wait without losing progress.
    NoEnergy,
    /// Every target has been visited.
    Done,
}

/// One tick: advance a target by one tick of progress, spend the FE and
/// report the block to break. `cost` is the target's [`break_cost`], or
/// `None` when the caller wants it skipped.
pub fn tick(machine: &mut Mining, area: &Area, cost: Option<i32>) -> Tick {
    if machine.finished {
        return Tick::Done;
    }
    let Some(pos) = area.target(machine.cursor) else {
        machine.finished = true;
        return Tick::Done;
    };
    let Some(cost) = cost else {
        machine.cursor += 1;
        machine.progress = 0;
        machine.skipped += 1;
        if machine.cursor >= area.len() {
            machine.finished = true;
        }
        return Tick::Skipped;
    };
    if machine.progress + 1 < MINE_TICKS {
        machine.progress += 1;
        return Tick::Working;
    }
    if machine.energy < cost {
        return Tick::NoEnergy;
    }
    machine.energy -= cost;
    machine.progress = 0;
    machine.cursor += 1;
    machine.mined += 1;
    if machine.cursor >= area.len() {
        machine.finished = true;
    }
    Tick::Broke { pos, cost }
}

/// One item stack as the drop plan counts it (no components: the support
/// copies the original `ItemStack` into an empty slot and only merges equal
/// ids, damage and components).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stack {
    pub item: String,
    pub count: u32,
    pub damage: u32,
}

impl Stack {
    pub fn new(item: &str, count: u32, damage: u32) -> Self {
        Self {
            item: item.to_owned(),
            count,
            damage,
        }
    }
}

/// Where a drop goes: `(slot, amount)` pairs in order, merging into equal
/// partial stacks first and then filling empty slots, at most `max_stack` in
/// each. What the plan does not place is the overflow (the inventory is
/// full); the caller keeps it out of the slots.
pub fn drop_plan(slots: &[Option<Stack>], drop: &Stack, max_stack: u32) -> Vec<(usize, u32)> {
    if drop.count == 0 || max_stack == 0 {
        return Vec::new();
    }
    let mut remaining = drop.count;
    let mut plan = Vec::new();
    for (slot, existing) in slots.iter().enumerate() {
        if remaining == 0 {
            break;
        }
        if let Some(existing) = existing
            && existing.item == drop.item
            && existing.damage == drop.damage
            && existing.count < max_stack
        {
            let put = (max_stack - existing.count).min(remaining);
            if put > 0 {
                plan.push((slot, put));
                remaining -= put;
            }
        }
    }
    for (slot, existing) in slots.iter().enumerate() {
        if remaining == 0 {
            break;
        }
        if existing.is_none() {
            let put = max_stack.min(remaining);
            plan.push((slot, put));
            remaining -= put;
        }
    }
    plan
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_area_is_three_by_three_by_depth() {
        let area = Area::default_at([10, 100, -4], DEFAULT_RADIUS, DEFAULT_DEPTH);
        assert_eq!(area.len(), 3 * 3 * DEFAULT_DEPTH as usize);
        assert_eq!(area.min, [9, 100 - DEFAULT_DEPTH, -5]);
        assert_eq!(area.max, [11, 99, -3]);
        assert!(area.contains([9, 92, -5]));
        assert!(
            !area.contains([10, 100, -4]),
            "the quarry itself is not dug"
        );
        assert!(!area.contains([12, 99, -4]), "outside the radius");
    }

    #[test]
    fn the_default_area_clamps_its_size() {
        let area = Area::default_at([0, 100, 0], i32::MAX, i32::MAX);
        assert_eq!(area.max[0] - area.min[0] + 1, 2 * MAX_RADIUS + 1);
        assert_eq!(area.max[2] - area.min[2] + 1, 2 * MAX_RADIUS + 1);
        assert_eq!(area.max[1] - area.min[1] + 1, MAX_DEPTH);
    }

    #[test]
    fn an_area_rejects_inverted_bounds() {
        assert!(Area::new([1, 0, 0], [0, 0, 0]).is_err());
        assert!(Area::new([0, 1, 0], [0, 0, 0]).is_err());
        assert!(Area::new([0, 0, 1], [0, 0, 0]).is_err());
        assert!(Area::new([0, 0, 0], [0, 0, 0]).is_ok());
    }

    #[test]
    fn targets_are_top_down_then_z_then_x() {
        let area = Area::new([0, 0, 0], [1, 1, 1]).expect("a valid area");
        let got: Vec<[i32; 3]> = (0..area.len()).filter_map(|i| area.target(i)).collect();
        assert_eq!(
            got,
            vec![
                [0, 1, 0],
                [0, 1, 1],
                [1, 1, 0],
                [1, 1, 1],
                [0, 0, 0],
                [0, 0, 1],
                [1, 0, 0],
                [1, 0, 1],
            ]
        );
        assert_eq!(area.target(area.len()), None);
    }

    #[test]
    fn every_target_is_unique_and_inside_the_area() {
        let area = Area::default_at([3, 64, 3], 1, 3);
        let mut seen = std::collections::HashSet::new();
        for index in 0..area.len() {
            let pos = area.target(index).expect("a target for every index");
            assert!(area.contains(pos), "{pos:?} is not inside the area");
            assert!(seen.insert(pos), "{pos:?} is visited twice");
        }
        assert_eq!(seen.len(), area.len());
    }

    #[test]
    fn break_costs_follow_the_no_enchantment_formula() {
        assert_eq!(break_cost(1.5), Some(37));
        assert_eq!(break_cost(3.0), Some(75));
        assert_eq!(break_cost(0.0), Some(0));
        assert_eq!(break_cost(f32::NAN), Some(0));
        assert_eq!(break_cost(-1.0), None);
        assert_eq!(break_cost(f32::INFINITY), None);
        assert_eq!(break_cost(f32::NEG_INFINITY), None);
    }

    #[test]
    fn mining_takes_four_ticks_and_charges_the_break() {
        let area = Area::new([0, 0, 0], [0, 0, 0]).expect("a valid area");
        let mut machine = Mining {
            energy: MAX_ENERGY,
            ..Mining::default()
        };
        assert_eq!(tick(&mut machine, &area, Some(37)), Tick::Working);
        assert_eq!(tick(&mut machine, &area, Some(37)), Tick::Working);
        assert_eq!(tick(&mut machine, &area, Some(37)), Tick::Working);
        assert_eq!(machine.progress, MINE_TICKS - 1);
        assert_eq!(machine.energy, MAX_ENERGY);
        assert_eq!(
            tick(&mut machine, &area, Some(37)),
            Tick::Broke {
                pos: [0, 0, 0],
                cost: 37
            }
        );
        assert_eq!(machine.energy, MAX_ENERGY - 37);
        assert_eq!(machine.mined, 1);
        assert!(machine.finished);
        assert_eq!(tick(&mut machine, &area, Some(37)), Tick::Done);
    }

    #[test]
    fn mining_waits_without_energy_then_breaks_when_fed() {
        let area = Area::new([0, 0, 0], [0, 0, 0]).expect("a valid area");
        let mut machine = Mining {
            energy: 10,
            ..Mining::default()
        };
        for _ in 0..MINE_TICKS - 1 {
            assert_eq!(tick(&mut machine, &area, Some(37)), Tick::Working);
        }
        assert_eq!(tick(&mut machine, &area, Some(37)), Tick::NoEnergy);
        assert_eq!(machine.progress, MINE_TICKS - 1);
        assert_eq!(machine.energy, 10);
        machine.energy = 50;
        assert_eq!(
            tick(&mut machine, &area, Some(37)),
            Tick::Broke {
                pos: [0, 0, 0],
                cost: 37
            }
        );
        assert_eq!(machine.energy, 13);
    }

    #[test]
    fn unbreakable_or_empty_targets_are_skipped() {
        let area = Area::new([0, 0, 0], [1, 0, 0]).expect("a valid area");
        let mut machine = Mining::default();
        assert_eq!(tick(&mut machine, &area, None), Tick::Skipped);
        assert_eq!(machine.skipped, 1);
        assert_eq!(machine.cursor, 1);
        assert_eq!(machine.progress, 0);
        assert_eq!(
            tick(&mut machine, &area, Some(37)),
            Tick::Working,
            "the next target starts its own four ticks"
        );
    }

    #[test]
    fn the_machine_finishes_after_the_last_target() {
        let area = Area::new([0, 0, 0], [0, 0, 0]).expect("a valid area");
        let mut machine = Mining::default();
        for _ in 0..MINE_TICKS {
            tick(&mut machine, &area, Some(0));
        }
        assert!(machine.finished);
        assert_eq!(machine.cursor, 1);
        assert_eq!(tick(&mut machine, &area, Some(0)), Tick::Done);
        assert_eq!(machine.mined, 1);
    }

    #[test]
    fn drop_plan_merges_then_uses_empty_slots() {
        let slots = vec![
            Some(Stack::new("minecraft:cobblestone", 60, 0)),
            None,
            Some(Stack::new("minecraft:dirt", 1, 0)),
        ];
        let drop = Stack::new("minecraft:cobblestone", 10, 0);
        assert_eq!(drop_plan(&slots, &drop, 64), vec![(0, 4), (1, 6)]);
    }

    #[test]
    fn drop_plan_leaves_overflow_out_of_a_full_inventory() {
        let slots = vec![Some(Stack::new("minecraft:cobblestone", 64, 0))];
        let drop = Stack::new("minecraft:cobblestone", 3, 0);
        assert_eq!(drop_plan(&slots, &drop, 64), Vec::new());
    }

    #[test]
    fn drop_plan_does_not_merge_stacks_with_other_damage() {
        let slots = vec![Some(Stack::new("minecraft:iron_pickaxe", 1, 1)), None];
        let drop = Stack::new("minecraft:iron_pickaxe", 1, 2);
        assert_eq!(drop_plan(&slots, &drop, 1), vec![(1, 1)]);
    }
}
