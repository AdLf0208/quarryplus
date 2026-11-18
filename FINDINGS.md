# QuarryPlus upstream findings — slice 1 (the Quarry)

Analysis of **QuarryPlus 21.1.165** (`Kotori316/QuarryPlus`, LGPL-3.0-only),
tag `v21.1.165`, commit `42d30e43c4c941c78666b3a9ff11da596f6cd945`. The exact
ATM10 jar is
`AdditionalEnchantedMiner-1.21.1-neoforge-21.1.165.jar` (SHA-1
`52ab6b6e…9a04`), provenance in
`ports/_shared/carry-work/quarryplus/SOURCE.md`. This is a behaviour reading,
not a translation; no Java was copied.

## Files read

- `common/src/main/java/com/yogpc/qp/machine/quarry/QuarryEntity.java` —
  the state machine, the break loop, drops, fluids, energy use.
- `common/src/main/java/com/yogpc/qp/machine/quarry/QuarryBlock.java` —
  properties, placement, default area, GUI open, creative-tab variants.
- `common/src/main/java/com/yogpc/qp/machine/quarry/QuarryState.java` —
  `FINISHED, WAITING, BREAK_INSIDE_FRAME, MAKE_FRAME, MOVE_HEAD,
  BREAK_BLOCK, REMOVE_FLUID, FILLER`; working = everything but finished/waiting.
- `common/src/main/java/com/yogpc/qp/machine/Area.java` — bounds, the
  frame and dig iterators, chain/edge helpers.
- `common/src/main/java/com/yogpc/qp/machine/PowerEntity.java` —
  `ONE_FE = 1_000_000_000`, energy storage, `useEnergy`/`addEnergy`.
- `common/src/main/java/com/yogpc/qp/machine/PowerMap.java` — the numbers.
- `common/src/main/java/com/yogpc/qp/machine/misc/DigMinY.java` — the
  default dig floor.
- `common/src/main/java/com/yogpc/qp/machine/WorkResult.java` — the result
  vocabulary (`SUCCESS`, `SKIPPED`, `NOT_ENOUGH_ENERGY`, …).
- `neoforge/src/main/java/com/yogpc/qp/neoforge/machine/quarry/QuarryBlockNeoForge.java`
  — registration and the GUI menu.
- the jar's `assets/quarryplus/**` (blockstates, models, textures, lang) and
  `data/quarryplus/**` (recipe, loot, tags), plus
  `data/minecraft/tags/block/mineable/pickaxe.json`.

## Behaviour extracted

### Block

- `Properties.of().mapColor(METAL).pushReaction(BLOCK).strength(1.5f, 10f)
  .sound(STONE)`: hardness **1.5**, blast resistance **10**, stone sounds.
  The jar's pickaxe tag lists `quarryplus:quarry`; the block does **not**
  call `requiresCorrectToolForDrops`, so the drop does not require a pickaxe.
- States: `working` (boolean) × `facing` (six directions); the default is
  `facing=north, working=false`. `getStateForPlacement` sets `facing` to the
  player's opposite direction.
- The block is not fire-sensitive, not random-ticking, full cube, no
  occlusion override; item group `QuarryPlus`; the item is the block item.
- Right click opens the machine GUI (a custom `GeneralScreenHandler`); when
  disabled it prints a chat message instead.

### Area and size rules

- Placement builds the area from the block **behind** the quarry (opposite
  `facing`): corner 1 = base + clockwise × 5 + up 4; corner 2 = base +
  counter-clockwise × 5 + opposite × 10. For `facing=north` that is an
  **11 × 11 × 5** box from x−5..x+5, z+1..z+11, y..y+4.
- `Area.assumeY` extends a box shorter than 4 blocks in Y to exactly 4
  blocks; the placement box is already 5 tall (distanceY = 4).
- The **dug volume is the interior**: `minX+1..maxX−1` and
  `minZ+1..maxZ−1`, i.e. **9 × 9** for the default area. A box narrower than
  2 in X or Z has no targets (`quarryDigPosIterator` returns empty).
- `QuarryMarker` links override the default area; markers and frames are out
  of slice 1.

### Dig order and depth

- `QuarryDigPosIterator` walks one Y layer at a time in a **serpentine**
  order: even Y rows run minX+1 → maxX−1, odd Y rows run the other way, with
  the Z direction alternating by the row/X parity. It starts at the layer
  passed in and goes **down**.
- `createTargetIterator` starts at `area.maxY()−1` when `digFromMaxY` is set,
  else `area.minY()−1`; the shipped default is `digFromMaxY = false`, so the
  first layer is `minY−1` (for a plain placement, the layer below the quarry).
