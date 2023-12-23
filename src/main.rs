use clap::{Parser, Subcommand};
use sonosremote::{discover, gradually_change_volume, interactive, show_speaker_info};

#[derive(Parser)]
#[command()]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// connect to a speaker, given an IP address, and enter into an interactive terminal to control it
    Interactive { ip_addr: String },
    /// incrementally change the volume of the given speaker
    ChangeVolume {
        /// the IP address of the speaker
        ip_addr: String,
        /// the number of seconds to wait between changing the volume
        interval_seconds: u64,
        /// how much to change the volume by
        volume_change: i32,
    },
    /// discover Sonos devices on the current network
    Discover {
        /// the number of seconds to search for devices before returning results
        search_secs: u64,
    },
    /// get info about a specific speaker, given its IP address
    GetInfo { ip_addr: String },
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Interactive { ip_addr } => interactive(&ip_addr).await?,
        Commands::ChangeVolume {
            ip_addr,
            interval_seconds,
            volume_change,
        } => gradually_change_volume(ip_addr, *interval_seconds, *volume_change).await?,
        Commands::Discover { search_secs } => discover(*search_secs).await?,
        Commands::GetInfo { ip_addr } => show_speaker_info(ip_addr).await?,
    }

    Ok(())
}
