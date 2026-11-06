//! The Tools panel's settings and actions, t3code's `DeviceActions`: one typed action per control, and each value read
//! back from the device after, never kept here. What a device can't read is left out, so the panel shows only what
//! its platform has.
use super::{Device, Platform, Probe};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextSize {
    Small,
    Default,
    Large,
    ExtraLarge,
}

pub const TEXT_SIZES: [(TextSize, &str); 4] = [(TextSize::Small, "Small"), (TextSize::Default, "Default"), (TextSize::Large, "Large"), (TextSize::ExtraLarge, "Extra large")];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Toggle {
    ReduceMotion,
    IncreaseContrast,
    ReduceTransparency,
    ShowBorders,
    VoiceOver,
    /// Android's Wi-Fi and mobile data together.
    Network,
}

impl Toggle {
    pub fn label(self) -> &'static str {
        match self {
            Toggle::ReduceMotion => "Reduce Motion",
            Toggle::IncreaseContrast => "Increase Contrast",
            Toggle::ReduceTransparency => "Reduce Transparency",
            Toggle::ShowBorders => "Show Borders",
            Toggle::VoiceOver => "VoiceOver",
            Toggle::Network => "Network",
        }
    }
}

/// iOS's Liquid Glass looks (iOS 26 and later).
pub const LIQUID_GLASS: [(&str, &str); 2] = [("clear", "Clear"), ("tinted", "Tinted")];

pub const COLOR_FILTERS: [(&str, &str); 5] = [
    ("none", "None"),
    ("grayscale", "Grayscale"),
    ("red-green", "Red / green (protanopia)"),
    ("green-red", "Green / red (deuteranopia)"),
    ("blue-yellow", "Blue / yellow (tritanopia)"),
];

/// How a device is held, by t3code's names, each a quarter turn clockwise on from the last (`Frame::turn`): held
/// landscape left, its top is on the left, as Simulator.app's Rotate Left turns it.
pub const ORIENTATIONS: [(&str, &str); 4] =
    [("portrait", "Portrait"), ("landscape_right", "Landscape right"), ("portrait_upside_down", "Upside down"), ("landscape_left", "Landscape left")];

pub const LOCATIONS: [(&str, f64, f64); 5] = [
    ("San Francisco", 37.7749, -122.4194),
    ("New York", 40.7128, -74.006),
    ("London", 51.5074, -0.1278),
    ("Stockholm", 59.3293, 18.0686),
    ("Tokyo", 35.6762, 139.6503),
];

/// The permissions a platform grants, by t3code's names, with their labels.
pub fn permissions(platform: Platform) -> &'static [(&'static str, &'static str)] {
    match platform {
        // ponytail: notifications left out, simctl has no service for it (serve-sim edits the BulletinBoard plist)
        Platform::Ios => &[
            ("camera", "Camera"),
            ("microphone", "Microphone"),
            ("photos", "Photos"),
            ("contacts", "Contacts"),
            ("calendar", "Calendar"),
            ("reminders", "Reminders"),
            ("location", "Location"),
            ("motion", "Motion"),
            ("media-library", "Media library"),
            ("faceid", "Face ID"),
        ],
        Platform::Android => &[
            ("camera", "Camera"),
            ("microphone", "Microphone"),
            ("photos", "Photos"),
            ("contacts", "Contacts"),
            ("calendar", "Calendar"),
            ("location", "Location"),
            ("notifications", "Notifications"),
            ("motion", "Physical activity"),
        ],
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decision {
    Grant,
    Revoke,
    /// iOS's: back to asking.
    Reset,
}

impl Decision {
    pub fn verb(self) -> &'static str {
        match self {
            Decision::Grant => "grant",
            Decision::Revoke => "revoke",
            Decision::Reset => "reset",
        }
    }
}

