use std::env;

pub struct Features;

impl Features {
    pub fn video_capture_enabled() -> bool {
        env::var("VIDEO_CAPTURE_ENABLED").unwrap_or("false".to_string()) == "true"
    }
}
