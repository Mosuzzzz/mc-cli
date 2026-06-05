# ⛏️ mc-cli

> A blazing-fast, terminal-native Minecraft server manager written in Rust.

`mc-cli` automates the full lifecycle of a Minecraft server — from downloading the right jar to running a real-time TUI dashboard — with a single command.

---

## ✨ Features

- 🚀 **One-command launch** — download, configure, and start a server in seconds
- 🖥️ **Interactive TUI dashboard** — real-time console, CPU/RAM usage, and player count
- 🔄 **Auto-restart on first boot** — automatically restarts after initial world generation so clients can connect immediately
- 📦 **Multi-provider support** — Paper, Vanilla, and Fabric
- ✅ **Checksum verification** — SHA-1/SHA-256 validated downloads (Paper & Vanilla)
- 🔒 **Secure self-update** — downloads pre-built binary from GitHub Releases, verifies SHA-256 before installing
- 🔓 **Offline mode by default** — `online-mode=false` pre-configured; use `--online` flag to require premium accounts
- ♻️ **In-TUI restart** — type `restart` in the console bar without stopping mc-cli
- ☕ **Java version guard** — warns you if your Java is too old for the requested server version
- 🩺 **Crash diagnostics** — prints the last 40 lines of server output when the server exits unexpectedly

---

## 🚀 Installation

### Quick Install (no Rust required)

**macOS/Linux:**
```bash
curl -sSL https://raw.githubusercontent.com/Mosuzzzz/mc-cli/master/install.sh | bash
```

**Windows (PowerShell):**
```powershell
irm https://raw.githubusercontent.com/Mosuzzzz/mc-cli/master/install.ps1 | iex
```

The install script downloads the pre-built binary for your platform and verifies its SHA-256 checksum before installing.

### Install from source

Requires [Rust & Cargo](https://rustup.rs/) (edition 2024).

```bash
cargo install --git https://github.com/Mosuzzzz/mc-cli.git --locked
```

### Prerequisites

| Requirement | Minimum Version |
|---|---|
| Java | 8+ (17+ for MC 1.17+, 21+ for MC 1.21+) |

### Self-update

```bash
mc-cli update
```

---

## 💻 Usage

### Start a server

```bash
# Start with a specific version (downloads automatically on first run)
mc-cli start --version 1.21.1

# Start in a specific directory
mc-cli start /path/to/server --version 1.21.1

# Resume an already downloaded server (no --version needed)
mc-cli start .

# Use a different provider or allocate more RAM
mc-cli start --version 1.21.1 --provider fabric --ram 4G
mc-cli start --version 1.20.4 --provider vanilla --ram 1G

# Require premium accounts (online mode)
mc-cli start --version 1.21.1 --online
```

On **first launch**, mc-cli will:
1. Download the server jar and verify its checksum
2. Accept the EULA automatically
3. Generate `server.properties` (offline mode by default; use `--online` to change)
4. Start the server, wait for it to finish initializing, then **auto-restart** so clients can connect

### TUI Controls

| Key / Input | Action |
|---|---|
| **Ctrl+C** | Gracefully stop the server and exit |
| **Type a command + Enter** | Send command to the server console |
| **Type `stop` + Enter** | Stop the server |
| **Type `restart` + Enter** | Restart the server without exiting mc-cli |

### List available versions

```bash
mc-cli list-versions                    # Paper (default)
mc-cli list-versions --provider fabric
mc-cli list-versions --provider vanilla
```

Output shows the latest version at the top, marked with `★ latest`.

### Update mc-cli

```bash
mc-cli update
```

Fetches the latest release tag from GitHub, downloads the pre-built binary for your platform, verifies its SHA-256 checksum, then atomically replaces the running binary. On Windows, the swap happens via a background script after mc-cli exits.

Falls back to `cargo install --locked` if no pre-built binary is available for your platform.

### Uninstall

```bash
mc-cli uninstall
```

Or use the provided scripts:
- **macOS/Linux**: `./uninstall.sh`
- **Windows**: `.\uninstall.ps1`

---

## 📁 Directory Structure

After running `mc-cli start`, your directory will look like:

```
my-server/
└── server/
    ├── paper-1.21.1.jar       ← downloaded & cached
    ├── eula.txt               ← auto-accepted
    ├── server.properties      ← online-mode=false (or true with --online)
    ├── world/
    └── ...
```

All server files live in a `server/` subfolder inside the directory you point mc-cli at.

---

## ⚙️ CLI Reference

```
mc-cli <COMMAND>

Commands:
  start          Start a Minecraft server
  list-versions  List available versions for a provider
  update         Update mc-cli to the latest release
  uninstall      Uninstall mc-cli from the system
  help           Print help

start [DIR] [OPTIONS]
  [DIR]                Target directory (default: .)
  -v, --version        Server version to download/use
  -r, --ram            RAM to allocate, e.g. 2G or 512M (default: 2G)
  -p, --provider       Provider: paper | vanilla | fabric (default: paper)
  --online             Enable online mode (requires premium Minecraft accounts)

list-versions [OPTIONS]
  -p, --provider       Provider to query (default: paper)

update                 Download and install the latest mc-cli release
uninstall              Remove mc-cli binary from the system
```

---

## 🛠️ Built With

| Crate | Purpose |
|---|---|
| `tokio` | Async runtime, child process I/O |
| `ratatui` + `crossterm` | Terminal UI |
| `reqwest` + `serde` | HTTP API calls & JSON parsing |
| `sysinfo` | Live CPU/RAM monitoring |
| `sha1` / `sha2` | Download checksum verification |
| `clap` | CLI argument parsing |
| `anyhow` | Error handling |

---

## 🤝 Contributing

Contributions are welcome! Please keep in mind:

1. **Performance over bloat** — non-network operations should complete in `<100ms`
2. **Graceful shutdowns** — always send `stop` to stdin before killing the process; never SIGKILL
3. **Cross-platform** — test against Windows, macOS, and Linux before submitting PRs
