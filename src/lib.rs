use std::{
    thread::sleep,
    time::{Duration, SystemTime},
};

use rusty_sonos::{
    discovery::{discover_devices, get_speaker_info},
    speaker::Speaker,
};

async fn show_queue(speaker: &Speaker) -> Result<String, String> {
    let queue = speaker.get_queue().await?;

    let mut queue_string = String::from("Queue:\n");

    for track in queue {
        let track_string = format!(
            "{} by {}\nURI: {}\nDuration: {}",
            track.title.unwrap_or("None".to_owned()),
            track.artist.unwrap_or("None".to_owned()),
            track.uri,
            track.duration.unwrap_or("N/A".to_owned())
        );
        queue_string.push_str(&track_string);
    }

    Ok(queue_string)
}

async fn show_current_track(speaker: &Speaker) -> Result<String, String> {
    let current_track = speaker.get_current_track().await?;

    Ok(format!(
        "{} by {}\nURI: {}\nPosition: {} / {}",
        current_track.title.unwrap_or("None".to_owned()),
        current_track.artist.unwrap_or("None".to_owned()),
        current_track.uri,
        current_track.position,
        current_track.duration
    ))
}

async fn show_current_volume(speaker: &Speaker) -> Result<String, String> {
    let current_volume = speaker.get_volume().await?;

    Ok(current_volume.to_string())
}

async fn show_current_status(speaker: &Speaker) -> Result<String, String> {
    let current_status = speaker.get_playback_status().await?;

    Ok(current_status.playback_state.to_string())
}

pub async fn connect(ip_addr: &str) -> Result<(), String> {
    let speaker = Speaker::new(&ip_addr)
        .await
        .map_err(|err| format!("Error initializing speaker: {err}"))?;

    let help_menu = "COMMANDS:
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

    loop {
        println!(">");

        let mut input = String::new();

        if let Err(x) = std::io::stdin().read_line(&mut input) {
            eprintln!("Error reading input: {}", x);
            continue;
        }

        let input: Vec<&str> = input.trim().split(" ").collect();

        // indexing is ok because .split() always returns at least one element
        let output = match input[0] {
            "play" => (&speaker)
                .play()
                .await
                .map(|_| "Playing current track".to_owned()),
            "pause" => (&speaker)
                .play()
                .await
                .map(|_| "Pausing current track".to_owned()),
            "queue" => show_queue(&speaker).await,
            "current" => show_current_track(&speaker).await,
            "seturi" => match input.get(1) {
                None => Err("Must enter a URI".to_owned()),
                Some(uri) => (&speaker)
                    .set_current_uri(uri)
                    .await
                    .map(|_| format!("Playing from URI: {}", uri)),
            },
            "setvolume" => match input.get(1) {
                None => Err("Must provide volume".to_owned()),
                Some(str_volume) => match str_volume.parse::<u8>() {
                    Ok(volume) => (&speaker)
                        .set_volume(volume)
                        .await
                        .map(|_| "Setting volume to {volume}".to_owned()),
                    Err(_) => Err("Invalid volume".to_owned()),
                },
            },
            "getvolume" => show_current_volume(&speaker).await,
            "status" => show_current_status(&speaker).await,
            "seek" => match input.get(1) {
                None => Err("Must enter a target time".to_owned()),
                Some(target_time) => (&speaker)
                    .seek(target_time)
                    .await
                    .map(|_| format!("Moving to position {}", target_time)),
            },
            "next" => (&speaker)
                .move_to_next_track()
                .await
                .map(|_| "Moving to next track".to_owned()),
            "previous" => (&speaker)
                .move_to_previous_track()
                .await
                .map(|_| "Moving to next track".to_owned()),
            "endcontrol" => (&speaker)
                .end_external_control()
                .await
                .map(|_| "End external control of speaker".to_owned()),
            "enterqueue" => (&speaker)
                .enter_queue()
                .await
                .map(|_| "Entering the queue".to_owned()),
            "info" => Ok(format!(
                "{}, {}",
                &speaker.get_friendly_name(),
                &speaker.get_uid()
            )),
            "addtoqueue" => match input.get(1) {
                None => Err("Must enter a URI".to_owned()),
                Some(uri) => (&speaker)
                    .add_track_to_queue(uri)
                    .await
                    .map(|_| format!("Adding track from {} to queue", uri)),
            },
            "clearqueue" => (&speaker)
                .clear_queue()
                .await
                .map(|_| "Clearing queue".to_owned()),
            "help" => Ok(String::from(help_menu)),
            _ => Err("Invalid option".to_owned()),
        };

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
) -> Result<(), String> {
    let volume_change_interval = Duration::new(interval_seconds, 0);

    if volume_change.abs() > 100 {
        return Err("Volume change magnitude must be less than or equal to 100".to_owned());
    }

    let speaker = Speaker::new(&ip_addr)
        .await
        .map_err(|err| format!("Error initializing speaker: {err}"))?;

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
        let time_elapsed = now
            .duration_since(initial_time)
            .map_err(|err| err.to_string())?;

        println!(
            "Program has been running for {} seconds",
            time_elapsed.as_secs_f32()
        );

        speaker.set_volume(new_volume).await?;

        println!("Changed volume from {volume} to {new_volume}");
    }

    Ok(())
}

pub async fn discover(search_secs: u64) -> Result<(), String> {
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

pub async fn show_speaker_info(ip_addr: &str) -> Result<(), String> {
    let info = get_speaker_info(ip_addr).await?;

    println!("Speaker found:");
    println!("{}, {}", info.friendly_name, info.room_name);

    Ok(())
}
