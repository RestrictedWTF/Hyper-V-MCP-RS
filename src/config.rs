use crate::dpapi::{decrypt_from_base64, encrypt_to_base64};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct EncryptedCredential {
    pub username: String,
    pub password_encrypted: String,
}

impl EncryptedCredential {
    pub fn encrypt(username: &str, password: &str) -> Result<Self, crate::dpapi::DpapiError> {
        Ok(Self {
            username: username.to_string(),
            password_encrypted: encrypt_to_base64(password)?,
        })
    }

    pub fn decrypt_password(&self) -> Result<String, crate::dpapi::DpapiError> {
        decrypt_from_base64(&self.password_encrypted)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct Config {
    #[serde(default)]
    pub default_credential: Option<EncryptedCredential>,
    #[serde(default)]
    pub default_vhdx_path: Option<String>,
    #[serde(default)]
    pub powershell_direct_timeout_seconds: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct CredentialStore {
    #[serde(default)]
    pub vms: HashMap<String, EncryptedCredential>,
}

#[derive(Debug, Clone)]
pub struct ResolvedCredential {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to read config file: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to parse config JSON: {0}")]
    Json(#[from] serde_json::Error),
}

pub fn app_data_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("HYPERV_MCP_CONFIG_DIR") {
        return PathBuf::from(dir);
    }
    std::env::var("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("USERPROFILE").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home).join("AppData").join("Roaming")
        })
        .join("hyperv-mcp")
}

fn config_path() -> PathBuf {
    app_data_dir().join("config.json")
}

fn credentials_path() -> PathBuf {
    app_data_dir().join("credentials.json")
}

fn ensure_app_dir() -> std::io::Result<()> {
    let dir = app_data_dir();
    if !dir.exists() {
        std::fs::create_dir_all(&dir)?;
    }
    Ok(())
}

impl Config {
    pub fn load() -> Result<Self, ConfigError> {
        let path = config_path();
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(&path)?;
        let config = serde_json::from_str(&text)?;
        Ok(config)
    }

    pub fn save(&self) -> std::io::Result<()> {
        ensure_app_dir()?;
        let path = config_path();
        let text = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, text)
    }
}

impl CredentialStore {
    pub fn load() -> Result<Self, ConfigError> {
        let path = credentials_path();
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(&path)?;
        let store = serde_json::from_str(&text)?;
        Ok(store)
    }

    pub fn save(&self) -> std::io::Result<()> {
        ensure_app_dir()?;
        let path = credentials_path();
        let text = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, text)
    }

    pub fn set(&mut self, vm_name: &str, credential: EncryptedCredential) {
        self.vms.insert(vm_name.to_string(), credential);
    }
}

pub struct ConfigManager {
    pub config: Config,
    pub credentials: CredentialStore,
}

impl ConfigManager {
    pub fn load() -> Self {
        let config = match Config::load() {
            Ok(cfg) => cfg,
            Err(err) => {
                tracing::warn!("failed to load config, using defaults: {err}");
                Config::default()
            }
        };
        let credentials = match CredentialStore::load() {
            Ok(store) => store,
            Err(err) => {
                tracing::warn!("failed to load credentials, using defaults: {err}");
                CredentialStore::default()
            }
        };
        Self {
            config,
            credentials,
        }
    }

    pub fn resolve(
        &self,
        vm_name: &str,
        explicit_username: Option<&str>,
        explicit_password: Option<&str>,
    ) -> Option<ResolvedCredential> {
        if let (Some(u), Some(p)) = (explicit_username, explicit_password) {
            return Some(ResolvedCredential {
                username: u.to_string(),
                password: p.to_string(),
            });
        }

        if let Some(entry) = self.credentials.vms.get(vm_name) {
            if let Ok(password) = entry.decrypt_password() {
                return Some(ResolvedCredential {
                    username: entry.username.clone(),
                    password,
                });
            }
        }

        if let Some(entry) = &self.config.default_credential {
            if let Ok(password) = entry.decrypt_password() {
                return Some(ResolvedCredential {
                    username: entry.username.clone(),
                    password,
                });
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use std::sync::Mutex;

    static CONFIG_DIR_LOCK: Mutex<()> = Mutex::new(());

    fn with_config_dir<F, R>(dir: &Path, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        let _guard = CONFIG_DIR_LOCK.lock().unwrap();
        let var_name = "HYPERV_MCP_CONFIG_DIR";
        let previous = std::env::var_os(var_name);
        std::env::set_var(var_name, dir.as_os_str());
        let result = f();
        match previous {
            Some(value) => std::env::set_var(var_name, value),
            None => std::env::remove_var(var_name),
        }
        result
    }

    #[test]
    fn config_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        with_config_dir(dir.path(), || {
            let config = Config {
                default_credential: Some(EncryptedCredential::encrypt("Admin", "Pass").unwrap()),
                default_vhdx_path: Some("C:\\Base.vhdx".to_string()),
                powershell_direct_timeout_seconds: Some(120),
            };
            config.save().unwrap();
            let loaded = Config::load().unwrap();
            assert_eq!(loaded, config);
        });
    }

    #[test]
    fn missing_config_file_returns_default() {
        let dir = tempfile::tempdir().unwrap();
        with_config_dir(dir.path(), || {
            let loaded = Config::load().unwrap();
            assert_eq!(loaded, Config::default());
        });
    }

    #[test]
    fn malformed_config_json_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        with_config_dir(dir.path(), || {
            std::fs::write(config_path(), "not valid json {").unwrap();
            let result = Config::load();
            assert!(matches!(result, Err(ConfigError::Json(_))));
        });
    }

    #[test]
    fn credential_store_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        with_config_dir(dir.path(), || {
            let mut store = CredentialStore::default();
            store.set(
                "VM1",
                EncryptedCredential::encrypt("VmUser", "VmPass").unwrap(),
            );
            store.save().unwrap();
            let loaded = CredentialStore::load().unwrap();
            assert_eq!(loaded, store);
        });
    }

    #[test]
    fn credential_resolution_order() {
        let mut manager = ConfigManager {
            config: Config {
                default_credential: Some(
                    EncryptedCredential::encrypt("Default", "DefPass").unwrap(),
                ),
                ..Default::default()
            },
            credentials: CredentialStore::default(),
        };
        manager.credentials.set(
            "VM1",
            EncryptedCredential::encrypt("VmUser", "VmPass").unwrap(),
        );

        // explicit wins
        let r = manager
            .resolve("VM1", Some("Explicit"), Some("ExpPass"))
            .unwrap();
        assert_eq!(r.username, "Explicit");
        assert_eq!(r.password, "ExpPass");

        // vm-specific wins over default
        let r = manager.resolve("VM1", None, None).unwrap();
        assert_eq!(r.username, "VmUser");
        assert_eq!(r.password, "VmPass");

        // default fallback
        let r = manager.resolve("VM2", None, None).unwrap();
        assert_eq!(r.username, "Default");
        assert_eq!(r.password, "DefPass");

        // no match
        let manager2 = ConfigManager {
            config: Config::default(),
            credentials: CredentialStore::default(),
        };
        assert!(manager2.resolve("VMX", None, None).is_none());
    }
}
