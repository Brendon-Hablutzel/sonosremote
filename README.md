# A command line interface for controlling sonos speakers, written in Rust


## TODO:
- parse error codes returned by speaker
    - reference found here: https://svrooij.io/sonos-api-docs/services/
- create more robust search for the 'UDN' tag when initializing speaker
    - also add parsing for speaker name and zone
- improve handling of XMLErrors while building XML
- improve parsing of queue items
- improve retrieval of action names (don't allocate a string every time?)