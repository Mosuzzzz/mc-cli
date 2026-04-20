# ⛏️ mc-cli

> A blazing-fast, terminal-native Minecraft server manager written in Rust.

`mc-cli` automates the full lifecycle of a Minecraft server — from downloading the right jar to running a real-time TUI dashboard — with a single command.

---

## ✨ Features

- 🚀 **One-command launch** — download, configure, and start a server in seconds
- 🖥️ **Interactive TUI dashboard** — real-time console, CPU/RAM usage, and player count
- 🔄 **Auto-restart on first boot** — automatically restarts after initial world generation so clients can connect immediately
- 📦 **Multi-provider support** — Paper, Vanilla, and Fabric
- ✅ **Checksum verification** — SHA-1/SHA-256 validated downloads
- 🔓 **Offline mode by default** — `online-mode=false` pre-configured so both premium and non-premium clients can join
- ♻️ **In-TUI restart** — type `restart` in the console bar without stopping mc-cli
- 🔁 **Self-updating** — `mc-cli update` fetches and installs the latest version from GitHub
- ☕ **Java version guard** — warns you if your Java is too old for the requested server version
- 🩺 **Crash diagnostics** — prints the last 40 lines of server output when the server exits unexpectedly

---

## 🚀 Installation

### Prerequisites

| Requirement | Minimum Version |
|---|---|
| [Rust & Cargo](https://rustup.rs/) | Edition 2024 (stable) |
| Java | 8+ (21+ for MC 1.21+) |

### Quick Install

**macOS/Linux:**
```bash
curl -sSL https://raw.githubusercontent.com/Mosuzzzz/mc-cli/master/install.sh | bash
```

**Windows (PowerShell):**
```powershell
irm https://raw.githubusercontent.com/Mosuzzzz/mc-cli/master/install.ps1 | iex
```

### Install from source

```bash
git clone https://github.com/Mosuzzzz/mc-cli.git
cd mc-cli
cargo install --path .
```

### Self-update (after first install)

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
mc-cli start . --version 1.21.1

# Resume an already downloaded server (no --version needed)
mc-cli start .

# Use a different provider or allocate more RAM
mc-cli start --version 1.21.1 --provider fabric --ram 4G
mc-cli start --version 1.20.4 --provider vanilla --ram 1G
```

On **first launch**, mc-cli will:
1. Download the server jar and verify its checksum
2. Accept the EULA automatically
3. Generate `server.properties` with `online-mode=false`
4. Start the server, wait for it to finish initializing, then **auto-restart** it so clients can connect properly

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

### Update the server version

```bash
# Downloads the new jar, removes the old one
mc-cli update --version 1.21.4
mc-cli update --version 1.21.4 --provider paper

# Provider is auto-detected from your existing jar if omitted
```

### Update mc-cli itself

```bash
mc-cli update
```

This builds the latest binary from GitHub and swaps it in. On Windows, it uses a background task to replace the exe after mc-cli exits (avoiding "Access Denied" errors). On Linux/macOS, the update is applied immediately.

### Uninstall

To remove `mc-cli` from your system:

```bash
mc-cli uninstall
```

Alternatively, use the provided scripts if you installed via a script:
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
    ├── server.properties      ← online-mode=false
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
  update         Update the server jar or mc-cli itself
  uninstall      Uninstall mc-cli from the system
  help           Print help

start [DIR] [OPTIONS]
  [DIR]                Target directory (default: .)
  -v, --version        Server version to download/use
  -r, --ram            RAM to allocate (default: 2G)
  -p, --provider       Provider: paper | vanilla | fabric (default: paper)

list-versions [OPTIONS]
  -p, --provider       Provider to query (default: paper)

update [DIR] [OPTIONS]
  [DIR]                Target directory (default: .)
  -v, --version        New server version to download
  -p, --provider       Provider (auto-detected if omitted)
  (no args)            Update mc-cli itself from GitHub
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

---

