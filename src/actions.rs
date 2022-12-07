use crate::parse_utils::{
    get_error_code, parse_current, parse_getvolume, parse_queue, parse_status,
};
use crate::services::{AVTransport, ContentDirectory, RenderingControl, Service};
use reqwest::{self, StatusCode};
use std::collections::HashMap;
use std::str;
// NOTE: async fns in traits was recently added to nightly build
use async_trait::async_trait;

async fn get_res_text(res: reqwest::Response) -> Result<String, String> {
    Ok(res
        .text()
        .await
        .map_err(|err| format!("Error getting response body: {err}"))?)
}

#[async_trait]
pub trait Action {
    fn get_service(&self) -> Box<dyn Service>;

    fn get_action_name(&self) -> &'static str;

    fn get_args_map(&self) -> HashMap<&str, &str> {
        let mut map = HashMap::new();
        map.insert("InstanceID", "0");
        map
    }

    fn handle_sonos_err_code(&self, _err_code: &str) -> Option<String> {
        None
    }

    async fn handle_successful_response(&self, _res: reqwest::Response) -> Result<String, String> {
        Ok("Success".to_owned())
    }

    async fn res_to_output(
        &self,
        res: Result<reqwest::Response, reqwest::Error>,
    ) -> Result<String, String> {
        let res = res.map_err(|err| format!("Error sending request: {err}"))?;
        let status = res.status();

        match status {
            StatusCode::OK => self.handle_successful_response(res).await,
            status_code => {
                let body = get_res_text(res).await?;
                match get_error_code(body, self) {
                    Ok(sonos_err_code) => {
                        let details = self.handle_sonos_err_code(&sonos_err_code).unwrap_or(format!("Sonos error code: {sonos_err_code}"));
                        Err(format!("Speaker responded with {status_code}\n{details}"))
                    },
                    Err(err) => Err(format!("Speaker responded with {status_code}:\nA more specific error code could not be found: {err}"))
                }
            }
        }
    }
}

pub struct Play;

#[async_trait]
impl Action for Play {
    fn get_service(&self) -> Box<dyn Service> {
        Box::new(AVTransport)
    }

    fn get_action_name(&self) -> &'static str {
        "Play"
    }

    fn get_args_map(&self) -> HashMap<&str, &str> {
        let mut map = HashMap::new();
        map.insert("InstanceID", "0");
        map.insert("Speed", "1");
        map
    }

    fn handle_sonos_err_code(&self, err_code: &str) -> Option<String> {
        match err_code {
            "701" => Some("Action currently unavailable. Ensure there is a track selected and that it is not currently playing.".to_owned()),
            _ => None
        }
    }
}

pub struct Pause;

#[async_trait]
impl Action for Pause {
    fn get_service(&self) -> Box<dyn Service> {
        Box::new(AVTransport)
    }

    fn get_action_name(&self) -> &'static str {
        "Pause"
    }

    fn handle_sonos_err_code(&self, err_code: &str) -> Option<String> {
        match err_code {
            "701" => Some("Action currently unavailable. Ensure there is a track selected and that it is currently playing.".to_owned()),
            _ => None
        }
    }
}

pub struct GetQueue;

#[async_trait]
impl Action for GetQueue {
    fn get_service(&self) -> Box<dyn Service> {
        Box::new(ContentDirectory)
    }

    fn get_action_name(&self) -> &'static str {
        "Browse"
    }

    fn get_args_map(&self) -> HashMap<&str, &str> {
        let mut map = HashMap::new();
        map.insert("ObjectID", "Q:0");
        map.insert("BrowseFlag", "BrowseDirectChildren");
        map.insert("Filter", "*");
        map.insert("StartingIndex", "0");
        map.insert("RequestedCount", "100");
        map.insert("SortCriteria", "");
        map
    }

    async fn handle_successful_response(&self, res: reqwest::Response) -> Result<String, String> {
        let xml = get_res_text(res).await?;

        let mut output = String::new();
        output.push_str("Queue:");

        let queue_items = parse_queue(xml, self)?;

        if queue_items.len() == 0 {
            return Ok("No tracks found in queue".to_owned());
        }

        let mut index = 1;

        for item in queue_items {
            output.push_str(&format!("\n-----\n{index}: {item}"));
            index += 1;
        }

        Ok(output)
    }
}

pub struct GetCurrentTrackInfo;

#[async_trait]
impl Action for GetCurrentTrackInfo {
    fn get_service(&self) -> Box<dyn Service> {
        Box::new(AVTransport)
    }

    fn get_action_name(&self) -> &'static str {
        "GetPositionInfo"
    }

    async fn handle_successful_response(&self, res: reqwest::Response) -> Result<String, String> {
        let xml = get_res_text(res).await?;

        let output = parse_current(xml, self)?;

        Ok(format!("Current track:\n{output}"))
    }
}

pub struct SetURI {
    uri: String,
}

impl SetURI {
    pub fn new(uri: String) -> Self {
        SetURI { uri }
    }
}

#[async_trait]
impl Action for SetURI {
    fn get_service(&self) -> Box<dyn Service> {
        Box::new(AVTransport)
    }

    fn get_action_name(&self) -> &'static str {
        "SetAVTransportURI"
    }

    fn get_args_map(&self) -> HashMap<&str, &str> {
        let mut map = HashMap::new();
        map.insert("InstanceID", "0");
        map.insert("CurrentURI", &self.uri);
        map.insert("CurrentURIMetaData", "");
        map
    }
}