- The floor is `DigMinY`: when unset, `level.getMinBuildHeight() + 1`; a Y
  setter item can move it. The quarry stops after the floor layer.
- The state machine runs one action per tick (no module): move the head, then
  break the block at the head; `RepeatTickModule` can run more actions/tick.
  A block therefore takes a few ticks of head travel plus one break; the
  upstream drill is animated.

### Energy (FE)

- The buffer is `PowerMap.Default.QUARRY.maxEnergy` = **10,000 FE**.
  `updateMaxEnergyWithEnchantment` multiplies it by `1 + efficiency`, so a
  plain quarry is 10,000 FE.
- `makeFrame` = **15 FE** per frame block.
- `breakBlockBase` = **25 FE**; a plain break (efficiency 0, unbreaking 0,
  fortune 0, no silk touch) costs
  `hardness × 25 FE`, truncated to a whole FE. Stone (1.5) → **37 FE**.
- `breakBlockFluid` = **125 FE** per fluid block (pump module only).
- `moveHeadBase` = **1 FE per block** of head travel; efficiency changes the
  travel factor.
- Enchantments scale the break cost: efficiency coefficient
  `1.379729661461215`, fortune `1.5874010519681996`, silk touch `4.0`,
  unbreaking divides by `1 + level`.
- Unbreakable blocks (`hardness < 0`) are skipped; NaN/zero hardness costs 0
  FE in the upstream formula. The energy must be available **before** the
  break; `useEnergy` with `force = true` (when the buffer is smaller than the
  cost) is the upstream safety net.

### Drops

- The quarry uses a **fake player** with a netherite pickaxe carrying the
  quarry's enchantments and calls
  `Block.getDrops(state, level, pos, blockEntity, player, pickaxe)` in the
  `afterBreak` path; the drops are passed through the item converter and
  added to `MachineStorage` (an effectively unbounded item map).
- Every position whose **x and z are both multiples of 3** also sweeps item
  entities within a 5-block box into storage (`itemConverter`), and optionally
  experience orbs / chest minecarts with modules.
- Breaking a block with a block entity copies its contents into the drops,
  as a player's break does.
- Fluid source blocks are skipped unless the pump module is installed; the
  pump module drains them into the fluid storage and dams the edges with
  frame blocks.

### Edge cases

- Air and any block equal to the "after break" state are not broken.
- Unbreakable blocks (negative hardness) are skipped and remembered in
  `skipped` so the iterator does not revisit them.
- A bed, a fluid, an unloaded chunk: the world calls return nothing; the
  quarry waits or skips.
- The area is validated when placed: an area with no dig positions prints
  `quarryplus.chat.quarry_no_space` and does not set the area.
- The block entity is removed cleanly: module inventory contents drop,
  chunk loading stops, energy/storage save in NBT.

## Slice 1 mapping and deviations

Slice 1 implements the **Quarry block only**: a bounded 3 × 3 shaft, an FE
buffer, a mining tick that destroys blocks and collects their drops into a
9-slot internal inventory, the carried block/item/recipe/lang/models/loot.

Deliberate deviations, all recorded in `notes.md`:

1. **Default area is 3 × 3 × 8**, centred on the quarry and settable with
   `/quarryplus area <pos> <radius> <depth>` (the harness control). Upstream's
   default is the 9 × 9 interior of the placement frame dug to
   `minBuildHeight + 1`; frames/markers are a later slice.
2. **One block per 4 ticks** (a fixed head-travel + break budget) instead of
   the animated head and 1 FE/block travel; energy is charged at the break.
3. **Break cost is `25 × hardness`** (the no-enchantment formula); the
   enchantment coefficients are not implemented.
4. **Drops go to a 9-slot vanilla `generic_3x3` menu** (a stand-in for the
   custom GUI), not to the unbounded `MachineStorage`; the x/z % 3 item-entity
   sweep, experience and minecart modules are not implemented.
5. **The block's `facing` stays at its default north**, because the arrival
   hook names the block id, not the placed state; the model is the carried
   `orientable_with_bottom` chain.
6. **No fluids, no pump/bedrock/filter modules**, no chunk loading, no
   advanced quarry, no GUI/screen, no energy item, no save-to-item.
7. The carried loot table's custom `quarryplus:machine_loot_function` is
   replaced by the vanilla self-drop shape; the item drop is unchanged.
8. The carried recipe is kept as-is; it needs `#quarryplus:markers`,
   `c:storage_blocks/redstone`, `c:ingots/iron` and `c:obsidians`, which this
   slice does not register, so it is not craftable yet (documented gap).
