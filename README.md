# 56sim

A satirical single-player "People's Republic" desktop game: open dossiers, denounce your neighbours, and try to survive Room 101. Built for the 70th anniversary of the 1956 Hungarian revolution.

This is fiction and satire. 56sim is unaffiliated with any party, state, or secret police, past or present. The only thing being liquidated here is the spare time of the player.

Desktop port of the iOS original: one Rust codebase, GTK4 + libadwaita, shipping for macOS, Windows, and Linux (x86-64 and arm64), with FreeBSD and OpenBSD supported from source.

## Features

- **Comrade enrolment** — the profile setup wizard makes you *import* an existing republic file (`.avh`) **or** swear in a new one. Not both. The Party does not do both.
- **A republic in a database** — all state lives in SQLite; the entire republic exports and imports as a single `.avh` file, byte-compatible with the iOS original.
- **The people's registry** — citizen dossiers, with interrogation in **Room 407** (a llama.cpp language model, linked as a library, plays the interrogator and decides your verdict), Radio Moscow bulletins read aloud by **audio.cpp** TTS (Voxtral-4B-TTS-2603), and a propaganda poster foundry.
- **The marches of the people** — endlessly regenerated procedural Soviet military marches as background music. New March at will; the hammer must be heard.
- **The living republic** — citizens denounce one another every 15 seconds, trust collapses on its own, and five denunciations or zero trust auto-neutralises a citizen. One Archive seal per citizen, as is proper.
- **The Ministry of Love** — denounce often enough and it comes for you: seven denunciations against your own alias and the screen belongs to Big Brother ("YOU LOVED BIG BROTHER.").
- **Five-Year Plan** — quarterly congress policies, surprise inspections, auditor challenges, and yearly decrees.
- **Defector: Escape 1956** — a 44-node text adventure with six endings, none of them guaranteed.
- **The People's Paper** — a newspaper generated from the actual event log, plus a sortable **Archive of the People**.
- **Net-rank vigilance** — a leaderboard of who has denounced the most, including the NPCs denouncing *you*.
- **Administrative suicide** — wiping the republic requires correctly answering three bureaucratic questions first.

## Requirements

- For source builds: Rust (see `rust-toolchain.toml`), GTK4 ≥ 4.10, libadwaita ≥ 1.6, cmake + ninja (to build the bundled audio.cpp plugin), and a Vulkan SDK / shaderc if you want the Vulkan LLM/TTS backends. Per-OS dependency lists are in [BUILD-DESKTOP.md](BUILD-DESKTOP.md).
- The interrogator LLM (`Ministral-3-3B-Instruct-2512` GGUF, 2.15 GB) and the TTS voice (`Voxtral-4B-TTS-2603` GGUF, 4.7 GB) download on first use from Hugging Face, verified against pinned SHA-256 hashes.

## Build & test

```
cargo test -p sim-core -p marchgen
cargo build --release -p sim-app            # add --features cuda,vulkan,tts as desired
```

## Releases

Precompiled builds from [GitHub Releases](../../releases): macOS arm64 (`.dmg`), Windows x86-64 and arm64 (`.zip` + NSIS setup), Linux x86-64 and arm64 (Flatpak bundle, AppImage, `.tar.xz`). They are deliberately **unsigned** — that is a feature, not an oversight. BSDs compile from source.

## License

MIT, see [LICENSE.md](LICENSE.md). The republic belongs to the people. Third-party notices and the licenses of the downloaded model weights live in `LICENSES/`.
