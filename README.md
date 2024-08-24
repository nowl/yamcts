# (Yet Another) Monte Carlo Tree Search

[![Crates.io link](https://img.shields.io/crates/v/yamcts)](https://crates.io/crates/yamcts)

This is an implementation of Monte Carlo Tree Search (MCTS) in rust.

The features that _potentially_ make this library a bit different from the many other
MCTS libraries out there:
- Zero-dependency (all dependencies are optional)
- Pluggable RNG (default uses nanorand::WyRand)
- Multi-threaded

## Usage

### Add dependency

```bash
cargo add yamcts
```

or in the `Cargo.toml` file

```toml
[dependencies]
yamcts = "0.1.0"
```

### Running MCTS

- Implement [GameState](src/lib.rs)
- Call `run_with_duration` or `run_with_iterations` on the game state to calculate next MCTS move
- `apply_move` on the game state and repeat

#### Example implementing [Nim 21 variation](https://en.wikipedia.org/wiki/Nim)

see [examples/nim.rs](examples/nim.rs)

Run with `cargo run --example nim` or `cargo run --release --example nim`

### Using a custom random number generator

The default RNG uses [nanorand](https://docs.rs/nanorand/0.7.0/nanorand/index.html) but if you
don't want that dependency and/or would like to use a different RNG it's just necessary to
implement `RngProvider` and `Rng`.

```rust
use rand::prelude::*;

// Wrapper struct, in this case rand::StdRng
struct CustomRng(StdRng);

// Implement RngProvider to return an instance of CustomRng/StdRng
impl yamcts::rng::RngProvider for CustomRng {
    fn init() -> Self {
        CustomRng(StdRng::from_entropy())
    }
}

// Implement Rng for gen_range
impl yamcts::rng::Rng for CustomRng {
    fn gen_range(&mut self, bounds: Range<usize>) -> usize {
        self.0.gen_range(bounds)
    }
}
```

Then to use this RNG:
```rust
let mcts = yamcts::MCTS::<CustomRng>::default();
```


## License

This project is licensed under the MIT License. See the [LICENSE file](./LICENSE) for details.
