use crate::*;
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

const VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    pub device_id: String,
    pub values: BTreeMap<String, i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub version: u32,
    pub selected_device: Option<String>,
    pub selected_profile: Option<String>,
    #[serde(default)]
    pub selected_profiles: BTreeMap<String, String>,
    pub profiles: Vec<Profile>,
    pub preview_enabled: bool,
    #[serde(default)]
    pub theme: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: VERSION,
            selected_device: None,
            selected_profile: None,
            selected_profiles: BTreeMap::new(),
            profiles: Vec::new(),
            preview_enabled: true,
            theme: None,
        }
    }
}

impl Settings {
    pub fn load(path: &Path) -> Result<Self> {
        let contents = match fs::read(path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self::default());
            }
            Err(error) => return Err(error.into()),
        };
        let settings: Self = serde_json::from_slice(&contents)?;
        if settings.version != VERSION {
            return Err(Error::Message(format!(
                "Unsupported configuration version: {}. The file has been preserved.",
                settings.version
            )));
        }
        Ok(settings)
    }
    pub fn save(&self, path: &Path) -> Result<()> {
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        fs::create_dir_all(parent)?;
        static SERIAL: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let suffix = SERIAL.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let temp = path.with_extension(format!("{}-{suffix}.tmp", std::process::id()));
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)?;
        let result = (|| -> Result<()> {
            file.write_all(&serde_json::to_vec_pretty(self)?)?;
            file.sync_all()?;
            fs::rename(&temp, path)?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(temp);
        }
        result
    }
    pub fn profile(&self, name: &str, device_id: &str) -> Option<&Profile> {
        self.profiles
            .iter()
            .find(|p| p.name == name && p.device_id == device_id)
    }
    pub fn store(&mut self, profile: Profile) -> Result<()> {
        if profile.name.trim().is_empty() {
            return Err(Error::Message("Enter a profile name".into()));
        }
        if let Some(existing) = self
            .profiles
            .iter_mut()
            .find(|p| p.name == profile.name && p.device_id == profile.device_id)
        {
            *existing = profile;
        } else {
            self.profiles.push(profile);
        }
        Ok(())
    }
}

pub fn config_path() -> Result<PathBuf> {
    if let Some(path) = std::env::var_os("XDG_CONFIG_HOME").filter(|p| Path::new(p).is_absolute()) {
        return Ok(PathBuf::from(path).join("c200-studio/settings.json"));
    }
    std::env::var_os("HOME")
        .map(|home| PathBuf::from(home).join(".config/c200-studio/settings.json"))
        .ok_or_else(|| Error::Message("Neither HOME nor XDG_CONFIG_HOME is set".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn profiles_round_trip_with_device_binding() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let mut settings = Settings::default();
        settings
            .store(Profile {
                name: "Meetings · ☕".into(),
                device_id: "camera-a".into(),
                values: BTreeMap::from([("fov".into(), 78)]),
            })
            .unwrap();
        settings.save(&path).unwrap();
        settings.save(&path).unwrap();
        let loaded = Settings::load(&path).unwrap();
        assert_eq!(
            loaded.profile("Meetings · ☕", "camera-a").unwrap().values["fov"],
            78
        );
        assert!(loaded.profile("Meetings · ☕", "camera-b").is_none());
    }
    #[test]
    fn invalid_or_future_settings_are_not_silently_replaced() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        assert!(Settings::load(&path).unwrap().profiles.is_empty());
        fs::write(&path, "broken").unwrap();
        assert!(Settings::load(&path).is_err());
        let settings = Settings {
            version: 99,
            ..Settings::default()
        };
        fs::write(&path, serde_json::to_vec(&settings).unwrap()).unwrap();
        assert!(Settings::load(&path).is_err());
    }
}
