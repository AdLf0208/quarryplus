# QuarryPlus — a Horizon port

**QuarryPlus** is a Minecraft mod. Advanced mining machines for Minecraft. This repository is a **port of it to [Horizon](https://github.com/horizon-rust/horizon)**, the Rust engine for Minecraft Java 26.3. It carries the complete source of the port plus the content files it ships, and installs as a single `.hmod` package.

Upstream: [Kotori316/QuarryPlus](https://github.com/Kotori316/QuarryPlus) — licence **LGPL-3.0-only**.

| | |
|---|---|
| Upstream mod | QuarryPlus ([source](https://github.com/Kotori316/QuarryPlus)) |
| Ported to | Horizon 0b14f8a, Minecraft Java 26.3, mod API `0.2` |
| Package | `quarryplus.hmod` (install it with the Horizon launcher) |
| Licence | **LGPL-3.0-only** — the upstream mod's licence; see [Licence and credit](#licence-and-credit) |

## What this port implements

- **Reviewer:** LGPL-3.0 source review is the reviewer's limitation
- **Reviewer:** PR body names the older artifact hash
- **Reviewer:** licence notice
- **Reviewer:** registry artifact delivery

## Not yet ported

- **No frames or markers.** The default area is a centred 3 × 3 × 8 box,
- **No mining animation.** Upstream moves an animated head at 1 FE/block and
- **No enchantments/upgrades.** The break cost is the no-enchantment
- **No fluids, bedrock, XP, item-entity sweep or chest minecarts.** Fluids are
- **The internal inventory is nine slots**, not upstream's unbounded
- **`facing` stays at its default north.** The arrival hook names the block
- **No chunk loading**, no advanced quarry, no pump, no save-to-item and no
- **The carried recipe is not craftable**: it needs `#quarryplus:markers`,

## Installing

1. Install the [Horizon launcher](https://github.com/horizon-rust/horizon/releases) (0.2.70 or newer).
2. Add `quarryplus.hmod` to the instance's mods.
3. The package is pinned to a specific Horizon build; the launcher checks that pin.

## Licence and credit

QuarryPlus was created by its upstream authors; the code, models and assets in the original mod
remain theirs and are used here under **LGPL-3.0-only**. The Rust in this repository is a clean-room
rewrite for Horizon, not a translation of the original source.

- `LICENSE` — the upstream licence text.
- `LICENSE-NOTICE.md` — what this port carries and where it came from.
- The upstream project: https://github.com/Kotori316/QuarryPlus

## Technical details

```
horizon-mod.toml   package manifest (id, version, api, [native] pin)
Cargo.toml         the crate; depends on `horizon-mod-api` from the fetched SDK
src/               lib.rs (the one `impl Hooks`), runtime.rs (pure logic + tests), glue
assets/ data/      the content the package ships
tools/             content generators and the e2e scenario
```

- Build: `cargo horizon-mod build` (packages; it does not compile against the game).
- Test: `cargo test`, `cargo clippy --all-targets -- -D warnings`.
- In-game: `ports/tools/launcher-game.sh run quarryplus` in the campaign tree, or install the
  `.hmod` and play; the harness scenario is `tools/e2e.sh`.

This repository is generated from the campaign's port at `ports/quarryplus/`; the port's own
`notes.md` is the full record (decisions, traps, evidence).
