pub trait Service {
    fn get_name(&self) -> &'static str;

    fn get_endpoint(&self) -> &'static str;
}

pub struct AVTransport;

impl Service for AVTransport {
    fn get_name(&self) -> &'static str {
        "AVTransport:1"
    }

    fn get_endpoint(&self) -> &'static str {
        "/MediaRenderer/AVTransport/Control"
    }
}

pub struct ContentDirectory;

impl Service for ContentDirectory {
    fn get_name(&self) -> &'static str {
        "ContentDirectory:1"
    }

    fn get_endpoint(&self) -> &'static str {
        "/MediaServer/ContentDirectory/Control"
    }
}

pub struct RenderingControl;

impl Service for RenderingControl {
    fn get_name(&self) -> &'static str {
        "RenderingControl:1"
    }

    fn get_endpoint(&self) -> &'static str {
        "/MediaRenderer/RenderingControl/Control"
    }
}
