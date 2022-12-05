use crate::actions::{self, Action};
use reqwest;
use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::str;
use std::str::FromStr;
use xml_builder::{XMLBuilder, XMLElement, XMLError, XMLVersion};

const DESCRIPTION_ENDPOINT: &str = "/xml/device_description.xml";

fn build_sonos_url(ip: Ipv4Addr, endpoint: &str) -> String {
    format!("http://{ip}:1400{endpoint}")
}

pub async fn enter_queue(s: &Speaker) -> Result<String, String> {
    let uri = format!("x-rincon-queue:{}#0", &s.uid);
    s.cmd(actions::SetURI::new(uri)).await
}

pub async fn action_then_current(s: &Speaker, action: impl Action + std::marker::Sync) -> Result<String, String> {
    let _action_res = s.cmd(action).await?;
    let current_track_info = s.cmd(actions::GetCurrentTrackInfo).await?;
    Ok(current_track_info)
}


pub struct Speaker {
    ip_addr: Ipv4Addr,
    uid: String,
    client: reqwest::Client,
}

impl Speaker {
    pub async fn new(ip: &str) -> Result<Self, String> {
        let ip_addr =
            Ipv4Addr::from_str(ip).map_err(|_| "Failed to parse IP address".to_owned())?;

        let client = reqwest::Client::new();

        let description_res = client
            .get(build_sonos_url(ip_addr, DESCRIPTION_ENDPOINT))
            .send()
            .await
            .map_err(|err| format!("Request to device failed {err}"))?;
        // parse the above request for speaker details such as zone, name, and uid

        let status = description_res.status();

        if let reqwest::StatusCode::OK = status {
            let text = description_res
                .text()
                .await
                .map_err(|err| format!("Failed to parse response text: {err}"))?;

            // I think this could be improved
            let uid_begin = text.find("RINCON").ok_or("Unable to find speaker uid")?;
            let after_uid = &text[uid_begin..];
            let uid_end = after_uid.find("<").ok_or("Error extracting speaker uid")? + uid_begin;
            let uid = String::from(&text[uid_begin..uid_end]);

            println!("Successfully connected to speaker");
            Ok(Speaker {
                ip_addr,
                uid,
                client,
            })
        } else {
            Err(format!("Device returned unsuccessful response: {}", status))
        }
    }

    // sync bound is required for default trait implementation
    pub async fn cmd(
        &self,
        action: impl actions::Action + std::marker::Sync,
    ) -> Result<String, String> {
        let action_name = action.get_action_name();
        let arguments = action.get_args_map();

        let service = action.get_service().get_data();
        let service_endpoint = service.get_endpoint();
        let service_name = service.get_name();

        let url = build_sonos_url(self.ip_addr, &service_endpoint);

        // find some other way to display XMLError
        let xml_bytes = generate_xml(&action_name, &service_name, arguments)
            .map_err(|err| format!("Error generating xml request: {:?}", err))?;

        let res = self
            .client
            .post(url)
            .body(xml_bytes)
            .header("Content-Type", "text/xml")
            .header(
                "SOAPACTION",
                format!(
                    "urn:schemas-upnp-org:service:{}#{}",
                    &service_name, &action_name
                ),
            )
            .send()
            .await;

        action.res_to_output(res).await
    }

    pub fn get_info(&self) -> String {
        format!("UID: {}", self.uid)
    }
}

// not checking for str validity since this fn is private
fn generate_xml(
    action: &str,
    service: &str,
    arguments: HashMap<&str, &str>,
) -> Result<Vec<u8>, XMLError> {
    let mut xml = XMLBuilder::new()
        .version(XMLVersion::XML1_1)
        .encoding("UTF-8".into())
        .build();

    let mut envelope = XMLElement::new("s:Envelope");
    envelope.add_attribute("xmlns:s", "http://schemas.xmlsoap.org/soap/envelope/");
    envelope.add_attribute(
        "s:encodingStyle",
        "http://schemas.xmlsoap.org/soap/encoding/",
    );

    let mut body = XMLElement::new("s:Body");

    let mut action = XMLElement::new(&format!("u:{}", action));
    action.add_attribute(
        "xmlns:u",
        &format!("urn:schemas-upnp-org:service:{}", service),
    );

    for (arg, value) in arguments {
        let mut xml_obj = XMLElement::new(arg);
        xml_obj.add_text(value.to_owned())?;
        action.add_child(xml_obj)?;
    }

    body.add_child(action)?;

    envelope.add_child(body)?;

    xml.set_root_element(envelope);

    let mut writer = Vec::new();
    xml.generate(&mut writer)?;
    Ok(writer)
}
