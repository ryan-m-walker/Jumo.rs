use std::io::Cursor;

use base64::{Engine, engine::general_purpose};
use image::{DynamicImage, ImageBuffer, imageops};
use nokhwa::{
    pixel_format::RgbFormat,
    utils::{CameraIndex, RequestedFormat, RequestedFormatType},
};

pub struct Camera {}

impl Camera {
    pub fn new() -> Self {
        Self {}
    }

    pub fn capture(&mut self) -> Result<Option<String>, anyhow::Error> {
        let requested =
            RequestedFormat::new::<RgbFormat>(RequestedFormatType::AbsoluteHighestFrameRate);
        let mut camera = nokhwa::Camera::new(CameraIndex::Index(0), requested)?;

        camera.open_stream()?;
        let buffer = camera.frame()?;
        camera.stop_stream()?;

        let decoded = buffer.decode_image::<RgbFormat>()?;
        let width = decoded.width();
        let height = decoded.height();

        let Some(buffer) =
            ImageBuffer::<image::Rgb<u8>, Vec<u8>>::from_raw(width, height, decoded.into_raw())
        else {
            return Ok(None);
        };

        let dynamic_img = DynamicImage::ImageRgb8(buffer);
        let resized = dynamic_img.resize(320, 320, imageops::FilterType::Nearest);

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

        rx.recv()?;

        Ok(())
    }
}
