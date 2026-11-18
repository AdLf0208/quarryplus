# QuarryPlus — Horizon port (slice 1: the Quarry)

A native Horizon port of **QuarryPlus 21.1.165** (`Kotori316/QuarryPlus`,
**LGPL-3.0-only**): an FE-powered Quarry block that mines a bounded area
downward and collects the drops into its internal inventory.

**LOCAL-ONLY** until the maintainer approves publishing (ARCHITECTURE.md §8).
LGPL is licence rank 1: a port may be published under the same licence with
its complete corresponding source, but approval is still required.

Provenance, hashes and the upstream commit are in
`ports/_shared/carry-work/quarryplus/SOURCE.md`; the behaviour reading is in
`FINDINGS.md`; the ladder and gaps are in `notes.md`.

## Layout (flat SDK package, mod API 0.2)

```
horizon-mod.toml                   the only manifest and the only identity
Cargo.toml                         one crate; horizon-mod-api from .horizon/sdk/
src/lib.rs                         the one impl mod_api::Hooks + the Quarry Part
src/runtime.rs                     pure area/order/cost/tick/drop logic + 13 tests
assets/quarryplus/                 carried models, blockstates, textures, item, lang
data/quarryplus/                   carried block, recipe, loot, creative tab
data/minecraft/tags/block/mineable/pickaxe.json
quarryplus.hmod                    the delivered package (code + content, one file)
tools/e2e.sh                       the in-game scenario (needs a launcher-built game)
```

## Build and verify

`cargo horizon-mod` fetches the pinned SDK into `.horizon/sdk/` (gitignored, never
packed); `build` packs, it does not compile against the game.

```sh
cd ports/quarryplus
CARGO_INCREMENTAL=0 cargo test                      # 13 runtime + 7 glue tests
CARGO_INCREMENTAL=0 cargo clippy --all-targets -- -D warnings
cargo fmt --check                                   # never --all: it would read the SDK
rm -f Cargo.lock                                    # a packed lock changes the artifact
CARGO_INCREMENTAL=0 cargo horizon-mod build
cp target/horizon-mod/quarryplus.hmod ./quarryplus.hmod
cargo horizon-mod check quarryplus.hmod             # exit 0
cargo horizon-mod fits quarryplus.hmod --build nightly
```

`SELFTEST=1 tools/e2e.sh` flips the main assertion and must fail.

## In game

Place `quarryplus:quarry`, feed FE (or `/quarryplus energy <amount> <pos>` op 2),
and it mines the default 3 × 3 × 8 shaft below it. `/quarryplus info <pos>`
prints the FE, the mined count and the inventory; `/quarryplus area <pos>
<radius> <depth>` (op 2) sets the bounded shaft. Frames/markers, the advanced
quarry, the pump, modules, chunk loading and the custom GUI are later slices.
