use crate::parse_utils::{
    get_error_code, parse_current, parse_getvolume, parse_queue, parse_status,
};
use reqwest::{self, StatusCode};
use std::collections::HashMap;
use std::str;
// NOTE: async fns in traits was recently added to nightly build
use async_trait::async_trait;

const CONTENT_DIRECTORY_ENDPOINT: &str = "/MediaServer/ContentDirectory/Control";

const CONTENT_DIRECTORY_SERVICE_NAME: &str = "ContentDirectory:1";

const TRANSPORT_ENDPOINT: &str = "/MediaRenderer/AVTransport/Control";

const TRANSPORT_SERVICE_NAME: &str = "AVTransport:1";

const RENDERING_CONTROL_ENDPOINT: &str = "/MediaRenderer/RenderingControl/Control";

const RENDERING_CONTROL_SERVICE_NAME: &str = "RenderingControl:1";

pub struct Service {
    name: &'static str,
    endpoint: &'static str,
}

impl Service {
    pub fn get_name(&self) -> &'static str {
        self.name
    }

    pub fn get_endpoint(&self) -> &'static str {
        self.endpoint
    }
}

pub enum Services {
    AVTransport,
    ContentDirectory,
    RenderingControl,
}

impl Services {
    pub fn get_data(&self) -> Service {
        let (name, endpoint) = match self {
            Services::AVTransport => (TRANSPORT_SERVICE_NAME, TRANSPORT_ENDPOINT),
            Services::ContentDirectory => {
                (CONTENT_DIRECTORY_SERVICE_NAME, CONTENT_DIRECTORY_ENDPOINT)
            }
            Services::RenderingControl => {
                (RENDERING_CONTROL_SERVICE_NAME, RENDERING_CONTROL_ENDPOINT)
            }
        };
        Service { name, endpoint }
    }
}

#[async_trait]
pub trait Action {
    fn get_service(&self) -> Services;

    fn get_action_name(&self) -> String;

    fn get_args_map(&self) -> HashMap<&str, &str> {
        let mut map = HashMap::new();
        map.insert("InstanceID", "0");
        map
    }

    fn handle_sonos_err_code(&self, _err_code: &str) -> Option<String> {
        None
    }

