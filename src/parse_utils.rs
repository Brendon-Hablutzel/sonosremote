use roxmltree;
use std::fmt;

use crate::actions::{Action, GetCurrentTrackInfo, GetQueue, GetStatus, GetVolume};

fn general_clean<T>(xml: String, action: &T) -> String
where
    T: Action + ?Sized,
{
    let action_name = action.get_action_name();
    let service_name = action.get_service().get_data().get_name();

    let response_service_url =
        format!(r#"xmlns:u="urn:schemas-upnp-org:service:{}""#, service_name);
    let new_action_tag_name = format!("{action_name}Response");
    let old_action_tag_name = format!("u:{action_name}Response");
    xml
        // .replace("s:Envelope", "Envelope")
        // .replace("s:Body", "Body")
        // RISKY BUT RESULTS IN MORE THOROUGH CLEANING:
        .replace("<s:", "<")
        .replace("</s:", "</")
        .replace(r#"xmlns:s="http://schemas.xmlsoap.org/soap/envelope/""#, "")
        .replace(
            r#"s:encodingStyle="http://schemas.xmlsoap.org/soap/encoding/""#,
            "",
        )
        .replace(&response_service_url, "")
        .replace(&old_action_tag_name, &new_action_tag_name)
}

fn ampersands_to_tags(xml: String) -> String {
    xml.replace("&quot;", "\"")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
}

fn clean_meta_data(xml: String) -> String {
    ampersands_to_tags(xml)
        .replace(r#"xmlns:dc="http://purl.org/dc/elements/1.1/""#, "")
        .replace(
            r#"xmlns:upnp="urn:schemas-upnp-org:metadata-1-0/upnp/""#,
            "",
        )
        .replace(
            r#"xmlns:r="urn:schemas-rinconnetworks-com:metadata-1-0/""#,
            "",
        )
        .replace(
            r#"xmlns="urn:schemas-upnp-org:metadata-1-0/DIDL-Lite/""#,
            "",
        )
        // .replace("upnp:albumArtURI", "albumArtURI")
        // .replace("dc:title", "title")
        // .replace("upnp:class", "class")
        // .replace("dc:creator", "creator")
        // .replace("upnp:album", "album")
        // .replace("upnp:originalTrackNumber", "originalTrackNumber")
        // .replace("r:albumArtist", "albumArtist")
        // .replace("r:streamContent", "streamContent")
        // RISKY BUT RESULTS IN MORE THOROUGH CLEANING:
        .replace("<dc:", "<")
        .replace("<upnp:", "<")
        .replace("<r:", "<")
        .replace("</dc:", "</")
        .replace("</upnp:", "</")
        .replace("</r:", "</")
}

#[derive(Debug)]
pub struct CurrentData {
    position: String,
    duration: String,
    uri: String,
    title: Option<String>,
    artist: Option<String>,
}

impl fmt::Display for CurrentData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let title = self.title.as_ref().map_or("None", |n| n);
        let artist = self.artist.as_ref().map_or("None", |n| n);
        write!(
            f,
            "{} by {}\nURI: {}\nPosition: {}/{}",
            title, artist, self.uri, self.position, self.duration
        )
    }
}

fn get_tag_by_name<'a>(
    parsed_xml: &'a roxmltree::Document,
    tag_name: &str,
) -> Result<roxmltree::Node<'a, 'a>, String> {
    let tag = parsed_xml
        .descendants()
        .find(|n| n.has_tag_name(tag_name))
        .ok_or(format!("'{tag_name}' tag not found"))?;
    Ok(tag)
}

fn get_text<'a>(tag: roxmltree::Node, err: &'a str) -> Result<String, &'a str> {
    Ok(tag.text().ok_or(err)?.to_owned())
}

