use c200_core::{DeviceInfo, Result};
use std::{collections::BTreeMap, fs, path::Path};
use v4l::{Device, capability::Flags};

fn text(path: impl AsRef<Path>) -> String {
    fs::read_to_string(path)
        .unwrap_or_default()
        .trim()
        .to_owned()
}

fn usb_identity(interface: &Path) -> Option<(String, String)> {
    for parent in interface.ancestors() {
        if text(parent.join("idVendor")) == "291a" && text(parent.join("idProduct")) == "3369" {
            let serial = text(parent.join("serial"));
            let id = if serial.is_empty() {
                format!("usb-path:{}", parent.file_name()?.to_str()?)
            } else {
                format!("usb:291a:3369:{serial}")
            };
            return Some((id, text(parent.join("product"))));
        }
    }
    None
}

pub fn devices() -> Result<Vec<DeviceInfo>> {
    let entries = match fs::read_dir("/sys/class/video4linux") {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error.into()),
    };
    let mut cameras = BTreeMap::new();
    let mut entries: Vec<_> = entries.filter_map(|e| e.ok()).collect();
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let Ok(interface) = fs::canonicalize(entry.path().join("device")) else {
            continue;
        };
        let Some((id, name)) = usb_identity(&interface) else {
            continue;
        };
        let path = format!("/dev/{}", entry.file_name().to_string_lossy());
        // Metadata nodes normally have index 1. Prefer index 0 when the
        // session has no permission to QUERYCAP, so the UI can explain access.
        let capture = match Device::with_path(&path).and_then(|d| d.query_caps()) {
            Ok(caps) => caps.capabilities.contains(Flags::VIDEO_CAPTURE),
            Err(_) => text(entry.path().join("index")) == "0",
        };
        if capture {
            cameras.entry(id.clone()).or_insert(DeviceInfo {
                id,
                name: if name.is_empty() {
                    "Anker PowerConf C200".into()
                } else {
                    name
                },
                path,
            });
        }
    }
    Ok(cameras.into_values().collect())
}
