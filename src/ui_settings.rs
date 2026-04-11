use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Deserializer, Serialize};

use crate::app::AppError;
use crate::mindmap::Theme;
use ratatui::prelude::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum ThemeId {
    #[default]
    Workbench,
    Paper,
    Blueprint,
    Calm,
    Violet,
    Monograph,
    TerminalNeon,
}

impl ThemeId {
    pub const ALL: [ThemeId; 7] = [
        ThemeId::Workbench,
        ThemeId::Paper,
        ThemeId::Blueprint,
        ThemeId::Calm,
        ThemeId::Violet,
        ThemeId::Monograph,
        ThemeId::TerminalNeon,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Workbench => "Workbench",
            Self::Paper => "Paper",
            Self::Blueprint => "Blueprint",
            Self::Calm => "Calm",
            Self::Violet => "Violet",
            Self::Monograph => "Monograph",
            Self::TerminalNeon => "Terminal Neon",
        }
    }

    pub fn summary(self) -> &'static str {
        match self {
            Self::Workbench => "Deep navy workspace with cyan and amber accents.",
            Self::Paper => "Light drafting surface with ink-blue structure and warm markers.",
            Self::Blueprint => "Cool blueprint grid feel with bright technical highlights.",
            Self::Calm => "Soft sage and sand palette for low-friction planning sessions.",
            Self::Violet => {
                "Lavender-on-violet surface with orchid borders and warm gold highlights."
            }
            Self::Monograph => {
                "Restrained graphite surface with quiet silver structure and cool focus cues."
            }
            Self::TerminalNeon => "Dark terminal glass with vivid green and electric cyan edges.",
        }
    }

    pub fn keywords(self) -> &'static str {
        match self {
            Self::Workbench => "theme workbench navy cyan amber default",
            Self::Paper => "theme paper light cream ink warm",
            Self::Blueprint => "theme blueprint blue technical grid",
            Self::Calm => "theme calm sage sand gentle",
            Self::Violet => "theme violet purple lavender orchid gold dusk",
            Self::Monograph => "theme monograph graphite minimal monochrome slate quiet",
            Self::TerminalNeon => "theme terminal neon green cyan dark",
        }
    }

    pub fn theme(self) -> Theme {
        match self {
            Self::Workbench => Theme {
                background: Color::Rgb(8, 15, 24),
                surface: Color::Rgb(15, 25, 38),
                surface_alt: Color::Rgb(24, 39, 58),
                border: Color::Rgb(41, 65, 91),
                accent: Color::Rgb(67, 201, 176),
                sky: Color::Rgb(94, 191, 255),
                warn: Color::Rgb(248, 189, 94),
                danger: Color::Rgb(244, 114, 93),
                text: Color::Rgb(233, 241, 248),
                muted: Color::Rgb(129, 153, 178),
            },
            Self::Paper => Theme {
                background: Color::Rgb(244, 239, 229),
                surface: Color::Rgb(252, 249, 243),
                surface_alt: Color::Rgb(232, 224, 210),
                border: Color::Rgb(123, 135, 153),
                accent: Color::Rgb(34, 88, 140),
                sky: Color::Rgb(78, 140, 181),
                warn: Color::Rgb(193, 129, 51),
                danger: Color::Rgb(179, 79, 66),
                text: Color::Rgb(34, 38, 46),
                muted: Color::Rgb(103, 110, 123),
            },
            Self::Blueprint => Theme {
                background: Color::Rgb(5, 24, 46),
                surface: Color::Rgb(10, 36, 66),
                surface_alt: Color::Rgb(16, 49, 86),
                border: Color::Rgb(66, 109, 166),
                accent: Color::Rgb(87, 217, 255),
                sky: Color::Rgb(163, 232, 255),
                warn: Color::Rgb(255, 197, 87),
                danger: Color::Rgb(255, 117, 117),
                text: Color::Rgb(226, 242, 255),
                muted: Color::Rgb(126, 169, 214),
            },
            Self::Calm => Theme {
                background: Color::Rgb(22, 31, 27),
                surface: Color::Rgb(33, 45, 40),
                surface_alt: Color::Rgb(46, 61, 55),
                border: Color::Rgb(92, 117, 105),
                accent: Color::Rgb(142, 191, 162),
                sky: Color::Rgb(148, 203, 214),
                warn: Color::Rgb(221, 184, 120),
                danger: Color::Rgb(208, 120, 110),
                text: Color::Rgb(233, 238, 230),
                muted: Color::Rgb(157, 173, 162),
            },
            Self::Violet => Theme {
                background: Color::Rgb(67, 36, 104),
                surface: Color::Rgb(89, 48, 134),
                surface_alt: Color::Rgb(106, 61, 156),
                border: Color::Rgb(166, 124, 214),
                accent: Color::Rgb(226, 191, 255),
                sky: Color::Rgb(203, 168, 255),
                warn: Color::Rgb(246, 201, 112),
                danger: Color::Rgb(255, 132, 178),
                text: Color::Rgb(245, 230, 255),
                muted: Color::Rgb(205, 181, 228),
            },
            Self::Monograph => Theme {
                background: Color::Rgb(14, 16, 20),
                surface: Color::Rgb(20, 23, 28),
                surface_alt: Color::Rgb(29, 33, 39),
                border: Color::Rgb(76, 84, 94),
                accent: Color::Rgb(191, 198, 206),
                sky: Color::Rgb(143, 175, 201),
                warn: Color::Rgb(202, 173, 124),
                danger: Color::Rgb(196, 121, 121),
                text: Color::Rgb(231, 235, 240),
                muted: Color::Rgb(132, 141, 151),
            },
            Self::TerminalNeon => Theme {
                background: Color::Rgb(6, 11, 10),
                surface: Color::Rgb(11, 20, 18),
                surface_alt: Color::Rgb(17, 33, 29),
                border: Color::Rgb(34, 84, 67),
                accent: Color::Rgb(67, 245, 162),
                sky: Color::Rgb(64, 219, 255),
                warn: Color::Rgb(255, 213, 92),
                danger: Color::Rgb(255, 107, 107),
                text: Color::Rgb(215, 255, 236),
                muted: Color::Rgb(116, 165, 145),
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UiSettings {
    pub theme: ThemeId,
    pub motion_enabled: bool,
    pub ascii_accents: bool,
    pub minimal_mode: bool,
    pub reading_mode: bool,
}

#[derive(Debug, Deserialize)]
struct RawUiSettings {
    theme: Option<ThemeId>,
    motion_enabled: Option<bool>,
    reduced_motion: Option<bool>,
    ascii_accents: Option<bool>,
    minimal_mode: Option<bool>,
    reading_mode: Option<bool>,
}

impl<'de> Deserialize<'de> for UiSettings {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = RawUiSettings::deserialize(deserializer)?;
        Ok(Self {
            theme: raw.theme.unwrap_or(ThemeId::Workbench),
            motion_enabled: raw
                .motion_enabled
                .unwrap_or_else(|| raw.reduced_motion.map(|reduced| !reduced).unwrap_or(true)),
            ascii_accents: raw.ascii_accents.unwrap_or(false),
            minimal_mode: raw.minimal_mode.unwrap_or(false),
            reading_mode: raw.reading_mode.unwrap_or(false),
        })
    }
}

impl Default for UiSettings {
    fn default() -> Self {
        Self {
            theme: ThemeId::Workbench,
            motion_enabled: true,
            ascii_accents: false,
            minimal_mode: false,
            reading_mode: false,
        }
    }
}

pub fn ui_settings_path_for(map_path: &Path) -> Result<PathBuf, AppError> {
    let file_name = map_path.file_name().ok_or_else(|| {
        AppError::new(format!(
            "Could not derive a UI settings file path from '{}'.",
            map_path.display()
        ))
    })?;

    let settings_name = format!(".{}.mdmind-ui.json", file_name.to_string_lossy());
    Ok(match map_path.parent() {
        Some(parent) => parent.join(settings_name),
        None => PathBuf::from(settings_name),
    })
}

pub fn load_ui_settings_for(map_path: &Path) -> Result<UiSettings, AppError> {
    let settings_path = ui_settings_path_for(map_path)?;
    if !settings_path.exists() {
        return Ok(UiSettings::default());
    }

    let contents = fs::read_to_string(&settings_path).map_err(|error| {
        AppError::new(format!(
            "Could not read UI settings '{}': {error}",
            settings_path.display()
        ))
    })?;

    serde_json::from_str(&contents).map_err(|error| {
        AppError::new(format!(
            "Could not parse UI settings '{}': {error}",
            settings_path.display()
        ))
    })
}

pub fn save_ui_settings_for(map_path: &Path, settings: &UiSettings) -> Result<(), AppError> {
    let settings_path = ui_settings_path_for(map_path)?;
    let contents = serde_json::to_string_pretty(settings).expect("ui settings should serialize");
    fs::write(&settings_path, contents).map_err(|error| {
        AppError::new(format!(
            "Could not write UI settings '{}': {error}",
            settings_path.display()
        ))
    })
}
