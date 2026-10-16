//! Deterministic demo camera, also used to verify behavior without hardware.
use crate::*;

#[derive(Default)]
pub struct MockCamera {
    connected: bool,
    values: BTreeMap<String, i64>,
    pub writes: Vec<(String, i64)>,
}

pub fn mock_controls() -> Vec<Control> {
    DEFINITIONS
        .iter()
        .map(|def| {
            let (kind, min, max, value) = match def.key {
                "fov" => (
                    ControlKind::Menu(vec![
                        (65, "65° · fechado".into()),
                        (78, "78° · médio".into()),
                        (95, "95° · amplo".into()),
                    ]),
                    0,
                    65535,
                    78,
                ),
                "hdr" | "horizontal_flip" | "vertical_screen" => (ControlKind::Boolean, 0, 1, 0),
                "white_balance_automatic" | "focus_automatic_continuous" => {
                    (ControlKind::Boolean, 0, 1, 1)
                }
                "auto_exposure" => (
                    ControlKind::Menu(vec![
                        (1, "Manual".into()),
                        (3, "Prioridade de abertura".into()),
                    ]),
                    0,
                    3,
                    3,
                ),
                "power_line_frequency" => (
                    ControlKind::Menu(vec![
                        (0, "Desativado".into()),
                        (1, "50 Hz".into()),
                        (2, "60 Hz".into()),
                    ]),
                    0,
                    2,
                    2,
                ),
                "white_balance_temperature" => (ControlKind::Integer, 2800, 6500, 4600),
                "pan_absolute" | "tilt_absolute" => (ControlKind::Integer, -3600, 3600, 0),
                "zoom_absolute" => (ControlKind::Integer, 100, 400, 100),
                _ => (ControlKind::Integer, 0, 255, 128),
            };
            Control {
                key: def.key.into(),
                label: def.label.into(),
                group: def.group,
                kind,
                value: Some(value),
                min,
                max,
                step: 1,
                default: None,
                available: true,
                writable: true,
                inactive: false,
                volatile: false,
                reason: None,
            }
        })
        .collect()
}

impl CameraBackend for MockCamera {
    fn devices(&self) -> Result<Vec<DeviceInfo>> {
        Ok(vec![DeviceInfo {
            id: "demo:c200".into(),
            name: "PowerConf C200 · demonstração".into(),
            path: "demo".into(),
        }])
    }
    fn connect(&mut self, _: &DeviceInfo) -> Result<()> {
        self.connected = true;
        Ok(())
    }
    fn disconnect(&mut self) {
        self.connected = false;
    }
    fn controls(&self) -> Result<Vec<Control>> {
        if !self.connected {
            return Err(Error::Message("Câmera desconectada".into()));
        }
        let mut controls = mock_controls();
        for control in &mut controls {
            if let Some(value) = self.values.get(&control.key) {
                control.value = Some(*value);
            }
        }
        let values = controls
            .iter()
            .filter_map(|c| c.value.map(|v| (c.key.clone(), v)))
            .collect();
        for control in &mut controls {
            control.inactive = !dependency_active(&control.key, &values);
        }
        Ok(controls)
    }
    fn write(&mut self, key: &str, value: i64) -> Result<()> {
        let control = self
            .controls()?
            .into_iter()
            .find(|c| c.key == key)
            .ok_or_else(|| Error::Message("Controle desconhecido".into()))?;
        control.validate(value)?;
        self.values.insert(key.into(), value);
        self.writes.push((key.into(), value));
        Ok(())
    }
    fn modes(&self) -> Result<Vec<CaptureMode>> {
        Ok(vec![CaptureMode {
            width: 1280,
            height: 720,
            fps: 30,
            fourcc: "MJPG".into(),
        }])
    }
}
