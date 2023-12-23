use anyhow::bail;
use rusty_sonos::{
    discovery::{discover_devices, get_speaker_info},
    speaker::Speaker,
};
use std::{
    net::Ipv4Addr,
    str::FromStr,
    thread::sleep,
    time::{Duration, SystemTime},
};

const INTERACTIVE_HELP_MESSAGE: &str = "COMMANDS:
play
pause
seek <hours:minutes:seconds>
current (prints information about current track)
seturi <URI> (plays audio from the specified URI)
setvolume <newvolume>
getvolume
status (whether the playback is stopped)
next
previous
endcontrol (ends the control of any other services communicating with the speaker)
info (prints info about the speaker)
addtoqueue <URI> (adds audio from specified URI to queue)
clearqueue (clears the queue)
help (displays this menu)";

async fn show_queue(speaker: &Speaker) -> Result<String, anyhow::Error> {
    let queue = speaker.get_queue().await?;

    let mut queue_string = String::from("Queue:\n");

    for track in queue {
        let track_string = format!(
            "{} by {}\nURI: {}\nDuration: {}",
            track.title.unwrap_or(String::from("None")),
            track.artist.unwrap_or(String::from("None")),
            track.uri,
            track.duration.unwrap_or(String::from("N/A"))
        );
        queue_string.push_str(&track_string);
    }

    Ok(queue_string)
}

async fn show_current_track(speaker: &Speaker) -> Result<String, anyhow::Error> {
    let current_track = speaker.get_current_track().await?;

    Ok(format!(
        "{} by {}\nURI: {}\nPosition: {} / {}",
        current_track.title.unwrap_or(String::from("None")),
        current_track.artist.unwrap_or(String::from("None")),
        current_track.uri,
        current_track.position,
        current_track.duration
    ))
}

async fn show_current_volume(speaker: &Speaker) -> Result<String, anyhow::Error> {
    let current_volume = speaker.get_volume().await?;

    Ok(current_volume.to_string())
}

async fn show_current_status(speaker: &Speaker) -> Result<String, anyhow::Error> {
    let current_status = speaker.get_playback_status().await?;

    Ok(current_status.playback_state.to_string())
}

async fn process_command(speaker: &Speaker, command: Vec<&str>) -> Result<String, anyhow::Error> {
    let output: String = match command[0] {
        "play" => speaker
            .play()
            .await
            .map(|_| String::from("Playing current track"))?,
        "pause" => speaker
            .play()
            .await
            .map(|_| String::from("Pausing current track"))?,
        "queue" => show_queue(speaker).await?,
        "current" => show_current_track(speaker).await?,
        "seturi" => match command.get(1) {
            None => bail!("must enter a URI"),
            Some(uri) => (speaker)
                .set_current_uri(uri)
                .await
                .map(|_| format!("Playing from URI: {}", uri))?,
        },
        "setvolume" => match command.get(1) {
            None => bail!("must provide volume"),
            Some(volume) => match volume.parse::<u8>() {
                Ok(volume) => speaker
                    .set_volume(volume)
                    .await
                    .map(|_| format!("Setting volume to {}", volume))?,
                Err(_) => bail!("invalid volume"),
            },
        },
        "getvolume" => show_current_volume(speaker).await?,
        "status" => show_current_status(speaker).await?,
        "seek" => match command.get(1) {
            None => bail!("must enter a target time"),
            Some(target_time) => speaker
                .seek(target_time)
                .await
                .map(|_| format!("Moving to position {}", target_time))?,
        },
        "next" => speaker
            .move_to_next_track()
            .await
            .map(|_| String::from("Moving to next track"))?,
        "previous" => speaker
            .move_to_previous_track()
            .await
            .map(|_| String::from("Moving to next track"))?,
        "endcontrol" => (speaker)
            .end_external_control()
            .await
            .map(|_| String::from("Ending external control of speaker"))?,
        "enterqueue" => speaker
            .enter_queue()
            .await
            .map(|_| String::from("Entering the queue"))?,
        "info" => format!("{}, {}", speaker.get_friendly_name(), speaker.get_uuid()),
        "addtoqueue" => match command.get(1) {
            None => bail!("must enter a URI"),
            Some(uri) => speaker
                .add_track_to_queue(uri)
                .await
                .map(|_| format!("Adding track from {} to queue", uri))?,
        },
        "clearqueue" => speaker
            .clear_queue()
            .await
            .map(|_| String::from("Clearing queue"))?,
        "help" => String::from(INTERACTIVE_HELP_MESSAGE),
        _ => bail!("invalid command"),
    };

    Ok(output)
}

pub async fn interactive(ip_addr: &str) -> Result<(), anyhow::Error> {
    let speaker = Speaker::new(Ipv4Addr::from_str(ip_addr)?).await?;

    loop {
        println!(">");

        let mut input = String::new();

        if let Err(x) = std::io::stdin().read_line(&mut input) {
            eprintln!("Error reading input: {}", x);
            continue;
        }

        let input: Vec<&str> = input.trim().split(" ").collect();

        // indexing is ok because .split() always returns at least one element
        let output = process_command(&speaker, input).await;

        match output {
            Ok(x) => println!("{x}\n"),
            Err(e) => eprintln!("{e}\n"),
        }
    }
}

pub async fn gradually_change_volume(
    ip_addr: &str,
    interval_seconds: u64,
    volume_change: i32,
) -> Result<(), anyhow::Error> {
    let volume_change_interval = Duration::new(interval_seconds, 0);

    if volume_change.abs() > 100 {
        bail!("Change in volume must be between -100 and 100")
    }

    let speaker = Speaker::new(Ipv4Addr::from_str(ip_addr)?).await?;

    println!(
        "Starting loop to change volume by {volume_change} every {s} seconds",
        s = volume_change_interval.as_secs_f32()
    );

    let initial_time = SystemTime::now();

    loop {
        let volume: i32 = speaker.get_volume().await?.into();
        if volume + volume_change < 0 || volume + volume_change > 100 {
            println!("Volume has reached {volume}, exiting...");
            break;
        }

        println!(
            "Waiting for {} second(s)",
            volume_change_interval.as_secs_f32()
        );

        sleep(volume_change_interval);

        println!("-----");

        let new_volume = (volume + volume_change) as u8;

        let now = SystemTime::now();
        let time_elapsed = now.duration_since(initial_time)?;

        println!(
            "Program has been running for {} seconds",
            time_elapsed.as_secs_f32()
        );

        speaker.set_volume(new_volume).await?;

        println!("Changed volume from {volume} to {new_volume}");
    }

    Ok(())
}

pub async fn discover(search_secs: u64) -> Result<(), anyhow::Error> {
    let devices = discover_devices(search_secs, 5).await?;
    let num_devices = devices.len();

    if num_devices == 0 {
        println!("No devices found");
    } else {
        println!("{num_devices} devices found in {search_secs} seconds:");
        for device in devices {
            println!("{}, {}", device.friendly_name, device.room_name);
        }
    }

    Ok(())
}

pub async fn show_speaker_info(ip_addr: &str) -> Result<(), anyhow::Error> {
    let info = get_speaker_info(Ipv4Addr::from_str(ip_addr)?).await?;

    println!("Speaker found:");
    println!("{}, {}", info.friendly_name, info.room_name);

    Ok(())
}
