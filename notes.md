# QuarryPlus — port notes, slice 1 (the Quarry)

Ported from **QuarryPlus 21.1.165** (`Kotori316/QuarryPlus`, **LGPL-3.0-only**),
the exact ATM10 jar `AdditionalEnchantedMiner-1.21.1-neoforge-21.1.165.jar`
(SHA-1 `52ab6b6ea613a3aee334215391fee53aab359a04`); provenance and hashes in
`ports/_shared/carry-work/quarryplus/SOURCE.md`. Upstream behaviour was read
from tag `v21.1.165` (commit `42d30e43c4c941c78666b3a9ff11da596f6cd945`) —
analysis only, no Java code was translated. `FINDINGS.md` lists the files read
and the numbers.

**LOCAL-ONLY** until the maintainer approves publishing (ARCHITECTURE.md §8).
LGPL-3.0-only is **licence rank 1 (copyleft)**: publishable under the same
licence with the complete corresponding source, but the approval is still
required.

**Class + tier:** `content` / **`machines`** (`ports/ledger.csv`; provenance
`ports/_shared/source-map/atm10-sources.csv` row `quarryplus`). Approach per
Horizon's `docs/modpacks.md`: a block entity, an FE capability, an item
container, a server tick and the carried block/item/recipe/lang — **not** a
data-only carry. The queue role is **mechanics, core**.

**Status:** slice 1 implemented and verified in-game on the **retired `.hnmod`**
(2026-11-09). The bundle was then migrated to the **flat SDK package, version
0.2.0** (2026-11-18): same runtime, same behaviour, same carried content, one
`.hmod` — see `## Migration to the SDK format` at the end, which is the current
state (the in-game leg has **not** been re-run at the new pin, so the port stays
`ported-partial`). The Quarry block, a 10,000 FE buffer, a bounded 3 × 3 shaft,
top-down mining at `25 × hardness` FE per block, drops collected into a
nine-slot internal inventory, and the carried content. Frames/markers, the advanced quarry, pump/fluid handling,
enchantments/upgrades, chunk loading and the custom GUI are later slices.

## Behaviour (slice 1)

The pure arithmetic is `runtime.rs`; the world work is the Part and the `impl Hooks` in `src/lib.rs` (the retired bundle kept it in `mixins/quarryplus/support.rs`).

| | Upstream | This port |
|---|---|---|
| energy | `PowerMap.Default.QUARRY.maxEnergy` **10,000 FE**, insert-only | same, `native_block_entity_energy` (API 142), `energy` in the saved compound |
| area | marker/frame `Area`; a plain placement is an 11 × 11 × 5 box whose **9 × 9 interior** is dug | a bounded **3 × 3 × depth** box centred on the quarry; default depth **8** |
| order | one Y layer at a time from the top, serpentine per layer | top layer first, then z, then x (`Area::target`) |
| speed | one state-machine action per tick: head travel (1 FE/block) then break | one block per **4 ticks** (`MINE_TICKS`), the head-travel budget folded into the tick |
| break cost | `hardness × 25 FE` with no enchantments, truncated | `break_cost(hardness)`: **37 FE** for stone (1.5), 0 for hardness 0/NaN, skipped for negative/infinite |
| unbreakable/air/fluid | skipped (fluids only with the pump module) | `cost = None` skips air, water/lava/waterlogged blocks (the `Fluid` fact) and unbreakable blocks |
| drops | fake player + netherite pickaxe through `Block.getDrops`, converted into an unbounded `MachineStorage`; x/z % 3 positions also sweep item entities, XP and chest minecarts | `native_machine_destroy_block(pos, netherite pickaxe, collect = true)` (API @API:machine_breaking@); the rolled drops go into **9 slots** (`drop_plan` merges same item+damage, then empty slots) |
| floor | `DigMinY`: `minBuildHeight + 1` unless set | the configured depth ends the shaft (bounded slice 1) |
| state | `FINISHED, WAITING, … REMOVE_FLUID, FILLER`, `working` block state | `working`/`done` in the readout; the blockstate's `working` property is carried but stays false |
| GUI | custom `GeneralScreenHandler` | a vanilla `minecraft:generic_3x3` menu over the nine slots |

### Hooks (the retired `.hnmod`'s seven injections; the SDK 0.2 mapping is in `## Migration to the SDK format` → *Helper → hook table*)

| Hook (API) | Helper | What it does |
|---|---|---|
| `block_entity` (15/129) | `block_entity.rs` | Gives `quarryplus:quarry` a block entity. |
| `block_entity_ticks` (15) | `ticks.rs` | The quarry ticks on the server. |
| `block_entity_energy` (142) | `energy.rs` | 10,000 FE, all faces insert, none extract. |
| `block_entity_container` (16/17) | `container.rs` | Nine slots, vanilla 3 × 3 menu. |
| `block_entity_tick` (17) | `tick.rs` | One mining tick: skip, advance, break, collect. |
| `register_commands` (8) | `register.rs` | `/quarryplus info\|energy\|area`. |
| `perform_command` (8) | `perform.rs` | The command bodies. |

### Commands (harness controls)

