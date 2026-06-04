use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "mc-cli")]
#[command(about = "Open Source Minecraft Server Manager", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Start a minecraft server
    Start {
        /// The target directory to start the server in
        #[arg(default_value = ".")]
        dir: String,

        /// The minecraft version to run
        #[arg(short, long)]
        version: Option<String>,

        /// The amount of RAM to allocate (e.g., 4G)
        #[arg(short, long, default_value = "2G")]
        ram: String,

        /// Server provider (e.g., paper, vanilla, fabric)
        #[arg(short, long, default_value = "paper")]
        provider: String,

        /// Enable online mode (requires premium Minecraft accounts). Default is offline mode.
        #[arg(long, default_value = "false")]
        online: bool,
    },
    /// List available versions for a provider
    ListVersions {
        /// Server provider (e.g., paper, vanilla, fabric)
        #[arg(short, long, default_value = "paper")]
        provider: String,
    },
    /// Update mc-cli to the latest version
    Update,
    /// Uninstall mc-cli from the system
    Uninstall,
}