    async fn res_to_output(
        &self,
        res: Result<reqwest::Response, reqwest::Error>,
    ) -> Result<String, String> {
        let res = res.map_err(|err| format!("Error sending request: {err}"))?;
        let status = res.status();

        match status {
            StatusCode::OK => Ok(format!("Response {}", status)),
            status_code => {
                let body = res
                    .text()
                    .await
                    .map_err(|err| format!("Error getting response body: {err}"))?;
                match get_error_code(body, &self.get_action_name(), self.get_service().get_data().name) {
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
    fn get_service(&self) -> Services {
        Services::AVTransport
    }

    fn get_action_name(&self) -> String {
        String::from("Play")
    }

    fn get_args_map(&self) -> HashMap<&str, &str> {
        let mut map = HashMap::new();
        map.insert("InstanceID", "0");
        map.insert("Speed", "1");
        map
    }
}

pub struct Pause;

#[async_trait]
impl Action for Pause {
    fn get_service(&self) -> Services {
        Services::AVTransport
    }

    fn get_action_name(&self) -> String {
        String::from("Pause")
    }
}

pub struct GetQueue;

#[async_trait]
impl Action for GetQueue {
    fn get_service(&self) -> Services {
        Services::ContentDirectory
    }

    fn get_action_name(&self) -> String {
        String::from("Browse")
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

    async fn res_to_output(
        &self,
        res: Result<reqwest::Response, reqwest::Error>,
    ) -> Result<String, String> {
        let res = res.map_err(|err| format!("Error sending request: {err}"))?;

        let status = res.status();
        let xml = res
            .text()
            .await
            .map_err(|err| format!("Error getting response body: {err}"))?;
        let mut output = String::new();
        output.push_str("Queue:");

        let queue_items = parse_queue(xml)?;
        let mut index = 1;

        for item in queue_items {
            output.push_str(&format!("\n-----\n{index}: {item}"));
            index += 1;
        }

        Ok(format!(
            "Get queue request responded with {}\n{}",
            status, output
        ))
    }
}

pub struct GetCurrentTrackInfo;

#[async_trait]
impl Action for GetCurrentTrackInfo {
    fn get_service(&self) -> Services {
        Services::AVTransport
    }

    fn get_action_name(&self) -> String {
        String::from("GetPositionInfo")
    }

    async fn res_to_output(
        &self,
        res: Result<reqwest::Response, reqwest::Error>,
    ) -> Result<String, String> {
        let res = res.map_err(|err| format!("Error sending request: {err}"))?;

        let status = res.status();
        let xml = res
            .text()
            .await
            .map_err(|err| format!("Error getting response body: {err}"))?;
        let output = parse_current(xml)?;

        Ok(format!(
            "Get current request responded with {}\n{}",
            status, output
        ))
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
    fn get_service(&self) -> Services {
        Services::AVTransport
    }

    fn get_action_name(&self) -> String {
        String::from("SetAVTransportURI")
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
    fn get_service(&self) -> Services {
        Services::RenderingControl
    }

    fn get_action_name(&self) -> String {
        String::from("SetVolume")
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
    fn get_service(&self) -> Services {
        Services::RenderingControl
    }

    fn get_action_name(&self) -> String {
        String::from("GetVolume")
    }

    fn get_args_map(&self) -> HashMap<&str, &str> {
        let mut map = HashMap::new();
        map.insert("InstanceID", "0");
        map.insert("Channel", "Master");
        map
    }

    async fn res_to_output(
        &self,
        res: Result<reqwest::Response, reqwest::Error>,
    ) -> Result<String, String> {
        let res = res.map_err(|err| format!("Error sending request: {err}"))?;

        let status = res.status();
        let xml = res
            .text()
            .await
            .map_err(|err| format!("Error getting response body: {err}"))?;

        let volume = parse_getvolume(xml)?;

        Ok(format!(
            "Get volume request responded with {}\nVolume: {}",
            status, volume
        ))
    }
}

pub struct GetStatus;

#[async_trait]
impl Action for GetStatus {
    fn get_service(&self) -> Services {
        Services::AVTransport
    }

    fn get_action_name(&self) -> String {
        String::from("GetTransportInfo")
    }

    async fn res_to_output(
        &self,
        res: Result<reqwest::Response, reqwest::Error>,
    ) -> Result<String, String> {
        let res = res.map_err(|err| format!("Error sending request: {err}"))?;

        let status = res.status();
        let xml = res
            .text()
            .await
            .map_err(|err| format!("Error getting response body: {err}"))?;

        let data = parse_status(xml)?;

        Ok(format!(
            "Get status request responded with {}\n{}",
            status, data
        ))
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
    fn get_service(&self) -> Services {
        Services::AVTransport
    }

    fn get_action_name(&self) -> String {
        String::from("Seek")
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
    fn get_service(&self) -> Services {
        Services::AVTransport
    }

    fn get_action_name(&self) -> String {
        String::from("Next")
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
    fn get_service(&self) -> Services {
        Services::AVTransport
    }

    fn get_action_name(&self) -> String {
        String::from("Previous")
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
    fn get_service(&self) -> Services {
        Services::AVTransport
    }

    fn get_action_name(&self) -> String {
        String::from("EndDirectControlSession")
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
    fn get_service(&self) -> Services {
        Services::AVTransport
    }

    fn get_action_name(&self) -> String {
        String::from("AddURIToQueue")
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
    fn get_service(&self) -> Services {
        Services::AVTransport
    }

    fn get_action_name(&self) -> String {
        String::from("RemoveAllTracksFromQueue")
    }
}