- `/quarryplus info <x> <y> <z>` — FE, `mined N of T`, `working|done`, and the
  inventory summary.
- `/quarryplus energy <amount> <x> <y> <z>` (op 2) — sets the FE buffer.
- `/quarryplus area <x> <y> <z> <radius> <depth>` (op 2) — sets the bounded
  shaft and restarts the scan.

These are the in-game controls the e2e uses; the quarry works without them
(the default area is 3 × 3 × 8). Native commands are typed through
`ctl chat "/…"` because `ctl command` bypasses `perform_command`.

## Slice boundary and gaps

Implemented: the Quarry block, its FE buffer, the bounded top-down shaft, the
per-block FE cost, the nine-slot collected-drop inventory, the commands, and
the carried block/item/recipe/loot/lang/models/textures/pickaxe tag.

1. **No frames or markers.** The default area is a centred 3 × 3 × 8 box,
   settable per block; upstream's marker/frame area (and the 9 × 9 default
   interior dug to `minBuildHeight + 1`) is a later slice.
2. **No mining animation.** Upstream moves an animated head at 1 FE/block and
   breaks on arrival; this port charges the whole break at the end of a fixed
   four-tick budget.
3. **No enchantments/upgrades.** The break cost is the no-enchantment
   `25 × hardness`; efficiency/unbreaking/fortune/silk-touch coefficients,
   modules (pump, bedrock, filter, repeat-tick, exp) and the creative variants
   are not implemented.
4. **No fluids, bedrock, XP, item-entity sweep or chest minecarts.** Fluids are
   skipped (the pump module is absent); the x/z % 3 drop sweep, experience and
   minecart modules are gaps.
5. **The internal inventory is nine slots**, not upstream's unbounded
   `MachineStorage`; a full inventory leaves the overflow out (no overflow
   buffer). The recipe viewer/GUI is the vanilla 3 × 3 menu, not the custom
   screen.
6. **`facing` stays at its default north.** The arrival hook names the block
   id, not the placed state; the carried `orientable_with_bottom` model is
   still loaded. The `working` state is carried but never set, so the idle top
   texture shows.
7. **No chunk loading**, no advanced quarry, no pump, no save-to-item and no
   energy item.
8. **The carried recipe is not craftable**: it needs `#quarryplus:markers`,
   `c:storage_blocks/redstone`, `c:ingots/iron` and `c:obsidians`, which this
   slice does not register. It is carried unchanged.
9. **The carried loot table's `quarryplus:machine_loot_function` is replaced**
   by the vanilla self-drop pool; the quarry item drop is unchanged.
10. `tools/test_template.py`/`test_stage.py` are the inherited template
    fixtures (one injection, one module), not tests of this seven-injection,
    two-module bundle; the real ladder is the evidence.

## Upstream source analysis

- Repository: <https://github.com/Kotori316/QuarryPlus> (LGPL-3.0-only), tag
  `v21.1.165`, commit `42d30e43c4c941c78666b3a9ff11da596f6cd945`; shallow clone
  in `ports/_shared/upstream/quarryplus` (gitignored).
- Files read: `machine/quarry/QuarryEntity.java`, `QuarryBlock.java`,
  `QuarryState.java`, `QuarryStorageHolder.java`, `machine/Area.java`,
  `PowerEntity.java`, `PowerMap.java`, `misc/DigMinY.java`, `WorkResult.java`,
  `neoforge/.../QuarryBlockNeoForge.java`, and the jar's
  `assets/quarryplus/**` + `data/quarryplus/**` +
  `data/minecraft/tags/block/mineable/pickaxe.json`.
- Behaviours extracted: the placement box and the 9 × 9 interior, the
  serpentine top-down dig iterator, the 10,000 FE buffer, the
  `hardness × 25 FE` break cost, the fake-player netherite-pickaxe drops, the
  x/z % 3 sweep, the fluid/bedrock/XP modules, the `working`/`facing` states,
  and the jar's content.
- Where the rewrite differs: the slice boundary and gaps above. No Java code
  was copied or translated; the numbers come from `PowerMap`/`QuarryBlock` and
  the behaviour from reading the state machine.

Skills: superpowers/using-superpowers, brainstorming, writing-plans, test-driven-development, systematic-debugging, verification-before-completion; BeautifulRust/rust-best-practices

## Verification (the retired `.hnmod` bundle)

Pin **`1d63e0a5a37a7a708eb0b32b49bf0dfeb28ed8a9`**, mixins
**`5a64af938ec86b00d53d16a408aabc0038fb3e57`** (schema 6). Every build, test
and e2e ran through `ports/tools/build-slot.sh` with `CARGO_INCREMENTAL=0`,
`--horizon ports/_shared/pins/horizon-1d63e0a`; the e2e used the pin-matched
shared `horizon-ctl` (`ports/_shared/bin/horizon-1d63e0a5/`). Evidence logs:
`ports/_shared/evidence/quarryplus/`.

### Ladder

