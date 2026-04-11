# ⛏️ mc-cli: Open Source Minecraft Server Manager

`mc-cli` is a lightweight, cross-platform, terminal-based utility written in Rust designed to completely automate the lifecycle of a Minecraft server. Jump straight from an empty directory to a fully-running server with an interactive dashboard in seconds.

## ✨ Features

- **Multi-Provider Backend:** Pull dynamically from Paper, official Vanilla Mojang, and Fabric Meta API endpoints.
- **Automated Resource Management:** Automatically downloads standalone server `.jar` files safely, validates cryptographic checksums (SHA-1 / SHA-256), enforces Java footprint checks, and natively auto-generates your `eula.txt`.
- **Interactive TUI Dashboard:** Replaces the messy default stdout stream with a stunning native `ratatui` interface containing:
  - Color-coded scrolling server console (Red errors, Yellow warnings).
  - Real-time `sysinfo` parsing showing the server's live CPU % and RAM usage inside the container.
  - Live socket player count tracking (`Joined` vs `Left`).
- **Server Input Console:** Send commands natively to the underlying Java wrapper directly from the dashboard input bar!
- **Graceful Shutdowns:** Hitting `Ctrl-C` safely forces a soft-shutdown, pumping a `/stop\n` command explicitly into the JVM `stdin` pipelines to ensure world data saves properly.

## 🚀 Getting Started

### Prerequisites
- [Rust & Cargo](https://rustup.rs/) (latest stable, min Edition 2024)
- Java Runtime Environment (JRE) mapped to your system's `PATH`.

### Installation

Clone the repository and build from source:

```bash
cd mc-cli
cargo build --release
```

To install it globally so you can use it from any terminal session:

```bash
cargo install --path .
```

## 💻 Usage

`mc-cli` operates using a primary `start` command to spin up instances with minimal configuration.

```bash
mc-cli start --version 1.21.1
```

By default, this will spin up a **Paper** server with **2G** of RAM. All components are cached locally inside a `server/` subdirectory to avoid re-downloading on subsequent boots!

### CLI Arguments

You can dynamically configure your provider natively via cli flags:

```bash
# Start a Fabric server with 4 Gigabytes of memory
mc-cli start --version 1.21.1 --ram 4G --provider fabric

# Start a Vanilla server on version 1.20
mc-cli start --version 1.20 --ram 1G --provider vanilla
```

### Checking Available Versions
To query what versions are currently available to play cleanly from their native upstream API structures:

```bash
mc-cli list-versions --provider paper
mc-cli list-versions --provider fabric
```

## 🛠️ Built With
- `tokio` (Async runtime and Child sub-process stream piping)
- `ratatui` & `crossterm` (Interactive terminal graphics)
- `reqwest` & `serde` (API validation algorithms)
- `sysinfo` (Live hardware allocations)  
- `sha1` / `sha2` (Cryptographic verification hooks)

## 🤝 Contribution Requirements

If you wish to contribute to the project, please respect the rules outlined in our initial SRS constraints:
1. **Performance over Bloat:** All basic operations excluding API networking must evaluate inside `<100ms`.
2. **Crash Resilience:** Ensure graceful handler teardowns so block-data never corrupts during terminal interrupts.
3. **Cross Platform Form-Factor:** Only adopt Rust native dependencies verified against Windows, MacOS, and standard Linux targets.