/// The device's settings as it reads them now: `None` (or a toggle left out) where it can't say.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Settings {
    pub dark: Option<bool>,
    pub text_size: Option<TextSize>,
    /// The switches it has, in the panel's order.
    pub toggles: Vec<(Toggle, bool)>,
    pub liquid_glass: Option<&'static str>,
    pub color_filter: Option<&'static str>,
    /// The app in front: its bundle id or package.
    pub app: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    Dark(bool),
    TextSize(TextSize),
    Toggle(Toggle, bool),
    LiquidGlass(&'static str),
    ColorFilter(&'static str),
    /// Android's, by `ORIENTATIONS`' names (a simulator turns through its `Session::rotate`).
    Orientation(&'static str),
    Location(f64, f64),
    /// iOS's; an emulator keeps its last fix.
    ClearLocation,
    Permission { app: String, permission: &'static str, decision: Decision },
    OpenUrl(String),
    Launch(String),
    Terminate(String),
    /// iOS's: an alert pushed to `app`.
    Push { app: String, text: String },
}

/// Reads `device`'s settings, all at once.
pub fn read(device: &Device, probe: &Probe) -> Settings {
    match device.platform {
        #[cfg(target_os = "macos")]
        Platform::Ios => super::ios::read_tools(device, probe),
        Platform::Android => super::android::read_tools(&device.id),
        #[allow(unreachable_patterns)]
        _ => {
            let _ = probe;
            Settings::default()
        }
    }
}

pub fn act(device: &Device, action: &Action) -> anyhow::Result<()> {
    match device.platform {
        #[cfg(target_os = "macos")]
        Platform::Ios => super::ios::act(&device.id, action),
        Platform::Android => super::android::act(&device.id, action),
        #[allow(unreachable_patterns)]
        _ => anyhow::bail!("iOS Simulators need macOS"),
    }
}

/// The frontmost app's accessibility elements, for the overlay: each one's frame as `[x, y, width, height]`, 0 to 1
/// across the screen.
pub fn elements(device: &Device, probe: &Probe) -> anyhow::Result<Vec<[f64; 4]>> {
    match device.platform {
        #[cfg(target_os = "macos")]
        Platform::Ios => super::ios::elements(probe),
        Platform::Android => super::android::elements(&device.id),
        #[allow(unreachable_patterns)]
        _ => {
            let _ = probe;
            anyhow::bail!("iOS Simulators need macOS")
        }
    }
}

/// iOS's `content_size` categories for the panel's sizes, as t3code maps them.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub(super) fn ios_text_size(size: TextSize) -> &'static str {
    match size {
        TextSize::Small => "small",
        TextSize::Default => "large",
        TextSize::Large => "extra-extra-large",
        TextSize::ExtraLarge => "accessibility-large",
    }
}

#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub(super) fn text_size_from_ios(category: &str) -> Option<TextSize> {
    Some(match category {
        "unknown" | "unsupported" | "" => return None,
        "large" => TextSize::Default,
        c if c.starts_with("accessibility") => TextSize::ExtraLarge,
        "extra-small" | "small" | "medium" => TextSize::Small,
        _ => TextSize::Large,
    })
}

pub(super) fn android_font_scale(size: TextSize) -> &'static str {
    match size {
        TextSize::Small => "0.85",
        TextSize::Default => "1.0",
        TextSize::Large => "1.15",
        TextSize::ExtraLarge => "1.3",
    }
}

pub(super) fn text_size_from_android(scale: f64) -> TextSize {
    match scale {
        s if s <= 0.9 => TextSize::Small,
        s if s >= 1.25 => TextSize::ExtraLarge,
        s if s >= 1.1 => TextSize::Large,
        _ => TextSize::Default,
    }
}

/// A word as `adb shell` passes it on: it joins its arguments unquoted for the device's shell.
pub(super) fn shell_word(word: &str) -> String {
    let plain = !word.is_empty() && word.chars().all(|c| c.is_ascii_alphanumeric() || "_@%+=:,./-".contains(c));
    if plain { word.to_string() } else { format!("'{}'", word.replace('\'', r"'\''")) }
}

/// A `&'static` name from `table` for what a device printed.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub(super) fn known(table: &[(&'static str, &'static str)], value: &str) -> Option<&'static str> {
    table.iter().map(|(v, _)| *v).find(|v| *v == value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_sizes_read_back_as_they_were_set() {
        for (size, _) in TEXT_SIZES {
            assert_eq!(text_size_from_ios(ios_text_size(size)), Some(size));
            assert_eq!(text_size_from_android(android_font_scale(size).parse().unwrap()), size);
        }
        assert_eq!(text_size_from_ios("unsupported"), None);
    }

    #[test]
    fn adb_shell_words_are_quoted() {
        assert_eq!(shell_word("com.example.app"), "com.example.app");
        assert_eq!(shell_word("https://x.dev/a?b=1&c=2"), "'https://x.dev/a?b=1&c=2'");
        assert_eq!(shell_word("it's"), r"'it'\''s'");
    }
}
