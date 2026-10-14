//! herdr's own settings, `config.toml`, edited in place: comments, key order and the blocks other tools generate
//! survive, and a symlinked file (dotfiles) is written through to its target. herdr rereads it on
//! `server.reload_config`.
use anyhow::{Context, anyhow};
use std::path::{Path, PathBuf};
use toml_edit::{DocumentMut, Item, Table, Value};

/// A string or a list of strings; anything else is none.
pub(super) fn str_list(value: &Value) -> Vec<String> {
    match value {
        Value::String(s) => vec![s.value().clone()],
        Value::Array(a) => a.iter().filter_map(|v| v.as_str().map(String::from)).collect(),
        _ => Vec::new(),
    }
}

/// Where herdr reads it: `HERDR_CONFIG_PATH`, else in [`dir`].
pub fn config_path() -> PathBuf {
    if let Some(path) = std::env::var_os("HERDR_CONFIG_PATH") {
        return path.into();
    }
    dir().join("config.toml")
}

/// herdr's folder, with its config and its sessions' sockets: `$XDG_CONFIG_HOME/herdr`, else `%APPDATA%\herdr` on
/// Windows, else `~/.config/herdr`.
pub(super) fn dir() -> PathBuf {
    let config = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).or_else(|| std::env::var_os("APPDATA").filter(|_| cfg!(windows)).map(PathBuf::from));
    config.unwrap_or_else(|| crate::settings::home().join(".config")).join("herdr")
}

/// The file as herdr reads it; a missing one is empty, so every key reads as herdr's default.
#[derive(Debug, Clone, Default)]
pub struct HerdrConfig(pub(super) DocumentMut);

impl HerdrConfig {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        match std::fs::read_to_string(path) {
            Ok(text) => Ok(Self(text.parse().with_context(|| format!("{} is not valid TOML", path.display()))?)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e).with_context(|| format!("cannot read {}", path.display())),
        }
    }

    /// A dotted key: `ui.toast.delivery`.
    fn get(&self, key: &str) -> Option<&Value> {
        key.split('.').try_fold(self.0.as_item(), |item, part| item.get(part))?.as_value()
    }

    pub fn bool(&self, key: &str, default: bool) -> bool {
        self.get(key).and_then(Value::as_bool).unwrap_or(default)
    }

    pub fn int(&self, key: &str, default: i64) -> i64 {
        self.get(key).and_then(Value::as_integer).unwrap_or(default)
    }

    pub fn str(&self, key: &str) -> Option<&str> {
        self.get(key).and_then(Value::as_str)
    }

    /// A key herdr takes as a string or a list of them (`next_tab = ["prefix+n", "ctrl+alt+]"]`); `None` when
    /// missing, so herdr's default applies.
    pub fn strs(&self, key: &str) -> Option<Vec<String>> {
        self.get(key).map(str_list)
    }

    /// Sets a dotted key, making missing tables; `None` removes it, so herdr's default applies. A value already
    /// there keeps its trailing comment.
    pub fn set(&mut self, key: &str, value: Option<Value>) -> anyhow::Result<()> {
        let (tables, last) = key.rsplit_once('.').unwrap_or(("", key));
        let mut table = self.0.as_table_mut();
        for part in tables.split('.').filter(|p| !p.is_empty()) {
            table = table
                .entry(part)
                .or_insert_with(|| Item::Table(Table::new()))
                .as_table_mut()
                .ok_or_else(|| anyhow!("`{part}` in herdr's config is not a table"))?;
        }
        match (value, table.get_mut(last)) {
            (None, _) => {
                table.remove(last);
            }
            (Some(mut value), Some(Item::Value(old))) => {
                *value.decor_mut() = old.decor().clone();
                *old = value;
            }
            (Some(value), _) => {
                table.insert(last, Item::Value(value));
            }
        }
        Ok(())
    }

    /// Writes next to the real file, then renames over it: a crash never leaves half a config, and a symlink stays
    /// a symlink.
    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        let target = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        if let Some(dir) = target.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = target.with_extension("toml.shepherd");
        std::fs::write(&tmp, self.0.to_string())?;
        std::fs::rename(&tmp, &target).with_context(|| format!("cannot write {}", target.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONFIG: &str = r#"onboarding = false
# herdr configuration

[ui]
# Ask for a tab name before creating a new tab.
prompt_new_tab_name = false

[ui.toast]
# off = disable pop-up notifications
delivery = "terminal" # where pop-ups go

[ui.sound]
enabled = false
"#;

    #[test]
    fn reads_dotted_keys_with_defaults() {
        let config: HerdrConfig = HerdrConfig(CONFIG.parse().unwrap());
        assert_eq!(config.str("ui.toast.delivery"), Some("terminal"));
        assert!(!config.bool("ui.sound.enabled", true));
        assert!(config.bool("session.resume_agents_on_restore", true), "missing reads as the default");
        assert_eq!(config.int("ui.toast.delay_seconds", 1), 1);
    }

    #[test]
    fn edits_keep_comments_and_order() {
        let mut config = HerdrConfig(CONFIG.parse().unwrap());
        config.set("ui.toast.delivery", Some("off".into())).unwrap();
        config.set("ui.sound.enabled", Some(true.into())).unwrap();
        assert_eq!(config.0.to_string(), CONFIG.replace(r#"= "terminal""#, r#"= "off""#).replace("enabled = false", "enabled = true"));
    }

    #[test]
    fn missing_tables_are_made_and_none_removes() {
        let mut config = HerdrConfig(CONFIG.parse().unwrap());
        config.set("session.resume_agents_on_restore", Some(false.into())).unwrap();
        assert!(config.0.to_string().ends_with("[session]\nresume_agents_on_restore = false\n"));
        config.set("ui.prompt_new_tab_name", None).unwrap();
        assert!(!config.0.to_string().contains("prompt_new_tab_name"));
        assert!(config.set("onboarding.x", Some(1.into())).is_err(), "a value is not a table");
    }

    #[cfg(unix)] // a symlink on Windows needs developer mode or admin
    #[test]
    fn saving_writes_through_a_symlink() {
        let dir = tempfile::tempdir().unwrap();
        let (real, link) = (dir.path().join("dotfiles.toml"), dir.path().join("config.toml"));
        std::fs::write(&real, CONFIG).unwrap();
        std::os::unix::fs::symlink(&real, &link).unwrap();
        let mut config = HerdrConfig::load(&link).unwrap();
        config.set("ui.sound.enabled", Some(true.into())).unwrap();
        config.save(&link).unwrap();
        assert!(std::fs::symlink_metadata(&link).unwrap().file_type().is_symlink());
        assert!(std::fs::read_to_string(&real).unwrap().contains("enabled = true"));
    }

    #[test]
    fn a_missing_file_is_empty() {
        let dir = tempfile::tempdir().unwrap();
        let config = HerdrConfig::load(&dir.path().join("none.toml")).unwrap();
        assert_eq!(config.str("ui.toast.delivery"), None);
    }
}
