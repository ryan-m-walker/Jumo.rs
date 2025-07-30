use image::ImageBuffer;
use nokhwa::{
    pixel_format::RgbFormat,
    utils::{CameraIndex, RequestedFormat, RequestedFormatType},
};

pub struct Camera {
    // event_sender: mpsc::Sender<AppEvent>,
}

impl Camera {
    pub fn new() -> Self {
        // pub fn new(event_sender: mpsc::Sender<AppEvent>) -> Self {
        Self {}
        // Self { event_sender }
    }

    pub fn capture(&mut self) -> Result<(), anyhow::Error> {
        let requested =
            RequestedFormat::new::<RgbFormat>(RequestedFormatType::AbsoluteHighestFrameRate);
        let mut camera = nokhwa::Camera::new(CameraIndex::Index(0), requested).unwrap();

        camera.open_stream().unwrap();
        let buffer = camera.frame().unwrap();
        camera.stop_stream().unwrap();

        let decoded = buffer.decode_image::<RgbFormat>().unwrap();
        let width = decoded.width();
        let height = decoded.height();

        let img =
            ImageBuffer::<image::Rgb<u8>, Vec<u8>>::from_raw(width, height, decoded.into_raw())
                .unwrap();

        img.save("frame.jpg").unwrap();

        Ok(())
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
