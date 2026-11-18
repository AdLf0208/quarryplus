# QuarryPlus slice 1 — validation summary

Flat SDK package: API **0.2**, SDK nightly
**`0b14f8acfd97fa2c750e42795249dcc981178ca4`** (2026-11-18), horizon-mixins
**`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`**, game 26.3. Every step ran with
`CARGO_INCREMENTAL=0`; test/lint went through `ports/tools/build-slot.sh`, `cargo
horizon-mod` ran directly in the port (the slot runner speaks the retired tool).

| Check | Result |
|---|---|
| `runtime.rs` carried | `cmp` clean vs the retired `mixins/quarryplus/runtime.rs`; SHA-256 `7a8ff3b1…8281` |
| runtime tests (before any glue) | **13 passed**, 0 failed |
| runtime + glue tests | **20 passed**, 0 failed |
| `cargo fmt --check` / clippy `-D warnings` | exit 0 / exit 0, **0 warnings** |
| `cargo horizon-mod build` | SDK fetched; `target/horizon-mod/quarryplus.hmod` |
| fresh-clone repeat (no `.horizon`, `target`, `Cargo.lock`) | same 30,511 B, same SHA-256 |
| `cargo horizon-mod check quarryplus.hmod` | `quarryplus 0.2.0 (native-mod, format native-1/0b14f8ac…/e59da909…, for 26.3)`, **EXIT 0** |
| `cargo horizon-mod fits … --build nightly` | `0b14f8a made for`, **EXIT 0** |
| `quarryplus.hmod` | **30,511 B**, SHA-256 `eb337c69…6bda`, **23 entries** |
| in-game e2e | **not run at this pin** — needs a launcher-built game (`horizon-launcher` is not installed here) |

Evidence logs: `ports/_shared/evidence/quarryplus/sdk-0b14f8a/` (`runtime-carry.log`,
`unit.log`, `lint.log`, `build.log`, `fresh-clone.log`, `check.log`, `check.json`,
`uses.log`, `package-entries.log`). The retired bundle and its ladder stay under
`ports/_shared/evidence/quarryplus/` (`e2e-*.log`, `package.log`, …).