pub struct SetVolume {
    desired_volume: String,
}

impl SetVolume {
    pub fn new(new_volume: String) -> Result<Self, String> {
        // could perhaps refactor this to use a match expression
        let parsed_volume: u8 = new_volume
            .parse()
            .map_err(|_| "Error parsing volume".to_owned())?;
        if parsed_volume > 100 {
            return Err("Volume out of range".to_owned());
        };
        Ok(SetVolume {
            desired_volume: new_volume,
        })
    }
}

#[async_trait]
impl Action for SetVolume {
    fn get_service(&self) -> Box<dyn Service> {
        Box::new(RenderingControl)
    }

    fn get_action_name(&self) -> &'static str {
        "SetVolume"
    }

    fn get_args_map(&self) -> HashMap<&str, &str> {
        let mut map = HashMap::new();
        map.insert("InstanceID", "0");
        map.insert("Channel", "Master");
        map.insert("DesiredVolume", &self.desired_volume);
        map
    }
}

pub struct GetVolume;

#[async_trait]
impl Action for GetVolume {
    fn get_service(&self) -> Box<dyn Service> {
        Box::new(RenderingControl)
    }

    fn get_action_name(&self) -> &'static str {
        "GetVolume"
    }

    fn get_args_map(&self) -> HashMap<&str, &str> {
        let mut map = HashMap::new();
        map.insert("InstanceID", "0");
        map.insert("Channel", "Master");
        map
    }

    async fn handle_successful_response(&self, res: reqwest::Response) -> Result<String, String> {
        let xml = get_res_text(res).await?;

        let volume = parse_getvolume(xml, self)?;

        Ok(format!("Current volume: {volume}"))
    }
}

pub struct GetStatus;

#[async_trait]
impl Action for GetStatus {
    fn get_service(&self) -> Box<dyn Service> {
        Box::new(AVTransport)
    }

    fn get_action_name(&self) -> &'static str {
        "GetTransportInfo"
    }

    async fn handle_successful_response(&self, res: reqwest::Response) -> Result<String, String> {
        let xml = get_res_text(res).await?;

        let data = parse_status(xml, self)?;

        Ok(format!("Current status:\n{data}"))
    }
}

pub struct Seek {
    target: String,
}

impl Seek {
    pub fn new(target_time: String) -> Self {
        // might wanna validate that time
        Seek {
            target: target_time,
        }
    }
}

#[async_trait]
impl Action for Seek {
    fn get_service(&self) -> Box<dyn Service> {
        Box::new(AVTransport)
    }

    fn get_action_name(&self) -> &'static str {
        "Seek"
    }

    fn get_args_map(&self) -> HashMap<&str, &str> {
        let mut map = HashMap::new();
        map.insert("InstanceID", "0");
        map.insert("Unit", "REL_TIME");
        map.insert("Target", &self.target);
        map
    }
}

pub struct Next;

#[async_trait]
impl Action for Next {
    fn get_service(&self) -> Box<dyn Service> {
        Box::new(AVTransport)
    }

    fn get_action_name(&self) -> &'static str {
        "Next"
    }

    fn handle_sonos_err_code(&self, err_code: &str) -> Option<String> {
        match err_code {
            "711" => Some("Could not find next track. Ensure that you are in the queue and that there are tracks after the current one.".to_owned()),
            _ => None,
        }
    }
}

pub struct Previous;

#[async_trait]
impl Action for Previous {
    fn get_service(&self) -> Box<dyn Service> {
        Box::new(AVTransport)
    }

    fn get_action_name(&self) -> &'static str {
        "Previous"
    }

    fn handle_sonos_err_code(&self, err_code: &str) -> Option<String> {
        match err_code {
            "711" => Some("Could not find previous track. Ensure that you are in the queue and that there are tracks before the current one.".to_owned()),
            _ => None,
        }
    }
}

pub struct EndDirectControlSession;

#[async_trait]
impl Action for EndDirectControlSession {
    fn get_service(&self) -> Box<dyn Service> {
        Box::new(AVTransport)
    }

    fn get_action_name(&self) -> &'static str {
        "EndDirectControlSession"
    }
}

pub struct AddURIToQueue {
    uri: String,
}

impl AddURIToQueue {
    pub fn new(uri: String) -> Self {
        AddURIToQueue { uri }
    }
}

#[async_trait]
impl Action for AddURIToQueue {
    fn get_service(&self) -> Box<dyn Service> {
        Box::new(AVTransport)
    }

    fn get_action_name(&self) -> &'static str {
        "AddURIToQueue"
    }

    fn get_args_map(&self) -> HashMap<&str, &str> {
        let mut map = HashMap::new();
        map.insert("InstanceID", "0");
        map.insert("EnqueuedURI", &self.uri);
        map.insert("EnqueuedURIMetaData", "");
        map.insert("DesiredFirstTrackNumberEnqueued", "0");
        map.insert("EnqueueAsNext", "0");
        map
    }
}

pub struct ClearQueue;

#[async_trait]
impl Action for ClearQueue {
    fn get_service(&self) -> Box<dyn Service> {
        Box::new(AVTransport)
    }

    fn get_action_name(&self) -> &'static str {
        "RemoveAllTracksFromQueue"
    }
}
