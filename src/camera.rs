use std::io::Cursor;

use base64::{Engine, engine::general_purpose};
use image::{DynamicImage, ImageBuffer, Rgb, imageops};
use nokhwa::{
    pixel_format::RgbFormat,
    utils::{CameraIndex, RequestedFormat, RequestedFormatType},
};

pub type Img = ImageBuffer<Rgb<u8>, Vec<u8>>;

pub struct Camera {
    // event_sender: mpsc::Sender<AppEvent>,
}

impl Camera {
    pub fn new() -> Self {
        // pub fn new(event_sender: mpsc::Sender<AppEvent>) -> Self {
        Self {}
        // Self { event_sender }
    }

    pub fn capture(&mut self) -> Result<Option<String>, anyhow::Error> {
        let requested =
            RequestedFormat::new::<RgbFormat>(RequestedFormatType::AbsoluteHighestFrameRate);
        let mut camera = nokhwa::Camera::new(CameraIndex::Index(0), requested).unwrap();

        camera.open_stream()?;
        let buffer = camera.frame()?;
        camera.stop_stream()?;

        let decoded = buffer.decode_image::<RgbFormat>().unwrap();
        let width = decoded.width();
        let height = decoded.height();

        let Some(buffer) =
            ImageBuffer::<image::Rgb<u8>, Vec<u8>>::from_raw(width, height, decoded.into_raw())
        else {
            return Ok(None);
        };

        let dynamic_img = DynamicImage::ImageRgb8(buffer);
        let resized = dynamic_img.resize(640, 640, imageops::FilterType::Lanczos3);

        let mut jpeg_bytes = Vec::new();
        resized.write_to(&mut Cursor::new(&mut jpeg_bytes), image::ImageFormat::Jpeg)?;
        let base64 = general_purpose::STANDARD.encode(&jpeg_bytes);

        Ok(Some(base64))
    }

    pub fn start_nokhwa() -> Result<(), anyhow::Error> {
        let (tx, rx) = std::sync::mpsc::channel();

        nokhwa::nokhwa_initialize(move |_| {
            let _ = tx.send(());
        });

        rx.recv().unwrap();

        Ok(())
    }
}
