use crate::{discovery, vendor};
use c200_core::*;
use v4l::{
    Device,
    control::{Flags, Type, Value},
    video::Capture,
};

#[derive(Default)]
pub struct LinuxCamera {
    device: Option<Device>,
}

pub fn standard_id(key: &str) -> Option<u32> {
    use v4l::v4l_sys::*;
    Some(match key {
        "brightness" => V4L2_CID_BRIGHTNESS,
        "contrast" => V4L2_CID_CONTRAST,
        "saturation" => V4L2_CID_SATURATION,
        "hue" => V4L2_CID_HUE,
        "white_balance_automatic" => V4L2_CID_AUTO_WHITE_BALANCE,
        "gamma" => V4L2_CID_GAMMA,
        "power_line_frequency" => V4L2_CID_POWER_LINE_FREQUENCY,
        "white_balance_temperature" => V4L2_CID_WHITE_BALANCE_TEMPERATURE,
        "sharpness" => V4L2_CID_SHARPNESS,
        "auto_exposure" => V4L2_CID_EXPOSURE_AUTO,
        "exposure_time_absolute" => V4L2_CID_EXPOSURE_ABSOLUTE,
        "pan_absolute" => V4L2_CID_PAN_ABSOLUTE,
        "tilt_absolute" => V4L2_CID_TILT_ABSOLUTE,
        "focus_absolute" => V4L2_CID_FOCUS_ABSOLUTE,
        "focus_automatic_continuous" => V4L2_CID_FOCUS_AUTO,
        "zoom_absolute" => V4L2_CID_ZOOM_ABSOLUTE,
        _ => return None,
    })
}

impl LinuxCamera {
    fn device(&self) -> Result<&Device> {
        self.device
            .as_ref()
            .ok_or_else(|| Error::Message("Câmera desconectada".into()))
    }
}

