use crate::speaker::action_then_current;
use std::{env, process};
mod actions;
mod parse_utils;
mod speaker;

#[tokio::main]
async fn main() {
    let mut args = env::args();
    args.next(); // skip first argument

    let ip = args.next().unwrap_or_else(|| {
        eprintln!("Must provide an IP address");
        process::exit(1);
    });

    let speaker = speaker::Speaker::new(&ip).await.unwrap_or_else(|err| {
        eprintln!("Error creating speaker: {}", err);
        process::exit(1);
    });

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
            "current" => speaker.cmd(actions::GetCurrentTrackInfo).await,
            "seturi" => match input.get(1) {
                None => Err("Must enter a URI".to_owned()),
                Some(uri) => speaker.cmd(actions::SetURI::new(uri.to_string())).await,
            },
            "setvolume" => match input.get(1) {
                None => Err("Invalid volume".to_owned()),
                Some(volume) => match actions::SetVolume::new(volume.to_string()) {
                    Ok(action) => speaker.cmd(action).await,
                    Err(_) => Err("Invalid volume".to_owned()),
                },
            },
            "getvolume" => speaker.cmd(actions::GetVolume).await,
            "status" => speaker.cmd(actions::GetStatus).await,
            "seek" => match input.get(1) {
                None => Err("Must enter a target time".to_owned()),
                Some(target_time) => {
                    speaker
                        .cmd(actions::Seek::new(target_time.to_string()))
                        .await
                }
            },
            "next" => action_then_current(&speaker, actions::Next).await,
            "previous" => action_then_current(&speaker, actions::Previous).await,
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
