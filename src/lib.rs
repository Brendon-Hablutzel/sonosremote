use crate::speaker::action_then_current;
use parse_utils::{get_tag_by_name, get_text};
use std::net::UdpSocket;
use std::thread::sleep;
use std::time::{Duration, SystemTime};

pub mod actions;
pub mod parse_utils;
pub mod services;
pub mod speaker;

pub async fn get_info(ip_addr: &str) -> Result<String, String> {
    let url = format!("http://{ip_addr}:1400/xml/device_description.xml");
    let res = reqwest::get(&url)
        .await
        .map_err(|err| err.to_string())?
        .text()
        .await
        .map_err(|err| err.to_string())?;

    let parsed_xml = roxmltree::Document::parse(&res).map_err(|err| err.to_string())?;

    let friendly_name =
        get_tag_by_name(&parsed_xml, "friendlyName").map_err(|err| err.to_string())?;
    let room_name = get_tag_by_name(&parsed_xml, "roomName").map_err(|err| err.to_string())?;

    Ok(format!(
        "{} in {}",
        get_text(friendly_name, "No friendly name found")?,
        get_text(room_name, "No room name found")?
    ))
}

pub async fn discover_devices() -> Result<(), String> {
    let socket: UdpSocket =
        UdpSocket::bind("0.0.0.0:0").expect("Should be able to create a UDP socket");

    let body = "M-SEARCH * HTTP/1.1
HOST: 239.255.255.250:1900
MAN: ssdp:discover
MX: 1
ST: urn:schemas-upnp-org:device:ZonePlayer:1";

    socket
        .set_broadcast(true)
        .expect("Should be able to enable broadcast");

    socket
        .send_to(body.as_bytes(), "239.255.255.250:1900")
        .map_err(|err| err.to_string())?;

    socket
        .send_to(body.as_bytes(), "255.255.255.255:1900")
        .map_err(|err| err.to_string())?;

    let mut buf = [0; 1024];
    while let Ok((_, addr)) = socket.recv_from(&mut buf) {
        let addr = addr.to_string().replace(&format!(":{}", addr.port()), "");
        if let Ok(info) = get_info(&addr).await {
            println!("{info}");
        } else {
            eprintln!("Error fetching data for device at address {addr}");
        }
    }
    Ok(())
}

pub async fn connect(ip_addr: &str) -> Result<(), String> {
    let speaker = speaker::Speaker::new(&ip_addr)
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
            "play" => speaker.cmd(actions::Play).await,
            "pause" => speaker.cmd(actions::Pause).await,
            "queue" => speaker.cmd(actions::GetQueue).await,
            "current" => speaker
                .cmd(actions::GetCurrentTrackInfo)
                .await
                .map(|track_data| format!("{track_data}")),
            "seturi" => match input.get(1) {
                None => Err("Must enter a URI".to_owned()),
                Some(uri) => speaker.cmd(actions::SetURI::new(uri.to_string())).await,
            },
            "setvolume" => match input.get(1) {
                None => Err("Must provide volume".to_owned()),
                Some(str_volume) => match str_volume.parse::<u8>() {
                    Ok(volume) => match actions::SetVolume::new(volume) {
                        Ok(action) => speaker.cmd(action).await,
                        Err(_) => Err("Invalid volume".to_owned()),
                    },
                    Err(_) => Err("Invalid volume".to_owned()),
                },
            },
            "getvolume" => speaker
                .cmd(actions::GetVolume)
                .await
                .map(|vol| format!("Current volume: {vol}")),
            "status" => speaker
                .cmd(actions::GetStatus)
                .await
                .map(|status| format!("{status}")),
            "seek" => match input.get(1) {
                None => Err("Must enter a target time".to_owned()),
                Some(target_time) => {
                    speaker
                        .cmd(actions::Seek::new(target_time.to_string()))
                        .await
                }
            },
            "next" => action_then_current(&speaker, actions::Next)
                .await
                .map(|track_data| format!("{track_data}")),
            "previous" => action_then_current(&speaker, actions::Previous)
                .await
                .map(|track_data| format!("{track_data}")),
            "endcontrol" => speaker.cmd(actions::EndDirectControlSession).await,
            "enterqueue" => speaker::enter_queue(&speaker).await,
            "info" => Ok(speaker.get_info()),
            "addtoqueue" => match input.get(1) {
                None => Err("Must enter a URI".to_owned()),
                Some(uri) => {
                    speaker
                        .cmd(actions::AddURIToQueue::new(uri.to_string()))
                        .await
                }
            },
            "clearqueue" => speaker.cmd(actions::ClearQueue).await,
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

    let speaker = speaker::Speaker::new(&ip_addr)
        .await
        .map_err(|err| format!("Error initializing speaker: {err}"))?;

    println!(
        "Starting loop to change volume by {volume_change} every {s} seconds",
        s = volume_change_interval.as_secs_f32()
    );

    let initial_time = SystemTime::now();

    loop {
        let volume: i32 = speaker.cmd(actions::GetVolume).await?.into();
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

        speaker.cmd(actions::SetVolume::new(new_volume)?).await?;

        println!("Changed volume from {volume} to {new_volume}");
    }

    Ok(())
}
