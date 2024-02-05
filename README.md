# A command line interface for controlling Sonos speakers, written in Rust

Commands:
- `discover <search_secs>` finds speakers on the local network
- `interactive <ip_addr>` enters an interactive shell for sending commands to a speaker with the given IP address
- `change-volume <ip_addr> <interval_seconds> <volume_change>` changes the volume of the speaker at the given IP address by `volume_change` every `interval_seconds` seconds
- `get-info <ip_addr>` fetches basic information about a speaker at the given IP address

Relies on the [Rusty Sonos](https://github.com/Brendon-Hablutzel/rusty-sonos) library.