| Step | Command (through the slot runner) | Result |
|---|---|---|
| runtime tests (RED) | `cargo test --offline` before the API existed | **66 errors**, the new tests' API missing (`runtime-red.log`) |
| runtime tests (GREEN) | `cargo test --locked --offline` | **13 passed**, 0 failed |
| runtime lints | `cargo fmt --check`; `cargo clippy --all-targets --locked --offline -- -D warnings` | pass, **0 warnings** |
| fetch | `cargo fetch --locked` on the staged lock | exit 0 (once, online) |
| stage | `tools/horizon.py stage` | exit 0, revision `1d63e0a…`, 7 injections / 2 modules |
| check | `tools/horizon.py check` | `cargo_exit: 0`, **0 warnings** (`horizon-check.log`) |
| test | `tools/horizon.py test` | **13 passed**, 1240 filtered (`horizon-test.log`) |
| build | `tools/horizon.py build` | `cargo_exit: 0` (fast) |
| e2e | `build-slot.sh e2e -- … -- tools/e2e.sh` | exit 0, `PASS`, `EXIT:0` (`e2e-default.log`) |
| selftest | `SELFTEST=1 … tools/e2e.sh` | **exit 1** with the flipped assertion (`e2e-selftest.log`) |
| embed ladder (this slice) | `cargo fmt --check`; `rustfmt --check mixins/quarryplus/embedded_content.rs`; `cargo clippy … -D warnings`; `cargo test` | 0 warnings, **13 passed** |
| package | `tools/package.sh` (content `.hmod` → `data/` → `.hnmod` → native check) | `.hmod` 6,110 B; `.hnmod` 17,884 B; native check exit 0 |
| e2e, one file only | `EMBEDDED=1 CONTROL=1 … tools/e2e.sh` | **PASS**, `EXIT:0`; the control shows the ids unknown (`e2e-embedded.log`) |
| embed fail-capability | `EMBEDDED=1 SELFTEST=1 … tools/e2e.sh` | **exit 1** (`e2e-embedded-selftest.log`) |
| `.hmod` | `cargo-horizon-mod build/check --json` | **6,110 B**, SHA-256 `43544aff…be19`, 16 entries, content only |
| `.hnmod` (content embedded) | `tools/package.py` | **17,884 B**, SHA-256 `e45b45f9…198e`, 13 entries incl. `mixins/quarryplus/data/quarryplus-content.hmod` |
| load check | `tools/hmod-load.sh` | `PASS` + `EXIT:0`; negative: `Unknown block type 'quarryplus:quarry'` |

### In-game scenario (quoted from `e2e-default.log`)

```
== A: 1,000 FE mines the stone cube and collects the drops ==
  cube 0,99,0 -> minecraft:air; 0,97,0 -> minecraft:air
  info(0,100,0): Quarry: 1/10000 FE, mined 27 of 27, done, inventory minecraft:cobblestone x27
  inventory: minecraft:cobblestone x27
== B: no FE leaves the cube untouched (control) ==
  info(8,100,0): Quarry: 0/10000 FE, mined 0 of 27, working, inventory empty
  cube 8,99,0 still minecraft:stone; inventory 0
== C: no target blocks mines nothing and keeps the FE (control) ==
  info(16,100,0): Quarry: 1000/10000 FE, mined 0 of 27, done, inventory empty
== D: a block outside the area stays (control) ==
  info(24,100,0): Quarry: 963/10000 FE, mined 1 of 27, done, inventory minecraft:cobblestone x1
  inside 25,97,0 -> minecraft:air; outside 27,97,0 -> minecraft:stone; inventory 1
PASS: 1,000 FE mined the 3x3x3 stone cube into 27 cobblestone in the quarry's inventory, no FE mined nothing, an empty area mined nothing and kept its FE, and a block outside the area stayed
EXIT:0
```

What the numbers prove: 27 stone cost **999 FE** and left **1 FE**, the exact
no-enchantment cost; the **same** cube with 0 FE stayed stone (so the break is
energy-gated); the empty shaft kept all **1,000 FE** and finished; the
out-of-area block stayed while the inside one was mined for exactly **37 FE**
(1,000 → 963), pinning the per-block cost and the bound. `SELFTEST=1` flips
A's inventory assertion and fails:

```
(selftest: asserting A's inventory is empty)
FAIL: expected /quarryplus info to say 'inventory empty', got 'Quarry: 1/10000 FE, mined 27 of 27, done, inventory minecraft:cobblestone x27'
```

### Content package and load check

`content/quarryplus.hmod` **6,110 B**, SHA-256
`43544aff65af9af96e462525d12414f522b3b057e527a7b707546fe7dc91be19`,
**16 entries** (manifest, block declaration, blockstate, two block models, item
model + 26.3 item definition, four textures, lang, recipe, loot, creative tab,
pickaxe tag). `cargo-horizon-mod check --json` reports content only, API 0.2,
`"capabilities":["registries"]`, `side: both`, no code.

`tools/hmod-load.sh` (evidence `hmod-load.log`) placed and gave
`quarryplus:quarry` on the shared unmodded `1d63e0a5` game with
`--dev-mod content/quarryplus.hmod`; the **same world without `--dev-mod`**
answers `Unknown block type 'quarryplus:quarry'` and gives no item, so the
check cannot pass vacuously.

### Embedded content (AE2 pattern) — one listable file

