use clap::{Parser, Subcommand};
use sonosremote::{connect, discover_devices, get_info, gradually_change_volume};

#[derive(Parser)]
#[command()]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// connect to a speaker, given an IP address
    Connect { ip_addr: String },
    /// change the volume in intervals of a given number of seconds
    ChangeVolume {
        ip_addr: String,
        interval_seconds: u64,
        volume_change: i32,
    },
    /// discover sonos devices on the current network
    Discover,
    /// get info about a specific speaker, given its IP address
    GetInfo { ip_addr: String },
}

#[tokio::main]
async fn main() -> Result<(), String> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Connect { ip_addr } => connect(&ip_addr).await?,
        Commands::ChangeVolume {
            ip_addr,
            interval_seconds,
            volume_change,
        } => gradually_change_volume(ip_addr, *interval_seconds, *volume_change).await?,
        Commands::Discover => discover_devices().await?,
        Commands::GetInfo { ip_addr } => {
            println!("{}", get_info(&ip_addr).await?);
        }
    }

    Ok(())
}
