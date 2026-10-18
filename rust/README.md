# Setup

## nix (linux)

```bash
nix-shell
```

## Other

install [rustup](https://rust-lang.org/tools/install/)

run 
```shell
rustup toolchain install stable --profile minimal
rustup target add riscv32i-unknown-none-elf --toolchain stable
rustup component add llvm-tools --toolchain stable
cargo install cargo-binutils
```

The native frontend accepts White and Black controller modes as positional
arguments. Available modes are `player`, `ai-rand`, `ai-greed`, and
`ai-lookahead-1` through `ai-lookahead-15`; both default to `ai-greed` when
omitted. Player moves use coordinate notation such as `e2e4`.

```shell
cargo run-native -- player ai-greed
cargo run-native -- ai-rand player
```

# Loading

`cargo run-powder` builds the Powder frontend, converts it to a raw image, and launches The Powder Toy with `config.lua`. Run `./build.sh` instead when only the raw image is needed.

> you need r4plot.lua in this dir for the cargo run commands to work properly