The registry keeps **one file per project** (playbook §1.0): an `.hnmod` is code
and an `.hmod` is content, so a split port could not be listed faithfully. This
bundle now carries its content, exactly as the merged AE2 entry does:

- **`mixins/quarryplus/data/quarryplus-content.hmod`** is the built
  `content/quarryplus.hmod`, byte for byte (6,110 B,
  `43544aff65af9af96e462525d12414f522b3b057e527a7b707546fe7dc91be19`).
  `tools/package.sh` rebuilds the content mod, copies it in, and
  `tools/package.py` packs `data/**` as `mixins/<id>/data/…` (top-level
  `.rs`/`.toml` plus everything under `data/`) with fixed zip timestamps, so a
  rebuild reproduces the SHA-256. It refuses a `data/<id>-content.hmod` that is
  not byte-identical to `content/<id>.hmod`, so a rebuilt content mod can never
  ship stale inside the bundle.
- **`mixins/quarryplus/embedded_content.rs`** with the `embedded-content`
  `[[injection]]` (`target = "horizon_client::game::mods::content_root"`,
  `kind = "HEAD"`, `cancellable = true`, bindings `root`/`enabled`/`dev`)
  compiles the file in with `native_data!` (API 41), writes it to
  `.horizon-cache/quarryplus-embedded-content.hmod` when the copy on disk
  differs, appends it to the dev-mod list and returns its `content_root`. The
  helper calls `content_root` once more and stops at its own
  `dev.contains(&file)` guard, so the second call runs the vanilla body — the
  same one-level recursion AE2 uses. An unwritable cache logs a warning and
  `Continue`s (no content) instead of failing the game.

Verified in game with **only the bundle** (`EMBEDDED=1`, no `--dev-mod`): the
whole scenario A–D passes, and the file the helper wrote is byte-identical to
`content/quarryplus.hmod`. The control (`CONTROL=1`) launches the same binary
with `.horizon-cache` replaced by a plain file, so the helper refuses its
content: the quarry item never arrives and
`setblock … quarryplus:quarry` leaves `minecraft:air` — the ids are unknown
without the embedded file, so the PASS is not vacuous.

This makes the `.hnmod` the **single deliverable** of this port: one file
installs the quarry block, its item, models, textures, recipe, loot and lang
**and** the block entity, FE buffer, inventory and tick. It does not by itself
publish it (licence approval and the copyleft reviewer note are the publisher's
step, ARCHITECTURE.md §8).

#### In-game transcript (quoted from `e2e-embedded.log`)

```
== launch (Overworld arena, content + native bundle) ==
== control: the same game with no embedded content file ==
  launch: no --dev-mod; the bundle carries its own content
  without the embedded file: no quarry item, the block stayed minecraft:air
  launch: no --dev-mod; the bundle carries its own content
  embedded content: 6110 B at /tmp/opencode/quarryplus-home.IuFxo5/.horizon-cache/quarryplus-embedded-content.hmod, identical to content/quarryplus.hmod
== A: 1,000 FE mines the stone cube and collects the drops ==
  cube 0,99,0 -> minecraft:air; 0,97,0 -> minecraft:air
  info(0,100,0): Quarry: 1/10000 FE, mined 27 of 27, done, inventory minecraft:cobblestone x27
  inventory: minecraft:cobblestone x27
== D: a block outside the area stays (control) ==
  inside 25,97,0 -> minecraft:air; outside 27,97,0 -> minecraft:stone; inventory 1
PASS: 1,000 FE mined the 3x3x3 stone cube into 27 cobblestone … a block outside the area stayed
EXIT:0
```

The same scenario with `SELFTEST=1` (still one file, no `--dev-mod`) ends
`EXIT:1` at the flipped A assertion (`e2e-embedded-selftest.log`), so the
one-file run is fail-capable.

Evidence for this slice:
[`ports/_shared/evidence/quarryplus/`](../_shared/evidence/quarryplus/)
(`embed-ladder.log` — fmt/clippy/test/package/check/test/build,
`package.log` — both artifacts and the native check, `e2e-lease.log` — the two
legs, `e2e-embedded.log`, `e2e-embedded-selftest.log`).

### Native package

`quarryplus.hnmod` **17,884 B**, SHA-256
`e45b45f933752a24657487ce7840833dc8e19eb5ef9c8aa74ea1bb211b0a198e`, **13
entries**: `horizon-native.json` (schema 1, game 26.3, `horizon` = the campaign
pin, `mixins` = `5a64af93…`), the eleven bundle files and
`mixins/quarryplus/data/quarryplus-content.hmod`.
`cargo horizon-mod check --kind native-mod` reports
`native-mod, format native-1/1d63e0a5…/5a64af93…, for 26.3`, 17,884 B, the same
SHA-256 (`../_shared/evidence/quarryplus/package.log`). It sits in the port root beside
`content/`, as the campaign requires; the old duplicate `quarryplus-0.1.0.hnmod`
(same bytes, stale) was removed so one artifact is the deliverable.

## Campaign gates at this touch

