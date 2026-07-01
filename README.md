# Swarm Mod: special-attacks

Special attack status effects — Hack, Drain, Overload, Debilitate, Disrupt, Fortify for Swarm
bool
string

## Directory Structure

```
mods/special-attacks/
├── Cargo.toml        # Static Bevy Plugin crate
├── mod.toml          # Mod metadata + configurable parameters
├── src/lib.rs        # `impl Plugin` entry point
└── README.md
```

## Configuration

See `mod.toml` for all configurable parameters. Server operators can override via:

```bash
swarm mod config special-attacks <key> <value>
```

Or in `world.toml`:

```toml
[mods.special-attacks.config]
# key = value
```

## Engine API

Mods are statically compiled Bevy Plugin crates. Enable this mod with the
`mod_special_attacks` Cargo feature, or with `vanilla_mods`.

## Publishing

```bash
git tag v0.1.0
git push --tags
swarm mod pack
```
