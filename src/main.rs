use sonosremote::{
    actions,
    speaker::{self, action_then_current},
};
use std::env;

#[tokio::main]
async fn main() -> Result<(), String> {
    let mut args = env::args();
    args.next(); // skip first argument

    let ip = args.next().ok_or("Must provide an IP address")?;

    let speaker = speaker::Speaker::new(&ip)
        .await
        .map_err(|err| format!("Error initializing speaker: {err}"))?;

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
            _ => Err("Invalid option".to_owned()),
        };

        match output {
            Ok(x) => println!("{x}\n"),
            Err(e) => eprintln!("{e}\n"),
        }
    }
}