- `check-e2e-logs.py quarryplus --strict` → **5/5** logs with a matching
  `EXIT:` verdict (`e2e-default`, `e2e-selftest`, `e2e`, `e2e-embedded`,
  `e2e-embedded-selftest`).
- `upstream_ready.py --copyleft quarryplus` → **READY** (the port is now one
  `.hnmod` that carries its content, so the split-port blocker is gone and the
  artifact is newer than its sources).
- The licence was read from the jar itself:
  `META-INF/neoforge.mods.toml` `license = "GNU Lesser General Public License
  v3.0"`, matching the brief's LGPL-3.0-only / rank 1. `precheck-mod.py`
  returns exit 2 only because the shared `licence-audit.csv` has not recorded
  this jar (that file is outside this job's write ownership; the jar read is
  in `SOURCE.md`).
- `check-pin-updates.py` reports the pin 2,645 commits behind latest
  (`7fd1b913…`), with **+2 hooks (`world_hold`, `world_warp`) / +3 helpers
  (`force_chunk`, `force_chunk_in`, `forced_chunks`)**. None is used by this
  slice; the job stays on the briefed `1d63e0a` pin as instructed.
- No `ARCHITECTURE.md`, ledger, pin checkout, `horizon/` or `horizon-mixins/`
  file was edited.

## Reviewer repairs (2026-11-11) — PR review items and what changed


Each item below is quoted from `ports/_shared/horizon-mods/reviews/pr-*-quarryplus.md` (the review is the specification), with what this repair did about it.

- **Reviewer:** LGPL-3.0 source review is the reviewer's limitation
  **Repair:** Not a code item: the registry accepts copyleft and the complete corresponding source travels with the repository.
- **Reviewer:** PR body names the older artifact hash
  **Repair:** The repackage produced a new hash; the PR body must be refreshed on re-push (orchestrator).
- **Reviewer:** licence notice
  **Repair:** `LICENSE` + `LICENSE-NOTICE.md` in the `.hnmod`, `data/quarryplus/license/` in the carried `.hmod`.
- **Reviewer:** registry artifact delivery
  **Repair:** **Staging (orchestrator, not ours)**: the artifact is not in registry release/staging as `<sha256>.<ext>`; the orchestrator answers it with `horizon-mods#686` (no write access to the registry's releases). Every repackaged artifact below has a fresh sha256 for that re-push, and the author release must be re-uploaded from the port repository.

Skills: superpowers/using-superpowers,systematic-debugging,verification-before-completion; BeautifulRust/rust-best-practices

## Migration to the SDK format (2026-11-18, SDK `0b14f8a`)

Horizon's mod-API merge (#16941, launcher ≥ 0.2.70) retired the `.hnmod` container (#17992),
and the registry closed this port's submission (PR **#827**) with *"republish this as a native
`.hmod` with the SDK from the next Nightly"*. The port is now **one flat package** that carries
both halves:

- **Pin:** Horizon **`0b14f8acfd97fa2c750e42795249dcc981178ca4`** (nightly 2026-11-18), horizon-mixins
  **`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`**, mod API **`0.2`**, game 26.3. The SDK is fetched
  into `.horizon/sdk/` by any `cargo horizon-mod` command and is never packed.
- **Version:** the port's own **0.2.0** (the retired bundle was 0.1.0; the content half called
  itself 21.1.165, the upstream mod's version, which is not the port's identity).
- **Class + tier unchanged:** class **content**, tier **machines** — a block entity with an FE
  buffer, an item container and a server tick over the same pure runtime. No feature is added or
  removed by the migration.

### File map — created

