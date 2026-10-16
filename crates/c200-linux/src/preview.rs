use c200_core::{CaptureMode, DeviceInfo, Error, Frame, Result};
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc::Sender,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
use v4l::{
    Device, FourCC,
    buffer::Type,
    io::{mmap::Stream, traits::CaptureStream},
    video::Capture,
};

pub type LatestFrame = Arc<Mutex<Option<Frame>>>;
#[derive(Debug)]
pub enum PreviewEvent {
    Started(u64, CaptureMode),
    Failed(u64, String),
}

pub struct Preview {
    pub frames: LatestFrame,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Default for Preview {
    fn default() -> Self {
        Self {
            frames: Arc::default(),
            stop: Arc::new(AtomicBool::new(false)),
            thread: None,
        }
    }
}
impl Preview {
    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        *self.frames.lock().unwrap_or_else(|p| p.into_inner()) = None;
    }
    pub fn start(
        &mut self,
        epoch: u64,
        device: DeviceInfo,
        mode: CaptureMode,
        events: Sender<PreviewEvent>,
    ) {
        self.stop();
        self.stop = Arc::new(AtomicBool::new(false));
        let stop = self.stop.clone();
        let frames = self.frames.clone();
        self.thread = Some(thread::spawn(move || {
            // libv4l-rs can panic while releasing buffers after USB errors.
            // Contain that failure in the capture session and keep the GUI alive.
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                if device.path == "demo" {
                    demo(epoch, &mode, &stop, &frames, &events);
                    Ok(())
                } else {
                    capture(epoch, &device, &mode, &stop, &frames, &events)
                }
            }));
            let error = match result { Ok(Ok(())) => None, Ok(Err(e)) => Some(e.to_string()), Err(_) => Some("The driver interrupted the capture session. Reconnect the camera or try starting preview again.".into()) };
            if !stop.load(Ordering::Relaxed)
                && let Some(error) = error
            {
                let _ = events.send(PreviewEvent::Failed(epoch, error));
            }
        }));
    }
}
impl Drop for Preview {
    fn drop(&mut self) {
        self.stop();
    }
}

fn publish(frames: &LatestFrame, frame: Frame) {
    *frames.lock().unwrap_or_else(|p| p.into_inner()) = Some(frame);
}

fn capture(
    epoch: u64,
    device: &DeviceInfo,
    mode: &CaptureMode,
    stop: &AtomicBool,
    frames: &LatestFrame,
    events: &Sender<PreviewEvent>,
) -> Result<()> {
    let device = Device::with_path(&device.path)?;
    let fourcc: [u8; 4] = mode
        .fourcc
        .as_bytes()
        .try_into()
        .map_err(|_| Error::Message("Invalid video format".into()))?;
    let format = device.set_format(&v4l::Format::new(
        mode.width,
        mode.height,
        FourCC::new(&fourcc),
    ))?;
    if format.fourcc != FourCC::new(b"MJPG") && format.fourcc != FourCC::new(b"YUYV") {
        return Err(Error::Message(
            "The driver selected an unsupported format".into(),
        ));
    }
    let parameters = device.set_params(&v4l::video::capture::Parameters::with_fps(mode.fps))?;
    let mut stream = Stream::with_buffers(&device, Type::VideoCapture, 4)?;
    stream.set_timeout(Duration::from_millis(100));
    let actual = CaptureMode {
        width: format.width,
        height: format.height,
        fps: parameters.interval.denominator / parameters.interval.numerator.max(1),
        fourcc: format.fourcc.to_string(),
    };
    let mut started = false;
    let mut last_frame = Instant::now();
    let mut sequence = 0;
    while !stop.load(Ordering::Relaxed) {
        let (buffer, meta) = match stream.next() {
            Ok(frame) => frame,
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::TimedOut
                        | std::io::ErrorKind::WouldBlock
                        | std::io::ErrorKind::Interrupted
                ) =>
            {
                if last_frame.elapsed() > Duration::from_secs(5) {
                    return Err(Error::Message(
                        "The camera is not delivering frames. Pause preview and try again.".into(),
                    ));
                }
                continue;
            }
            Err(error) => return Err(error.into()),
        };
        let used = (meta.bytesused as usize).min(buffer.len());
        let frame = decode(&actual, format.stride as usize, &buffer[..used], sequence)?;
        if !started {
            let _ = events.send(PreviewEvent::Started(epoch, actual.clone()));
            started = true;
        }
        publish(frames, frame);
        sequence += 1;
        last_frame = Instant::now();
    }
    Ok(())
}

