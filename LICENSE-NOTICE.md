# Licence notice — QuarryPlus

This port carries the original mod's assets and data and rewrites its behaviour
in Rust. The original licence and copyright notices are reproduced verbatim in
[LICENSE](LICENSE).

## Where the licence was read

| | |
|---|---|
| Pack jar | `AdditionalEnchantedMiner-1.21.1-neoforge-21.1.165.jar` (739,879 B, SHA-256 `6d3f8b43044ed66beeba44c3bc4e10089f35ac2ef9843e5a33c88391cd5fca9d`) |
| Declared licence | `GNU Lesser General Public License v3.0` in `META-INF/neoforge.mods.toml` |
| Licence text | the FSF LGPL-3.0 text, byte-identical to the copy shipped in the Powah 6.2.10 jar; the QuarryPlus jar bundles no licence file and only names the licence in `META-INF/neoforge.mods.toml` (7,654 B) |
| SPDX identifier | `LGPL-3.0-only` |
| Copyright line | `Copyright (c) Kotori316 and QuarryPlus contributors` |

The jar ships no separate licence file. The jar names the author(s) `Kotori316` but
carries no separate project copyright line, so the copyright line above uses
that author list; replace it with the upstream `LICENSE` copyright statement if
one is added later.

## The port

- Original: QuarryPlus 21.1.165 by
  Kotori316 — LGPL-3.0-only
  (https://github.com/Kotori316/QuarryPlus).
- Port to Horizon by AdLf0208: the Rust runtime, the Horizon declarations and the
  build/verification tooling in this repository.
- This repository is the complete corresponding source: the licence requires
  that it travels with every copy and with every modified version.
- No Mojang file is included. The carried files are the mod's own assets and
  data; `data/minecraft/tags/…` files only add this mod's ids to vanilla tags.
