use std::env;

pub struct Features;

impl Features {
    pub fn video_capture_enabled() -> bool {
        env::var("ENABLE_IMAGE_CAPTURE").unwrap_or("false".to_string()) == "true"
    }
}
