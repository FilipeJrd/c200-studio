use crate::interaction::WriteAck;

#[cfg(test)]
#[path = "controller_hardware_test.rs"]
mod hardware_test;
use c200_core::{mock::MockCamera, *};
use c200_linux::{
    LinuxCamera,
    preview::{LatestFrame, Preview, PreviewEvent},
};
use std::{
    collections::BTreeMap,
    sync::mpsc::{self, Receiver, RecvTimeoutError, Sender},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

pub enum Command {
    Scan,
    Connect {
        device: DeviceInfo,
        profile: Option<BTreeMap<String, i64>>,
        preview: bool,
    },
    Refresh,
    Write {
        ticket: u64,
        values: BTreeMap<String, i64>,
    },
    Apply(BTreeMap<String, i64>),
    Preview(Option<CaptureMode>),
    Shutdown,
}

pub enum Event {
    Devices(Vec<DeviceInfo>),
    Connected {
        device: DeviceInfo,
        controls: Vec<Control>,
        modes: Vec<CaptureMode>,
        selected_mode: Option<CaptureMode>,
    },
    Snapshot {
        controls: Vec<Control>,
        ack: Vec<WriteAck>,
    },
    WriteFailed(Vec<WriteAck>),
    Applied(Vec<String>),
    Preview {
        active: bool,
        message: String,
        mode: Option<CaptureMode>,
    },
    Disconnected(String),
    Error(String),
}

pub struct Controller {
    pub commands: Sender<Command>,
    pub events: Receiver<Event>,
    pub frames: LatestFrame,
    thread: Option<JoinHandle<()>>,
}
impl Controller {
    pub fn new(demo: bool) -> Self {
        let (tx, rx) = mpsc::channel();
        let (events, incoming) = mpsc::channel();
        let preview = Preview::default();
        let frames = preview.frames.clone();
        let thread = thread::spawn(move || Worker::new(demo, preview, events).run(rx));
        Self {
            commands: tx,
            events: incoming,
            frames,
            thread: Some(thread),
        }
    }
}
impl Drop for Controller {
    fn drop(&mut self) {
        let _ = self.commands.send(Command::Shutdown);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

struct Worker {
    backend: Box<dyn CameraBackend>,
    preview: Preview,
    events: Sender<Event>,
    preview_events: Receiver<PreviewEvent>,
    preview_tx: Sender<PreviewEvent>,
    device: Option<DeviceInfo>,
    previous_devices: Vec<DeviceInfo>,
    scan: Instant,
    epoch: u64,
    restore: Option<BTreeMap<String, i64>>,
    profile_pending: bool,
}
impl Worker {
    fn new(demo: bool, preview: Preview, events: Sender<Event>) -> Self {
        let (preview_tx, preview_events) = mpsc::channel();
        Self {
            backend: if demo {
                Box::<MockCamera>::default()
            } else {
                Box::<LinuxCamera>::default()
            },
            preview,
            events,
            preview_events,
            preview_tx,
            device: None,
            previous_devices: Vec::new(),
            scan: Instant::now(),
            epoch: 0,
            restore: None,
            profile_pending: false,
        }
    }
    fn emit(&self, event: Event) {
        let _ = self.events.send(event);
    }
    fn scan(&mut self, force: bool) {
        self.scan = Instant::now();
        match self.backend.devices() {
            Ok(devices) => {
                if self.device.as_ref().is_some_and(|d| {
                    !devices
                        .iter()
                        .any(|new| new.id == d.id && new.path == d.path)
                }) {
                    self.epoch += 1;
                    self.preview.stop();
                    self.backend.disconnect();
                    self.device = None;
                    self.restore = None;
                    self.emit(Event::Disconnected(
                        "Camera disconnected. Waiting to reconnect…".into(),
                    ));
                }
                if force || devices != self.previous_devices {
                    self.previous_devices = devices.clone();
                    self.emit(Event::Devices(devices));
                }
            }
            Err(error) => self.emit(Event::Error(error.to_string())),
        }
    }
    fn snapshot(&self, ack: Vec<WriteAck>) -> Vec<String> {
        self.snapshot_checked(ack, &[])
    }
    fn snapshot_checked(&self, ack: Vec<WriteAck>, required: &[String]) -> Vec<String> {
        match self.backend.controls() {
            Ok(controls) => {
                let errors = ack
                    .iter()
                    .flat_map(|(_, keys)| keys)
                    .chain(required)
                    .filter(|key| !controls.iter().any(|c| &c.key == *key && c.value.is_some()))
                    .map(|key| format!("{key}: readback failed"))
                    .collect();
                self.emit(Event::Snapshot { controls, ack });
                errors
            }
            Err(error) => {
                self.emit(Event::WriteFailed(ack));
                self.emit(Event::Error(error.to_string()));
                vec![format!("Readback failed: {error}")]
            }
        }
    }
    fn apply(&mut self, values: BTreeMap<String, i64>, report: bool) {
        let mut errors = apply_values(self.backend.as_mut(), &values);
        errors
            .extend(self.snapshot_checked(Vec::new(), &values.keys().cloned().collect::<Vec<_>>()));
        if report {
            self.emit(Event::Applied(errors));
        } else if !errors.is_empty() {
            self.emit(Event::Error(format!(
                "Some settings could not be restored: {}",
                errors.join("; ")
            )));
        }
    }
    fn start_preview(&mut self, mode: CaptureMode) {
        if let Some(device) = &self.device {
            self.epoch += 1;
            self.emit(Event::Preview {
                active: false,
                message: "Starting preview…".into(),
                mode: None,
            });
            self.preview
                .start(self.epoch, device.clone(), mode, self.preview_tx.clone());
        }
    }
    fn run(mut self, commands: Receiver<Command>) {
        self.scan(true);
        let mut deferred = None;
        loop {
            match deferred
                .take()
                .map(Ok)
                .unwrap_or_else(|| commands.recv_timeout(Duration::from_millis(40)))
            {
                Ok(Command::Shutdown) | Err(RecvTimeoutError::Disconnected) => break,
                Err(RecvTimeoutError::Timeout) => (),
                Ok(Command::Scan) => self.scan(true),
                Ok(Command::Connect {
                    device,
                    profile,
                    preview,
                }) => {
                    self.epoch += 1;
                    self.preview.stop();
                    self.device = None;
                    self.backend.disconnect();
                    self.restore = None;
                    self.profile_pending = false;
                    match self.backend.connect(&device) {
                        Ok(()) => {
                            self.device = Some(device.clone());
                            let controls = match self.backend.controls() { Ok(c) => c, Err(e) => { self.emit(Event::Error(e.to_string())); Vec::new() } };
                            let modes = self.backend.modes().unwrap_or_else(|e| { self.emit(Event::Error(format!("Could not list capture modes: {e}"))); Vec::new() });
                            let mode = preferred_mode(&modes);
                            self.emit(Event::Connected { device, controls, modes, selected_mode: mode.clone() });
                            if preview && let Some(mode) = mode {
                                self.profile_pending = profile.is_some(); self.restore = profile; self.start_preview(mode);
                            } else {
                                self.emit(Event::Preview { active: false, message: "Preview stopped".into(), mode: None });
                                if let Some(values) = profile { self.apply(values, true); }
                            }
                        }
                        Err(error) => self.emit(Event::Disconnected(format!("Could not open {}: {}. Check session permissions and the USB connection.", device.path, error))),
                    }
                }
                Ok(Command::Refresh) => {
                    if self.device.is_some() {
                        self.snapshot(Vec::new());
                    }
                }
                Ok(Command::Write { ticket, mut values }) => {
                    let mut ack = vec![(ticket, values.keys().cloned().collect())];
                    // Coalesce consecutive motion updates; never cross a profile,
                    // connection or preview command, which is an ordering barrier.
                    while let Ok(next) = commands.try_recv() {
                        if let Command::Write {
                            ticket,
                            values: latest,
                        } = next
                        {
                            ack.push((ticket, latest.keys().cloned().collect()));
                            values.extend(latest);
                        } else {
                            deferred = Some(next);
                            break;
                        }
                    }
                    let mut errors = Vec::new();
                    for (key, value) in &values {
                        if let Err(error) = self.backend.write(key, *value) {
                            errors.push(format!("{key}: {error}"));
                        }
                    }
                    // Edits made while capture starts must win over an earlier
                    // snapshot scheduled for restoration after format changes.
                    if let Some(restore) = &mut self.restore {
                        restore.extend(values.clone());
                    }
                    errors.extend(self.snapshot(ack));
                    if !errors.is_empty() {
                        self.emit(Event::Error(errors.join("; ")));
                    }
                }
                Ok(Command::Apply(values)) => {
                    if self.restore.is_some() {
                        self.restore = Some(values);
                        self.profile_pending = true;
                    } else {
                        self.apply(values, true);
                    }
                }
                Ok(Command::Preview(mode)) => {
                    if let Some(mode) = mode {
                        self.restore = self.backend.controls().ok().map(|controls| {
                            controls
                                .into_iter()
                                .filter(|c| c.available && c.writable && !c.volatile)
                                .filter_map(|c| c.value.map(|v| (c.key, v)))
                                .collect()
                        });
                        self.profile_pending = false;
                        self.start_preview(mode);
                    } else {
                        self.epoch += 1;
                        self.preview.stop();
                        if let Some(values) = self.restore.take() {
                            self.apply(values, self.profile_pending);
                        }
                        self.profile_pending = false;
                        self.emit(Event::Preview {
                            active: false,
                            message: "Capture released for other apps".into(),
                            mode: None,
                        });
                    }
                }
            }
            while let Ok(event) = self.preview_events.try_recv() {
                match event {
                    PreviewEvent::Started(epoch, mode) if epoch == self.epoch => {
                        if let Some(values) = self.restore.take() {
                            self.apply(values, self.profile_pending);
                        }
                        self.profile_pending = false;
                        self.emit(Event::Preview {
                            active: true,
                            message: "Live preview".into(),
                            mode: Some(mode),
                        });
                        self.snapshot(Vec::new());
                    }
                    PreviewEvent::Failed(epoch, error) if epoch == self.epoch => {
                        if let Some(values) = self.restore.take() {
                            self.apply(values, self.profile_pending);
                        }
                        self.profile_pending = false;
                        self.emit(Event::Preview { active: false, message: format!("Preview unavailable: {error}. If another app is using the camera, stop its capture and try again."), mode: None });
                        self.snapshot(Vec::new());
                    }
                    _ => (),
                }
            }
            if self.scan.elapsed() >= Duration::from_secs(2) {
                self.scan(false);
            }
        }
        self.preview.stop();
        self.backend.disconnect();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn controller_connects_applies_profiles_and_reads_back() {
        let controller = Controller::new(true);
        let Event::Devices(devices) = controller
            .events
            .recv_timeout(Duration::from_secs(2))
            .unwrap()
        else {
            panic!("expected devices")
        };
        controller
            .commands
            .send(Command::Connect {
                device: devices[0].clone(),
                profile: Some(BTreeMap::from([("fov".into(), 65)])),
                preview: false,
            })
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(2);
        let mut confirmed = false;
        while Instant::now() < deadline {
            if let Event::Snapshot { controls, .. } = controller
                .events
                .recv_timeout(Duration::from_secs(2))
                .unwrap()
            {
                confirmed = controls
                    .iter()
                    .any(|c| c.key == "fov" && c.value == Some(65));
                break;
            }
        }
        assert!(confirmed);
    }

    #[test]
    fn preview_can_be_released_without_losing_controls() {
        let controller = Controller::new(true);
        let Event::Devices(devices) = controller
            .events
            .recv_timeout(Duration::from_secs(2))
            .unwrap()
        else {
            panic!("expected devices")
        };
        controller
            .commands
            .send(Command::Connect {
                device: devices[0].clone(),
                profile: None,
                preview: true,
            })
            .unwrap();
        loop {
            if let Event::Preview { active: true, .. } = controller
                .events
                .recv_timeout(Duration::from_secs(2))
                .unwrap()
            {
                break;
            }
        }
        controller.commands.send(Command::Preview(None)).unwrap();
        loop {
            if let Event::Preview { active: false, .. } = controller
                .events
                .recv_timeout(Duration::from_secs(2))
                .unwrap()
            {
                break;
            }
        }
        controller
            .commands
            .send(Command::Write {
                ticket: 1,
                values: BTreeMap::from([("brightness".into(), 65)]),
            })
            .unwrap();
        loop {
            if let Event::Snapshot { controls, ack } = controller
                .events
                .recv_timeout(Duration::from_secs(2))
                .unwrap()
                && ack
                    .iter()
                    .any(|(_, keys)| keys.contains(&"brightness".into()))
            {
                assert_eq!(
                    controls
                        .iter()
                        .find(|c| c.key == "brightness")
                        .unwrap()
                        .value,
                    Some(65)
                );
                break;
            }
        }
        assert!(controller.frames.lock().unwrap().is_none());
    }

    struct ReadbackFailure {
        wrote: bool,
    }
    impl CameraBackend for ReadbackFailure {
        fn devices(&self) -> Result<Vec<DeviceInfo>> {
            Ok(Vec::new())
        }
        fn connect(&mut self, _: &DeviceInfo) -> Result<()> {
            Ok(())
        }
        fn disconnect(&mut self) {}
        fn controls(&self) -> Result<Vec<Control>> {
            if self.wrote {
                Err(Error::Message("USB disconnected".into()))
            } else {
                Ok(c200_core::mock::mock_controls())
            }
        }
        fn write(&mut self, _: &str, _: i64) -> Result<()> {
            self.wrote = true;
            Ok(())
        }
        fn modes(&self) -> Result<Vec<CaptureMode>> {
            Ok(Vec::new())
        }
    }

    #[test]
    fn profile_is_not_reported_as_confirmed_when_readback_fails() {
        let (events, incoming) = mpsc::channel();
        let mut worker = Worker::new(true, Preview::default(), events);
        worker.backend = Box::new(ReadbackFailure { wrote: false });
        worker.apply(BTreeMap::from([("brightness".into(), 65)]), true);
        assert!(
            incoming
                .try_iter()
                .any(|event| matches!(event, Event::Applied(errors) if !errors.is_empty()))
        );
    }
}
