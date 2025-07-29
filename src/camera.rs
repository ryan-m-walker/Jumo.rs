use std::io::Write;

use nokhwa::{
    Buffer, native_api_backend,
    pixel_format::RgbFormat,
    query,
    utils::{RequestedFormat, RequestedFormatType},
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

    // pub fn capture(&mut self) -> Result<Buffer, anyhow::Error> {
    // let backend = native_api_backend().unwrap();
    // let devices = query(backend)?;
    //
    // let device = devices
    //     .iter()
    //     .find(|d| d.human_name() != "Logitech StreamCam");
    //
    // let Some(device) = device else {
    //     anyhow::bail!("Camera device not found");
    // };
    //
    // let requested =
    //     RequestedFormat::new::<RgbFormat>(RequestedFormatType::AbsoluteHighestFrameRate);
    //
    // let mut camera = nokhwa::Camera::new(device.index().to_owned(), requested)?;
    //
    // println!("Camera opened");
    //
    // camera.open_stream()?;
    // let frame = camera.frame()?;
    // camera.stop_stream()?;
    //
    // let decoded = frame.decode_image::<RgbFormat>()?;
    //
    // let mut file = std::fs::File::create("frame.jpg")?;
    // file.write_all(&frame.buffer_bytes())?;
    // file.flush()?;
    //
    // Ok(frame)
    // }
}