pub fn parse_current(xml: String, action: &GetCurrentTrackInfo) -> Result<CurrentData, String> {
    let xml = clean_meta_data(general_clean(xml, action));
    let parsed_xml =
        roxmltree::Document::parse(&xml).map_err(|err| format!("Error parsing xml: {err}"))?;

    let duration = get_text(
        get_tag_by_name(&parsed_xml, "TrackDuration")?,
        "No duration found",
    )?;

    if duration == "NOT_IMPLEMENTED" {
        // this occurs when spotify has control
        return Err("Unable to fetch current track data".to_owned());
    };

    let uri = get_text(get_tag_by_name(&parsed_xml, "TrackURI")?, "No track found")?;

    let title = get_tag_by_name(&parsed_xml, "title")
        .ok()
        .map(|n| get_text(n, "Error getting title"))
        .transpose()?;

    let artist = get_tag_by_name(&parsed_xml, "albumArtist")
        .ok()
        .map(|n| get_text(n, "Error getting artist"))
        .transpose()?;

    let position = get_text(
        get_tag_by_name(&parsed_xml, "RelTime")?,
        "No position found",
    )?;

    Ok(CurrentData {
        position,
        duration,
        uri,
        title,
        artist,
    })
}

pub fn parse_getvolume(xml: String, action: &GetVolume) -> Result<String, String> {
    let xml = general_clean(xml, action);
    let parsed_xml =
        roxmltree::Document::parse(&xml).map_err(|err| format!("Error parsing xml: {err}"))?;

    let volume = get_text(
        get_tag_by_name(&parsed_xml, "CurrentVolume")?,
        "No volume found",
    )?;

    Ok(volume)
}

#[derive(Debug)]
pub struct PlaybackStatus {
    state: String,
    status: String,
}

impl fmt::Display for PlaybackStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}: {}", self.status, self.state)
    }
}

pub fn parse_status(xml: String, action: &GetStatus) -> Result<PlaybackStatus, String> {
    let xml = general_clean(xml, action);
    let parsed_xml =
        roxmltree::Document::parse(&xml).map_err(|err| format!("Error parsing xml: {err}"))?;

    let state = get_text(
        get_tag_by_name(&parsed_xml, "CurrentTransportState")?,
        "No state found",
    )?;

    let status = get_text(
        get_tag_by_name(&parsed_xml, "CurrentTransportStatus")?,
        "No status found",
    )?;

    Ok(PlaybackStatus { state, status })
}

#[derive(Debug)]
pub struct QueueItem {
    duration: Option<String>,
    uri: String,
    title: Option<String>,
    artist: Option<String>,
}

impl fmt::Display for QueueItem {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let title = self.title.as_ref().map_or("None", |n| n);
        let artist = self.artist.as_ref().map_or("None", |n| n);
        let duration = self.duration.as_ref().map_or("None", |n| n);
        write!(
            f,
            "{} by {}\nURI: {}\nDuration: {}",
            title, artist, self.uri, duration
        )
    }
}

fn parse_queue_item(item: roxmltree::Node) -> Result<QueueItem, String> {
    let res = item
        .descendants()
        .find(|n| n.has_tag_name("res"))
        .ok_or("'res' tag not found")?;

    let title = item
        .descendants()
        .find(|n| n.has_tag_name("title"))
        .map(|n| get_text(n, "Error getting title"))
        .transpose()?;

    let artist = item
        .descendants()
        .find(|n| n.has_tag_name("albumArtist"))
        .map(|n| get_text(n, "Error getting artist"))
        .transpose()?;

    let duration = res.attribute("duration").map(|n| n.to_owned());

    let uri = get_text(res, "No URI found")?;

    Ok(QueueItem {
        duration,
        uri,
        title,
        artist,
    })
}

pub fn parse_queue(xml: String, action: &GetQueue) -> Result<Vec<QueueItem>, String> {
    let xml = clean_meta_data(general_clean(xml, action));

    let parsed =
        roxmltree::Document::parse(&xml).map_err(|err| format!("Error parsing xml: {err}"))?;

    let items: Result<Vec<QueueItem>, String> = parsed
        .descendants()
        .filter(|n| n.has_tag_name("item"))
        .map(|item| parse_queue_item(item))
        .collect();
    let items = items?;
    Ok(items)
}

// ?Sized is required because size of 'action' is not known at compile time
pub fn get_error_code<T>(xml: String, action: &T) -> Result<String, String>
where
    T: Action + ?Sized,
{
    let xml = general_clean(xml, action);
    let parsed =
        roxmltree::Document::parse(&xml).map_err(|err| format!("Error parsing xml: {err}"))?;
    let error_code = get_text(
        get_tag_by_name(&parsed, "errorCode")?,
        "Could not find error code",
    )?;
    Ok(error_code)
}