| Path | What it holds |
|---|---|
| `horizon-mod.toml` | the only manifest and the only identity: `id`, `version = "0.2.0"`, `api = "0.2"`, `side = "both"`, `capabilities = ["registries"]`, `[native]` pinned to `0b14f8a` / `e59da90`. `cargo horizon-mod build` rewrites it to its canonical form and drops `license`/`homepage`/`issues` (trap 16); the licence travels as `LICENSE` + `LICENSE-NOTICE.md` inside the package |
| `Cargo.toml` | one crate; the only dependency is `horizon-mod-api` from `.horizon/sdk/` (no `rust-version`: the scaffold declares none, and a floor above the box's 1.97.0 refuses to build) |
| `src/lib.rs` | the one `#[mod_api::horizon_mod] impl Hooks` (`register`, `register_commands`, `run_mod_command`), the `Quarry` Part (`BlockPart` + `BlockEntityPart`), the NBT/command glue and 7 glue tests |
| `src/runtime.rs` | the retired `mixins/quarryplus/runtime.rs`, **byte-identical** (`cmp` clean, SHA-256 `7a8ff3b154b2b332d117b8276cfe37977b21bc8cd94044162eb19757b0118281`); its 13 tests ran green before any glue existed |
| `assets/quarryplus/**`, `data/quarryplus/**`, `data/minecraft/tags/block/mineable/pickaxe.json` | the carried content, `cmp`-clean from the retired `content/` |
| `quarryplus.hmod` | the delivered package, port root |
| `.gitignore` | `/target`, `/.horizon`, `/Cargo.lock` |

### File map — deleted (and where it went)

| Deleted | Why | Where it went |
|---|---|---|
| `mixins/quarryplus/` (`horizon-mixin.toml`, `runtime.rs`, `support.rs`, `embedded_content.rs`, seven per-hook helpers, `data/quarryplus-content.hmod`) | the container, the plan/injection schema and the per-hook helper files are not part of any SDK package; `runtime.rs` moved to `src/`; the hook bodies became two `BlockEntityPart` methods, two registrar calls and two `Hooks` methods | `ports/_shared/evidence/quarryplus/retired-hnmod-0.1.0/mixins/` |
| `quarryplus.hnmod` (17,884 B) | unloadable on launcher ≥ 0.2.70 | same folder |
| `content/` (its `horizon-mod.toml`, `quarryplus.hmod` 6,110 B, `target/`) | a second package; the flat package carries `assets/` and `data/` itself | `assets/` + `data/` at the package root; the rest to the evidence folder |
| `compatibility.json` | `[native]` in `horizon-mod.toml` is the pin | evidence folder |
| `rust-toolchain.toml` (1.95.0) | not part of any new package; the pin decides the toolchain | evidence folder |
| `tools/horizon.py`, `package.py`, `package.sh`, `test_stage.py`, `test_template.py`, `stamp_license.py`, `audit/`, `__pycache__`, `hmod-load.sh` | the staged/mixin build machinery, the `.hnmod` packer, the content stamper and the content-only load check; none applies to a package that is code *and* content at once | evidence folder |
| `.github/` (`workflows/ci.yml`) | the template's fixture workflow clones horizon-mixins and reads `compatibility.json` | evidence folder |
| `Cargo.lock` | not needed without registry dependencies, and a packed lock changes the artifact (see trap 5) | not kept |

**Kept on purpose:** `tools/e2e.sh` (the scenario is still the behaviour's only oracle; its header
now says it needs a launcher-built game and the two `.horizon-cache` legs are gone), `FINDINGS.md`,
`README.md`/`AGENTS.md`/`VALIDATION.md` (rewritten for the flat layout), `LICENSE`,
`LICENSE-NOTICE.md` (both travel inside the `.hmod`), this file.

**The embedded content is gone.** The retired bundle wrote its content half to `.horizon-cache/`
from a `content_root` injection (the AE2 pattern) because the registry keeps one file per project.
The flat SDK package carries `assets/` and `data/` **inside its own `.hmod`**, so the injection, the
static `native_data!` blob, the cache write, its control leg and `stamp_license.py` are all deleted
— that workaround is the migration's biggest simplification.

### Helper → hook table (the retired seven, and where each one lives now)

| Retired (`.hnmod`) | SDK 0.2 | Where |
|---|---|---|
| `[[injection]] hook = "block_entity"` + `block_entity.rs` (`native_block_entity`, API 15/129) | **no hook**: `r.block_entity_type("quarry").blocks(&["quarryplus:quarry"]).build()` declares the entity, and a declared type starts with an empty compound (the `block_entity` Part method is only needed to seed data) | `impl Hooks::register` |
| `hook = "block_entity_ticks"` + `ticks.rs` | **`BlockEntityPart::block_entity_ticks`**, `event.ticks = true` | `impl BlockEntityPart for Quarry` |
| `hook = "block_entity_energy"` + `energy.rs` (`native_block_entity_energy`, API 142) | **`r.block_energy(&block, ModEnergyBuffer { … })`** — the default energy domain; the amount stays the `energy` int in the saved compound | `impl Hooks::register` |
| `hook = "block_entity_container"` + `container.rs` (API 16/17) | **`r.block_container(&block, ModContainer { … })`** — nine slots and the vanilla `minecraft:generic_3x3` menu | `impl Hooks::register` |
| `hook = "block_entity_tick"` + `tick.rs` (API 17) | **`BlockEntityPart::block_entity_tick`**; `event.entity` is the saved `nbt`/`items`/`data`, `event.pos` the position | `impl BlockEntityPart for Quarry` |
| `hook = "register_commands"` + `register.rs` (API 8) | **`Hooks::register_commands`** with `Command::new` / `Command::op` | `impl Hooks for Mod` |
| `hook = "perform_command"` + `perform.rs` (raw string split) | **`Hooks::run_mod_command`** returning `Flow`: typed `CommandArgumentValue::BlockPos` / `Integer`, no string parsing or `parse_pos` | `impl Hooks for Mod` |
| `[[module]] runtime` / `[[module]] state` | `pub mod runtime;` (pure) + the glue in `src/lib.rs`; there is no plan section and no separate support module | `src/` |
| `[[injection]] embedded-content` (`content_root`) | — (content ships in the package) | — |

What the glue converts, and why each conversion is not a behaviour change:

- **Hardness.** `game.res.facts().hardness` is pin-internal and gone; the SDK publishes
  `host.block_states().destroy_speed(state)` (`BlockState.getDestroySpeed`), which is the same
  number upstream's `PowerMap` multiplies by 25.
- **Fluid / air.** `facts.fluid` became `level.fluid_state(host, pos).is_some_and(|f| !f.is_empty())`
  (a waterlogged block reads as fluid, as before) and `facts.is_air` became an explicit id test over
  the three vanilla air blocks (the SDK has no `isAir`).
- **Drops.** `game.native_machine_destroy_block(pos, tool, collect)` became
  `level.destroy_block(host, pos, drop, tool, collect)` — the same one level drop path, rolled with
  a netherite pickaxe and returned as stacks instead of item entities.
- **Modification marks.** `game.world.modified` is gone; the tick answers `Changed` only when it did
  something (`BlockEntityTickReply::Unchanged` otherwise), which is the engine's own dirtying rule.
- **Inventory read.** The command reads the nine slots through `BlockEntityRef::item_slot`, because
  the SDK publishes no whole-inventory read for a saved handle.

### Build and check (verbatim)

`build-slot.sh` speaks the retired tool, so `cargo horizon-mod` runs **directly in the port** with
`CARGO_INCREMENTAL=0`; only the compile/lint/test steps went through a slot.

```sh
# 1. the SDK must be fetched before anything compiles: `cargo test` alone fails on
#    `.horizon/sdk/api/crates/horizon-mod-api/Cargo.toml: No such file or directory`
cd ports/quarryplus && cargo horizon-mod build          # SDK 0b14f8a fetched, then packs
# 2. the pure runtime, before any glue existed
CARGO_INCREMENTAL=0 cargo test                          # → test result: ok. 13 passed
# 3. glue tests, formatting, lints
CARGO_INCREMENTAL=0 cargo test                          # → test result: ok. 20 passed (13 + 7)
CARGO_INCREMENTAL=0 cargo clippy --all-targets -- -D warnings   # → Finished, 0 warnings
cargo fmt --check                                       # → clean (never --all: it reads the SDK)
# 4. package, then the registry's own checks on the delivered file
rm -f Cargo.lock
CARGO_INCREMENTAL=0 cargo horizon-mod build             # → Built ./target/horizon-mod/quarryplus.hmod
cp target/horizon-mod/quarryplus.hmod ./quarryplus.hmod
cargo horizon-mod check quarryplus.hmod                 # → CHECK_EXIT:0
cargo horizon-mod fits quarryplus.hmod --build nightly  # → "0b14f8a  made for", FITS_EXIT:0
sha256sum quarryplus.hmod
```

Measured (evidence `ports/_shared/evidence/quarryplus/sdk-0b14f8a/`): **20 passed**; clippy 0
warnings; `fmt --check` clean; package **30,511 B**, SHA-256
`eb337c69080a6fb7ec1c1dbf82cc59ca094dccb6d203f6272f97b811bf346bda`, **23 entries**; a fresh-clone
repeat (`.horizon/`, `target/` and `Cargo.lock` removed) produced the **same bytes**; `check` and
`fits` both exit 0:

```
quarryplus 0.2.0 (native-mod, format native-1/0b14f8acfd97fa2c750e42795249dcc981178ca4/e59da9094f0c2fdda6c627fe9eff22dec6a906f0, for 26.3)
  30511 bytes, SHA-256 eb337c69080a6fb7ec1c1dbf82cc59ca094dccb6d203f6272f97b811bf346bda, unsigned
QuarryPlus:
0b14f8a  made for
```

The package's own `uses.json` (293 entries) names the surface the glue actually uses:
`hook:block_entity_ticks`, `hook:block_entity_tick`, `hook:register_commands`,
`hook:run_mod_command`, `method:BlockEntityPart::block_entity_tick`,
`method:DefaultCapabilitiesRegistrar::block_energy` / `::block_container`,
`handle:LevelRef::destroy_block`, `handle:BlockStates::destroy_speed`,
`handle:BlockEntityRef::item_slot` / `::data` / `::replace_data`, and the content half's
`hook:blocks` / `hook:items` / `hook:block_entity` (the carried `data/quarryplus/horizon/block/quarry.json`).

### Traps this migration cost

1. **Delete the old layout before the first build.** `error: unexpected package path:
   mixins/quarryplus/runtime.rs` — the project root may hold only package paths, so `mixins/`,
   the `.hnmod` and `content/` moved to evidence first.
2. **`cargo test` cannot fetch the SDK.** The path dependency is
   `.horizon/sdk/api/crates/horizon-mod-api`; without a fetch first it fails with
   `failed to load source for dependency horizon-mod-api … No such file or directory`. Run one
   `cargo horizon-mod build` (or `new`/`hooks`) in the port first.
3. **`build-slot.sh run` forwards to the second machine, which has no such path.** The forwarded
   `bash -c 'cd ports/quarryplus && …'` failed with `cd: ports/quarryplus: No such file or
   directory`; `BUILD_LOCAL=1` keeps the compile step local (as `repeatable_trial_vaults` recorded).
   The runner also does **not** `cd` into the port: wrap every command.
4. **`mod_api::JavaString` is not exported.** `rangedpumps`'s `fn key(name: &str) ->
   mod_api::JavaString` does not compile at this SDK. Read a compound key by comparing
   `name.0 == key.encode_utf16().collect::<Vec<_>>()` (as `justdirethings` does) and write it with
   `nbt.insert(name.into(), …)`.
5. **`Cargo.lock` is packed and changes the artifact.** Any `cargo test`/`clippy` writes one;
   `rm -f Cargo.lock` right before the build, and `/Cargo.lock` in `.gitignore`.
6. **`cargo fmt --all --check` reads the SDK** (a path dependency) and reports its `build.rs` diffs.
   Use plain `cargo fmt` / `cargo fmt --check`; `cargo fmt` leaves `runtime.rs` alone (confirmed
   with `cmp` after every glue edit).
7. **A 38-char hash reads as missing pins.** `error: horizon-mod.toml: native requires game and
   exact revisions`; both commits are 40 hex, and the mixins commit is `e59da9094f0c…`, **not** the
   retired campaign pin `5a64af93…`.
8. **`cargo horizon-mod check` with no file checks wasm**, not the package
   (`can't find crate for 'core'`); always `check <file.hmod>`.
9. **`capabilities = ["registries"]` is required** even where it reads as a claim the mod does not
   make: the manifest schema has exactly that value (here the quarry does declare a block entity
   type, so it is also true).
10. **`BlockEntityPart: BlockPart`** — the same type answers both, so an empty
    `impl BlockPart for Quarry {}` is needed beside the `BlockEntityPart` impl, and both need
    `#[mod_api::horizon_part]`.
11. **`ServerHost` writes are handle methods, not area methods**: `level.destroy_block(&mut *host, …)`
    and `entity.replace_data(host, …)`, while the reads take `&dyn Host` (`entity.data(&*host)`).
12. **The saved compound is `BTreeMap<JavaString, Tag>`** — `Tag::get(&str)` exists but only on a
    `Tag`, not on a `Compound` (see trap 4).
13. **A `block_entity_tick` that writes nothing must answer `Unchanged`** or the chunk is dirtied
    every tick; the old `mark_modified` had no equivalent and the reply is the replacement.
14. **`side` is the installation side**: `"both"`, so single player and a LAN host get the mod.
15. **`cargo horizon-mod build` rewrites `horizon-mod.toml`.** It strips `license`,
    `homepage` and `issues` (they are not part of the 0.2 schema), but it strips them *after*
    packing: a manifest the build has not canonicalised yet produces a package **with** them
    (30,559 B, `ebbb1813…`) and every later build one **without** (which is the recorded artifact, 30,511 B, `eb337c69…`). Write
    the canonical manifest, build twice, and record the second hash; do not add those fields back.
16. **The e2e could not be re-run here.** It needs a launcher-built game
    (`horizon-launcher native-build . --instance <i> --trust-native-code`) and `horizon-launcher` is
    not installed; `tools/e2e.sh` is kept with its assertions unchanged and its retired
    `.horizon-cache` legs deleted.

### Gaps after the migration

1. **No in-game run at `0b14f8a`** (no launcher, and the shared binaries are at `1d63e0a5`/
   `41f822b6`). Everything above is carried logic, type-checked glue against the pinned API crate,
   and `check`/`fits`; the first slice on a launcher-capable box runs `tools/e2e.sh` and records the
   transcript. **This is why the port stays `ported-partial`.**
2. **The menu is unverified.** `ModContainer::menu` takes a vanilla menu name (`MOD_MENUS`), and the
   port keeps `minecraft:generic_3x3`; whether the game resolves that name for a mod block at this
   pin is a gameplay fact the e2e must show (the old `.hnmod` did open it).
3. **`BlockEntityRef::item_slot` per slot** is how `/quarryplus info` reads the inventory; a
   whole-inventory read for a saved handle is still an issue-shaped gap ("read a mod container's
   slots in one call").
4. **No hardness getter on `LevelRef`**, so the glue takes `level.block_state` and
   `host.block_states().destroy_speed` in two reads; `BlockProperties::hardness` exists in
   `uses.json` from the **content** declaration, not from a runtime read.
5. **Everything the slice did not implement stays a gap** (frames/markers, the dig animation,
   enchantments/upgrades, fluids/pump, XP/item-entity sweep, chunk loading, advanced quarry,
   save-to-item, the custom GUI), exactly as the slice-1 list above says.

### Changelog for the resubmission

> Supersedes **#827**, closed because Horizon retired `.hnmod` (#17992). Changes: one flat SDK
> package with `api = "0.2"` and `[native]` pinned to `0b14f8a`/`e59da90`; the seven retired
> injections became one `impl mod_api::Hooks` (`register` declaring the block entity type, its
> 10,000 FE domain and its nine slots; `register_commands`/`run_mod_command` for
> `/quarryplus info|energy|area`) plus one `BlockEntityPart` (ticks and the mining tick); the
> carried block, item, recipe, loot, models, textures and lang now ship **inside the same
> `.hmod`** (`assets/`, `data/`), so the AE2 embedded-content workaround and its control leg are
> gone; hardness is read with `BlockStates::destroy_speed` and drops with `LevelRef::destroy_block`;
> `runtime.rs` and its 13 tests are unchanged. Version 0.2.0.

Skills: superpowers/using-superpowers,brainstorming,writing-plans,test-driven-development,systematic-debugging,verification-before-completion; BeautifulRust/rust-best-practices,rust-port
