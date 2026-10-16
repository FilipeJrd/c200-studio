//! Camera-independent domain types. No GUI, device I/O, or Linux ABI here.
pub mod mock;
pub mod profiles;

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Message(String),
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid configuration: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub id: String,
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Group {
    Framing,
    Image,
    Focus,
    Advanced,
}

impl Group {
    pub const ALL: [Self; 4] = [Self::Framing, Self::Image, Self::Focus, Self::Advanced];
    pub fn label(self) -> &'static str {
        match self {
            Self::Framing => "Framing",
            Self::Image => "Image",
            Self::Focus => "Focus and exposure",
            Self::Advanced => "Advanced",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControlKind {
    Boolean,
    Integer,
    Menu(Vec<(i64, String)>),
}

#[derive(Debug, Clone)]
pub struct Control {
    pub key: String,
    pub label: String,
    pub group: Group,
    pub kind: ControlKind,
    pub value: Option<i64>,
    pub min: i64,
    pub max: i64,
    pub step: i64,
    pub default: Option<i64>,
    pub available: bool,
    pub writable: bool,
    pub inactive: bool,
    pub volatile: bool,
    pub reason: Option<String>,
}

impl Control {
    pub fn editable(&self) -> bool {
        self.available && self.writable && !self.inactive
    }
    pub fn validate(&self, value: i64) -> Result<()> {
        if !self.editable() {
            return Err(Error::Message(format!(
                "{} is not available for editing",
                self.label
            )));
        }
        if value < self.min || value > self.max || (value - self.min) % self.step.max(1) != 0 {
            return Err(Error::Message(format!(
                "{}: value is outside the range or does not match the allowed step",
                self.label
            )));
        }
        if let ControlKind::Menu(items) = &self.kind
            && self.key != "fov"
            && !items.iter().any(|(id, _)| *id == value)
        {
            return Err(Error::Message(format!(
                "{}: unsupported option",
                self.label
            )));
        }
        Ok(())
    }
}

pub struct Definition {
    pub key: &'static str,
    pub label: &'static str,
    pub group: Group,
}
pub const DEFINITIONS: [Definition; 21] = [
    Definition {
        key: "fov",
        label: "Field of view",
        group: Group::Framing,
    },
    Definition {
        key: "hdr",
        label: "HDR",
        group: Group::Image,
    },
    Definition {
        key: "horizontal_flip",
        label: "Mirror horizontally",
        group: Group::Framing,
    },
    Definition {
        key: "vertical_screen",
        label: "Camera portrait mode",
        group: Group::Framing,
    },
    Definition {
        key: "anti_flicker",
        label: "Vendor anti-flicker (raw value)",
        group: Group::Advanced,
    },
    Definition {
        key: "brightness",
        label: "Brightness",
        group: Group::Image,
    },
    Definition {
        key: "contrast",
        label: "Contrast",
        group: Group::Image,
    },
    Definition {
        key: "saturation",
        label: "Saturation",
        group: Group::Image,
    },
    Definition {
        key: "hue",
        label: "Hue",
        group: Group::Advanced,
    },
    Definition {
        key: "white_balance_automatic",
        label: "Automatic white balance",
        group: Group::Image,
    },
    Definition {
        key: "gamma",
        label: "Gamma",
        group: Group::Advanced,
    },
    Definition {
        key: "power_line_frequency",
        label: "Power line frequency",
        group: Group::Image,
    },
    Definition {
        key: "white_balance_temperature",
        label: "Color temperature",
        group: Group::Image,
    },
    Definition {
        key: "sharpness",
        label: "Sharpness",
        group: Group::Image,
    },
    Definition {
        key: "auto_exposure",
        label: "Exposure mode",
        group: Group::Focus,
    },
    Definition {
        key: "exposure_time_absolute",
        label: "Exposure time",
        group: Group::Focus,
    },
    Definition {
        key: "pan_absolute",
        label: "Horizontal position",
        group: Group::Framing,
    },
    Definition {
        key: "tilt_absolute",
        label: "Vertical position",
        group: Group::Framing,
    },
    Definition {
        key: "focus_absolute",
        label: "Manual focus",
        group: Group::Focus,
    },
    Definition {
        key: "focus_automatic_continuous",
        label: "Autofocus",
        group: Group::Focus,
    },
    Definition {
        key: "zoom_absolute",
        label: "Digital zoom",
        group: Group::Framing,
    },
];

pub fn unavailable(def: &Definition, reason: impl Into<String>) -> Control {
    Control {
        key: def.key.into(),
        label: def.label.into(),
        group: def.group,
        kind: ControlKind::Integer,
        value: None,
        min: 0,
        max: 0,
        step: 1,
        default: None,
        available: false,
        writable: false,
        inactive: false,
        volatile: false,
        reason: Some(reason.into()),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureMode {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub fourcc: String,
}
impl CaptureMode {
    pub fn label(&self) -> String {
        format!(
            "{} × {} · {} FPS · {}",
            self.width, self.height, self.fps, self.fourcc
        )
    }
}

pub fn preferred_mode(modes: &[CaptureMode]) -> Option<CaptureMode> {
    modes
        .iter()
        .min_by_key(|m| {
            (
                m.width.abs_diff(1280) + m.height.abs_diff(720),
                m.fps.abs_diff(30),
                u8::from(m.fourcc != "MJPG"),
            )
        })
        .cloned()
}

#[derive(Debug, Clone)]
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub rgb: Vec<u8>,
    pub sequence: u64,
}

pub trait CameraBackend: Send {
    fn devices(&self) -> Result<Vec<DeviceInfo>>;
    fn connect(&mut self, device: &DeviceInfo) -> Result<()>;
    fn disconnect(&mut self);
    fn controls(&self) -> Result<Vec<Control>>;
    fn write(&mut self, key: &str, value: i64) -> Result<()>;
    fn modes(&self) -> Result<Vec<CaptureMode>>;
}

/// V4L2's exposure menu uses 1 for manual and 2 for shutter priority.
/// Exposure time is writable in these modes; do not assume every menu is boolean.
pub fn dependency_active(key: &str, values: &BTreeMap<String, i64>) -> bool {
    match key {
        "focus_absolute" => values.get("focus_automatic_continuous") != Some(&1),
        "white_balance_temperature" => values.get("white_balance_automatic") != Some(&1),
        "exposure_time_absolute" => values
            .get("auto_exposure")
            .is_none_or(|v| *v == 1 || *v == 2),
        _ => true,
    }
}

/// Apply vendor state first (it can affect standard controls), then automatic
/// modes, then manual controls. Report partial failures without claiming success.
pub fn apply_values(
    backend: &mut dyn CameraBackend,
    values: &BTreeMap<String, i64>,
) -> Vec<String> {
    let first = [
        "fov",
        "hdr",
        "horizontal_flip",
        "vertical_screen",
        "anti_flicker",
        "auto_exposure",
        "white_balance_automatic",
        "focus_automatic_continuous",
    ];
    let mut keys: Vec<&str> = first
        .into_iter()
        .filter(|k| values.contains_key(*k))
        .collect();
    keys.extend(
        values
            .keys()
            .map(String::as_str)
            .filter(|k| !first.contains(k)),
    );
    let mut errors = Vec::new();
    for key in keys {
        if !dependency_active(key, values) {
            continue;
        }
        if let Err(error) = backend.write(key, values[key]) {
            errors.push(format!("{key}: {error}"));
        }
    }
    errors
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn profile_application_obeys_automatic_modes_and_reports_errors() {
        let mut backend = mock::MockCamera::default();
        let device = backend.devices().unwrap().remove(0);
        backend.connect(&device).unwrap();
        let values = BTreeMap::from([
            ("focus_automatic_continuous".into(), 1),
            ("focus_absolute".into(), 45),
            ("brightness".into(), 999),
        ]);
        let errors = apply_values(&mut backend, &values);
        assert_eq!(errors.len(), 1);
        assert!(
            !backend
                .writes
                .iter()
                .any(|(key, _)| key == "focus_absolute")
        );
    }
    #[test]
    fn sparse_menus_and_steps_are_validated() {
        let mut control = mock::mock_controls()
            .into_iter()
            .find(|c| c.key == "auto_exposure")
            .unwrap();
        assert!(control.validate(2).is_err());
        assert!(control.validate(1).is_ok());
        control.kind = ControlKind::Integer;
        control.min = -10;
        control.max = 10;
        control.step = 5;
        assert!(control.validate(-5).is_ok());
        assert!(control.validate(-4).is_err());
    }
    #[test]
    fn preview_prefers_720p30_mjpeg() {
        let modes = [
            CaptureMode {
                width: 1280,
                height: 720,
                fps: 30,
                fourcc: "YUYV".into(),
            },
            CaptureMode {
                width: 1280,
                height: 720,
                fps: 30,
                fourcc: "MJPG".into(),
            },
        ];
        assert_eq!(preferred_mode(&modes).unwrap().fourcc, "MJPG");
    }
}