pub fn decode(mode: &CaptureMode, stride: usize, data: &[u8], sequence: u64) -> Result<Frame> {
    if mode.width == 0 || mode.height == 0 || mode.width > 8192 || mode.height > 8192 {
        return Err(Error::Message("Invalid video dimensions".into()));
    }
    if mode.fourcc == "MJPG" {
        let mut reader =
            image::ImageReader::with_format(std::io::Cursor::new(data), image::ImageFormat::Jpeg);
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(8192);
        limits.max_image_height = Some(8192);
        reader.limits(limits);
        let rgb = reader
            .decode()
            .map_err(|e| Error::Message(format!("Invalid MJPEG frame: {e}")))?
            .into_rgb8();
        return Ok(Frame {
            width: rgb.width(),
            height: rgb.height(),
            rgb: rgb.into_raw(),
            sequence,
        });
    }
    if mode.fourcc != "YUYV" || !mode.width.is_multiple_of(2) {
        return Err(Error::Message("Unsupported video format".into()));
    }
    let width = mode.width as usize;
    let height = mode.height as usize;
    let row_bytes = width * 2;
    let stride = stride.max(row_bytes);
    if data.len() < (height - 1) * stride + row_bytes {
        return Err(Error::Message("Incomplete YUYV frame".into()));
    }
    let mut rgb = vec![0; width * height * 3];
    for row in 0..height {
        for (index, pair) in data[row * stride..row * stride + row_bytes]
            .as_chunks::<4>()
            .0
            .iter()
            .enumerate()
        {
            let u = i32::from(pair[1]) - 128;
            let v = i32::from(pair[3]) - 128;
            for (pixel, y) in [pair[0], pair[2]].into_iter().enumerate() {
                let y = (i32::from(y) - 16).max(0);
                let offset = (row * width + index * 2 + pixel) * 3;
                rgb[offset] = ((298 * y + 409 * v + 128) >> 8).clamp(0, 255) as u8;
                rgb[offset + 1] = ((298 * y - 100 * u - 208 * v + 128) >> 8).clamp(0, 255) as u8;
                rgb[offset + 2] = ((298 * y + 516 * u + 128) >> 8).clamp(0, 255) as u8;
            }
        }
    }
    Ok(Frame {
        width: mode.width,
        height: mode.height,
        rgb,
        sequence,
    })
}

fn demo(
    epoch: u64,
    mode: &CaptureMode,
    stop: &AtomicBool,
    frames: &LatestFrame,
    events: &Sender<PreviewEvent>,
) {
    let _ = events.send(PreviewEvent::Started(epoch, mode.clone()));
    let mut sequence = 0;
    while !stop.load(Ordering::Relaxed) {
        let mut rgb = vec![0; (mode.width * mode.height * 3) as usize];
        for y in 0..mode.height {
            for x in 0..mode.width {
                let offset = ((y * mode.width + x) * 3) as usize;
                let checker = ((x / 80 + y / 80) % 2) as u8;
                rgb[offset] = 16 + checker * 8;
                rgb[offset + 1] = 28 + (x * 35 / mode.width) as u8;
                rgb[offset + 2] = 37 + (y * 50 / mode.height) as u8;
                if x.abs_diff(mode.width / 2) < 2 || y.abs_diff(mode.height / 2) < 2 {
                    rgb[offset..offset + 3].copy_from_slice(&[60, 105, 114]);
                }
                let moving = (sequence as u32 * 9) % mode.width;
                if x.abs_diff(moving) < 3 {
                    rgb[offset..offset + 3].copy_from_slice(&[72, 210, 181]);
                }
            }
        }
        publish(
            frames,
            Frame {
                width: mode.width,
                height: mode.height,
                rgb,
                sequence,
            },
        );
        sequence += 1;
        thread::sleep(Duration::from_millis(33));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn yuyv_conversion_respects_row_padding_and_rejects_truncation() {
        let mode = CaptureMode {
            width: 2,
            height: 2,
            fps: 30,
            fourcc: "YUYV".into(),
        };
        let data = [16, 128, 235, 128, 99, 99, 99, 99, 235, 128, 16, 128];
        let frame = decode(&mode, 8, &data, 7).unwrap();
        assert_eq!(frame.sequence, 7);
        assert_eq!(frame.rgb, [0, 0, 0, 255, 255, 255, 255, 255, 255, 0, 0, 0]);
        assert!(decode(&mode, 8, &data[..11], 0).is_err());
        assert!(
            decode(
                &CaptureMode {
                    fourcc: "MJPG".into(),
                    ..mode
                },
                0,
                b"broken",
                0
            )
            .is_err()
        );
    }
    #[test]
    fn stopping_demo_releases_latest_frame() {
        let mut preview = Preview::default();
        let (tx, rx) = std::sync::mpsc::channel();
        preview.start(
            5,
            DeviceInfo {
                id: "demo".into(),
                name: "demo".into(),
                path: "demo".into(),
            },
            CaptureMode {
                width: 16,
                height: 16,
                fps: 30,
                fourcc: "MJPG".into(),
            },
            tx,
        );
        assert!(matches!(
            rx.recv_timeout(Duration::from_secs(2)).unwrap(),
            PreviewEvent::Started(5, _)
        ));
        preview.stop();
        assert!(preview.frames.lock().unwrap().is_none());
    }
}
