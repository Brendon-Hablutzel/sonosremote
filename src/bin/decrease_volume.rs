use sonosremote::{actions, speaker};
use std::env;
use std::thread::sleep;
use std::time::{Duration, SystemTime};

#[tokio::main]
async fn main() -> Result<(), String> {
    let mut args = env::args();

    let ip = args.nth(1).ok_or("Must provide an IP address")?;

    let wait_seconds = args
        .next()
        .ok_or("Must provide a number of seconds to sleep between volume changes")?;
    let wait_time = Duration::new(
        wait_seconds
            .parse::<u64>()
            .map_err(|_| "Sleep duration must be a positive integer")?,
        0,
    );

    let volume_change = args
        .next()
        .ok_or("Must provide an amount by which the volume changes per iteration")?
        .parse::<i32>()
        .map_err(|_| "Volume change must be an integer".to_owned())?;
    if volume_change.abs() > 100 {
        return Err("Volume change must be less than or equal to 100".to_owned());
    }

    let speaker = speaker::Speaker::new(&ip)
        .await
        .map_err(|err| format!("Error initializing speaker: {err}"))?;

    println!(
        "Starting loop to change volume by {volume_change} every {} seconds",
        wait_time.as_secs_f32()
    );

    let initial_time = SystemTime::now();

    loop {
        let volume = speaker.cmd(actions::GetVolume).await?;
        if volume == 0 {
            println!("Volume has reached 0, exiting...");
            break;
        }

        let new_volume = (volume as i32 + volume_change) as u8;

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

        println!("Waiting for {} second(s)", wait_time.as_secs_f32());

        println!("-----");

        sleep(wait_time);
    }

    Ok(())
}