impl CameraBackend for LinuxCamera {
    fn devices(&self) -> Result<Vec<DeviceInfo>> {
        discovery::devices()
    }
    fn connect(&mut self, info: &DeviceInfo) -> Result<()> {
        self.device = None;
        self.device = Some(Device::with_path(&info.path)?);
        Ok(())
    }
    fn disconnect(&mut self) {
        self.device = None;
    }
    fn controls(&self) -> Result<Vec<Control>> {
        let device = self.device()?;
        let descriptions = device.query_controls()?;
        let io = vendor::IoctlTransport(device.handle().fd());
        let mut controls = Vec::new();
        for def in &DEFINITIONS {
            let result = if let Some(spec) = vendor::control(def.key) {
                (|| -> Result<Control> {
                    let (readable, writable) = vendor::capabilities(&io, spec)?;
                    if !readable {
                        return Err(Error::Message("Leitura não suportada pelo firmware".into()));
                    }
                    let value = vendor::read(&io, spec)?;
                    let (kind, max) = match spec.encoding {
                        vendor::Encoding::Fov => (
                            ControlKind::Menu(vec![
                                (65, "65° · fechado".into()),
                                (78, "78° · médio".into()),
                                (95, "95° · amplo".into()),
                            ]),
                            65535,
                        ),
                        vendor::Encoding::Boolean => (ControlKind::Boolean, 1),
                        vendor::Encoding::Byte => (ControlKind::Integer, 255),
                    };
                    Ok(Control {
                        key: def.key.into(),
                        label: def.label.into(),
                        group: def.group,
                        kind,
                        value: Some(value),
                        min: 0,
                        max,
                        step: 1,
                        default: None,
                        available: true,
                        writable,
                        inactive: false,
                        volatile: false,
                        reason: None,
                    })
                })()
            } else {
                (|| -> Result<Control> {
                    let id = standard_id(def.key)
                        .ok_or_else(|| Error::Message("Controle desconhecido".into()))?;
                    let desc = descriptions
                        .iter()
                        .find(|d| d.id == id)
                        .ok_or_else(|| Error::Message("Não oferecido por esta câmera".into()))?;
                    if desc.flags.contains(Flags::DISABLED) {
                        return Err(Error::Message("Desabilitado pelo driver".into()));
                    }
                    let value = match device.control(id)?.value {
                        Value::Integer(v) => v,
                        Value::Boolean(v) => i64::from(v),
                        _ => return Err(Error::Message("Tipo não suportado".into())),
                    };
                    let kind = match desc.typ {
                        Type::Boolean => ControlKind::Boolean,
                        Type::Menu | Type::IntegerMenu => ControlKind::Menu(
                            desc.items
                                .as_ref()
                                .map(|items| {
                                    items
                                        .iter()
                                        .map(|(index, item)| (i64::from(*index), item.to_string()))
                                        .collect()
                                })
                                .unwrap_or_default(),
                        ),
                        _ => ControlKind::Integer,
                    };
                    let writable = !desc.flags.contains(Flags::READ_ONLY);
                    let inactive = desc.flags.intersects(Flags::INACTIVE | Flags::GRABBED);
                    Ok(Control {
                        key: def.key.into(),
                        label: def.label.into(),
                        group: def.group,
                        kind,
                        value: Some(value),
                        min: desc.minimum,
                        max: desc.maximum,
                        step: desc.step as i64,
                        default: Some(desc.default),
                        available: true,
                        writable,
                        inactive,
                        volatile: desc.flags.contains(Flags::VOLATILE),
                        reason: if inactive {
                            Some("Indisponível no modo atual da câmera".into())
                        } else if !writable {
                            Some("Somente leitura".into())
                        } else {
                            None
                        },
                    })
                })()
            };
            controls.push(result.unwrap_or_else(|error| {
                let reason = if vendor::control(def.key).is_some() && matches!(&error, Error::Io(io) if matches!(io.raw_os_error(), Some(libc::ENOENT | libc::EINVAL | libc::ENOTTY | libc::EPIPE))) {
                    "Não oferecido por este firmware".to_string()
                } else { error.to_string() };
                unavailable(def, reason)
            }));
        }
        let values = controls
            .iter()
            .filter_map(|c| c.value.map(|v| (c.key.clone(), v)))
            .collect();
        for control in &mut controls {
            if !dependency_active(&control.key, &values) {
                control.inactive = true;
                control.reason =
                    Some("Desative o modo automático correspondente para ajustar".into());
            }
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
        let device = self.device()?;
        if let Some(spec) = vendor::control(key) {
            vendor::write(&vendor::IoctlTransport(device.handle().fd()), spec, value)?;
        } else {
            let value = if matches!(control.kind, ControlKind::Boolean) {
                Value::Boolean(value != 0)
            } else {
                Value::Integer(value)
            };
            device.set_control(v4l::Control {
                id: standard_id(key).unwrap(),
                value,
            })?;
        }
        // Return successful writes even when firmware coerces the value. The
        // controller refreshes all controls and shows the confirmed state.
        Ok(())
    }
    fn modes(&self) -> Result<Vec<CaptureMode>> {
        let device = self.device()?;
        let mut modes = Vec::new();
        for format in device.enum_formats()? {
            let fourcc = format.fourcc.to_string();
            if fourcc != "MJPG" && fourcc != "YUYV" {
                continue;
            }
            for size in device.enum_framesizes(format.fourcc)? {
                let v4l::framesize::FrameSizeEnum::Discrete(size) = size.size else {
                    continue;
                };
                for interval in
                    device.enum_frameintervals(format.fourcc, size.width, size.height)?
                {
                    let v4l::frameinterval::FrameIntervalEnum::Discrete(fraction) =
                        interval.interval
                    else {
                        continue;
                    };
                    if fraction.numerator == 0 {
                        continue;
                    }
                    let fps = fraction.denominator / fraction.numerator;
                    if fps == 0 || fps > 60 {
                        continue;
                    }
                    let mode = CaptureMode {
                        width: size.width,
                        height: size.height,
                        fps,
                        fourcc: fourcc.clone(),
                    };
                    if !modes.contains(&mode) {
                        modes.push(mode);
                    }
                }
            }
        }
        modes.sort_by_key(|m| (m.width, m.height, m.fps, m.fourcc.clone()));
        Ok(modes)
    }
}
