use clap::{Parser, Subcommand};
use rusty_sonos::discovery::{discover_devices, get_speaker_info};
use sonosremote::{connect, discover, gradually_change_volume, show_speaker_info};

#[derive(Parser)]
#[command()]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// connect to a speaker, given an IP address
    Interactive { ip_addr: String },
    /// change the volume in intervals of a given number of seconds
    ChangeVolume {
        ip_addr: String,
        interval_seconds: u64,
        volume_change: i32,
    },
    /// discover sonos devices on the current network
    Discover { search_secs: u64 },
    /// get info about a specific speaker, given its IP address
    GetInfo { ip_addr: String },
}

#[tokio::main]
async fn main() -> Result<(), String> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Interactive { ip_addr } => connect(&ip_addr).await?,
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
