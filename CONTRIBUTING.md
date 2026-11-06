# Contributing

## Bug reports

Include:

- What you did and where. Press F12 in game to save a screenshot and the game state to the `reports` folder.
- What happened, and what the original game does there.
- Your Fallout 3 version and active plugins.

Don't attach game files or anything extracted from the game.

## Code

- Rules come from Fallout 3's data or `Fallout3.exe` 1.7.0.3. Put the function's address in a comment next to the code.
- If you don't know what the original does, mark the code `[stand-in]` and say what it stands in for.
- Some rules still come from New Vegas. Check them against Fallout 3 when you touch them.
- Every fix needs a test. Tests build their own data.
- No game files, decompiler output or executables in the repo. `crates/nvinspect/tests/repo_hygiene.rs` checks this.

## Build and test

```powershell
cargo test --release --workspace          # engine crates
cd viewer;   cargo test --release         # game
cd launcher; cargo test --release         # launcher
```

Contributions are licensed under GPL v3 or later.
