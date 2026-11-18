# Working on this Horizon native mod

This is a **flat SDK package** (mod API 0.2): `horizon-mod.toml` + `Cargo.toml` +
`src/lib.rs` (`impl mod_api::Hooks`) + `src/runtime.rs` (the pure rules and their tests) +
`assets/` + `data/`. Horizon retired the `.hnmod` container in the mod-API merge (#16941),
so there is no `compatibility.json`, `mixins/<id>/`, `horizon-mixin.toml`, plan/injection
schema or `rust-toolchain.toml` here, and none of them should be recreated.

> **Build slots (v2):** a compile step goes through the campaign runner
> (`ports/tools/build-slot.sh` from the campaign root). It **blocks in the kernel** until a
> slot is free, so **never poll and never background a build**; a timeout exits **75**. It
> does **not** `cd` into the port — wrap the command (`bash -c 'cd ports/<id> && …'`) and
> never pass `--target-dir`. `cargo horizon-mod` itself runs directly in the port: the slot
> runner's built-in steps speak the retired tool.

```sh
ports/tools/build-slot.sh run --job sdk-qp --port ports/quarryplus --step test \
    -- bash -c 'cd ports/quarryplus && cargo test'
ports/tools/build-slot.sh run --job sdk-qp --port ports/quarryplus --step lint \
    -- bash -c 'cd ports/quarryplus && cargo fmt --check && cargo clippy --all-targets -- -D warnings'
```

- `cargo horizon-mod` fetches the pinned SDK into `.horizon/sdk/` (gitignored, never
  packed). `build` packs the `.hmod`; it does **not** compile against the game — that is
  `horizon-launcher native-build . --instance <instance> --trust-native-code`.
- **`src/runtime.rs` is byte-identical to the retired bundle's `runtime.rs` and must stay
  so**: the behaviour rules live there, `src/lib.rs` only converts SDK types to theirs.
  Never run `cargo fmt --all` (it would read the fetched SDK); `cargo fmt` leaves
  `runtime.rs` alone, but confirm with `cmp` after touching the glue.
- Only the API the SDK publishes: `cargo horizon-mod hooks <text>` finds the exact method;
  never guess a name, and record an API gap in `notes.md` instead of faking one.
- The delivered package is `quarryplus.hmod` **in the port root**, never in `target/`;
  `LICENSE` and `LICENSE-NOTICE.md` travel inside it.
- Keep the port separate from Horizon and horizon-mixins; never modify the pin checkouts,
  never commit binaries, exported sources or Minecraft assets, and never put Mojang files
  in the tree.
- `python3 ports/tools/check-skills.py --notes ports/quarryplus` must stay green, and
  `notes.md` records every claim with its command and output.
