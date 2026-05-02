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
    Amethyst,
    Atelier,
    Archive,
    Signal,
    TokyoMind,
    Monograph,
    TerminalNeon,
}

impl ThemeId {
    pub const ALL: [ThemeId; 12] = [
        ThemeId::Workbench,
        ThemeId::Paper,
        ThemeId::Blueprint,
        ThemeId::Calm,
        ThemeId::Violet,
        ThemeId::Amethyst,
        ThemeId::Atelier,
        ThemeId::Archive,
        ThemeId::Signal,
        ThemeId::TokyoMind,
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
            Self::Amethyst => "Amethyst",
            Self::Atelier => "Atelier",
            Self::Archive => "Archive",
            Self::Signal => "Signal",
            Self::TokyoMind => "Tokyo Mind",
            Self::Monograph => "Monograph",
            Self::TerminalNeon => "Terminal Neon",
        }
    }

    pub fn summary(self) -> &'static str {
        match self {
            Self::Workbench => {
                "A midnight workbench for everyday mapping, with teal structure, blue anchors, and amber metadata close at hand."
            }
            Self::Paper => {
                "A warm drafting page for writers and meeting notes, with ink-blue structure and sepia marginalia."
            }
            Self::Blueprint => {
                "A technical drafting table for engineers, with cool blue structure, cyan traces, and yellow field marks."
            }
            Self::Calm => {
                "A quiet moss-and-sand surface for brainstorming, decomposing, and staying with a plan without glare."
            }
            Self::Violet => {
                "A dusk-violet writing room for poets and nonlinear thinkers, with moonlit ids and soft gold annotations."
            }
            Self::Amethyst => {
                "A rich amethyst workspace with sea-glass accents, honey markers, and twilight-blue structure."
            }
            Self::Atelier => {
                "A graphite studio surface with chalk-white text, pewter structure, and quiet tape-and-clay accents."
            }
            Self::Archive => {
                "A warm archive surface with walnut depth, brass highlights, slate structure, and oxblood marks."
            }
            Self::Signal => {
                "A high-contrast control-room surface with black glass, white focus, cyan signals, amber warnings, and red reserved for danger."
            }
            Self::TokyoMind => {
                "A Tokyo Night-inspired editor surface with violet command energy, blue structure, green tags, orange metadata, and cyan links."
            }
            Self::Monograph => {
                "A restrained graphite desk for long focus, with silver structure, cool anchors, and almost no decorative noise."
            }
            Self::TerminalNeon => {
                "A dark terminal cockpit with neon green syntax, electric cyan links, and a bright command pulse."
            }
        }
    }

    pub fn keywords(self) -> &'static str {
        match self {
            Self::Workbench => "theme workbench navy cyan amber default planning project manager",
            Self::Paper => "theme paper light cream ink warm writer meeting notes drafting",
            Self::Blueprint => {
                "theme blueprint blue technical grid engineer decomposer architecture"
            }
            Self::Calm => "theme calm sage sand gentle brainstorming decomposing meeting notes",
            Self::Violet => "theme violet purple lavender orchid gold dusk poet writer nonlinear",
            Self::Amethyst => "theme amethyst purple sea-glass honey twilight jewel",
            Self::Atelier => {
                "theme atelier graphite charcoal pewter chalk tape clay studio neutral writer"
            }
            Self::Archive => {
                "theme archive walnut brass slate oxblood research scientist library warm"
            }
            Self::Signal => {
                "theme signal control room black glass white amber red high contrast project manager ops"
            }
            Self::TokyoMind => "theme tokyo mind night vscode editor blue purple green orange cyan",
            Self::Monograph => "theme monograph graphite minimal monochrome slate quiet focus",
            Self::TerminalNeon => "theme terminal neon green cyan dark software engineer cockpit",
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
                tag: Color::Rgb(67, 201, 176),
                metadata: Color::Rgb(248, 189, 94),
                id: Color::Rgb(94, 191, 255),
                query: Color::Rgb(158, 206, 255),
                attention: Color::Rgb(120, 224, 205),
                relation: Color::Rgb(67, 201, 176),
                count: Color::Rgb(94, 191, 255),
                selection: Color::Rgb(24, 39, 58),
                selection_text: Color::Rgb(233, 241, 248),
                text: Color::Rgb(233, 241, 248),
                muted: Color::Rgb(129, 153, 178),
            },
            Self::Paper => Theme {
                background: Color::Rgb(246, 243, 236),
                surface: Color::Rgb(255, 253, 247),
                surface_alt: Color::Rgb(233, 227, 215),
                border: Color::Rgb(118, 132, 148),
                accent: Color::Rgb(37, 86, 124),
                sky: Color::Rgb(70, 126, 164),
                warn: Color::Rgb(174, 116, 55),
                danger: Color::Rgb(179, 79, 66),
                tag: Color::Rgb(37, 86, 124),
                metadata: Color::Rgb(128, 84, 45),
                id: Color::Rgb(58, 109, 145),
                query: Color::Rgb(42, 58, 80),
                attention: Color::Rgb(76, 115, 144),
                relation: Color::Rgb(58, 109, 145),
                count: Color::Rgb(70, 126, 164),
                selection: Color::Rgb(233, 227, 215),
                selection_text: Color::Rgb(31, 36, 43),
                text: Color::Rgb(31, 36, 43),
                muted: Color::Rgb(96, 103, 114),
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
                tag: Color::Rgb(87, 217, 255),
                metadata: Color::Rgb(255, 197, 87),
                id: Color::Rgb(163, 232, 255),
                query: Color::Rgb(135, 215, 255),
                attention: Color::Rgb(87, 217, 255),
                relation: Color::Rgb(87, 217, 255),
                count: Color::Rgb(163, 232, 255),
                selection: Color::Rgb(16, 49, 86),
                selection_text: Color::Rgb(226, 242, 255),
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
                tag: Color::Rgb(142, 191, 162),
                metadata: Color::Rgb(221, 184, 120),
                id: Color::Rgb(148, 203, 214),
                query: Color::Rgb(179, 204, 182),
                attention: Color::Rgb(166, 214, 188),
                relation: Color::Rgb(148, 203, 214),
                count: Color::Rgb(179, 204, 182),
                selection: Color::Rgb(46, 61, 55),
                selection_text: Color::Rgb(233, 238, 230),
                text: Color::Rgb(233, 238, 230),
                muted: Color::Rgb(157, 173, 162),
            },
            Self::Violet => Theme {
                background: Color::Rgb(45, 30, 76),
                surface: Color::Rgb(67, 44, 108),
                surface_alt: Color::Rgb(88, 57, 137),
                border: Color::Rgb(145, 112, 190),
                accent: Color::Rgb(221, 188, 255),
                sky: Color::Rgb(162, 196, 255),
                warn: Color::Rgb(236, 187, 95),
                danger: Color::Rgb(255, 132, 178),
                tag: Color::Rgb(229, 197, 255),
                metadata: Color::Rgb(236, 187, 95),
                id: Color::Rgb(162, 196, 255),
                query: Color::Rgb(238, 222, 255),
                attention: Color::Rgb(221, 188, 255),
                relation: Color::Rgb(162, 196, 255),
                count: Color::Rgb(229, 197, 255),
                selection: Color::Rgb(88, 57, 137),
                selection_text: Color::Rgb(248, 237, 255),
                text: Color::Rgb(248, 237, 255),
                muted: Color::Rgb(194, 172, 220),
            },
            Self::Amethyst => Theme {
                background: Color::Rgb(29, 20, 47),
                surface: Color::Rgb(52, 34, 84),
                surface_alt: Color::Rgb(82, 52, 132),
                border: Color::Rgb(139, 116, 190),
                accent: Color::Rgb(82, 214, 190),
                sky: Color::Rgb(118, 178, 255),
                warn: Color::Rgb(244, 185, 82),
                danger: Color::Rgb(255, 122, 158),
                tag: Color::Rgb(82, 214, 190),
                metadata: Color::Rgb(244, 185, 82),
                id: Color::Rgb(118, 178, 255),
                query: Color::Rgb(211, 185, 255),
                attention: Color::Rgb(194, 164, 255),
                relation: Color::Rgb(82, 214, 190),
                count: Color::Rgb(118, 178, 255),
                selection: Color::Rgb(82, 52, 132),
                selection_text: Color::Rgb(250, 243, 255),
                text: Color::Rgb(250, 243, 255),
                muted: Color::Rgb(184, 168, 211),
            },
            Self::Atelier => Theme {
                background: Color::Rgb(31, 29, 29),
                surface: Color::Rgb(49, 48, 49),
                surface_alt: Color::Rgb(72, 70, 70),
                border: Color::Rgb(127, 129, 130),
                accent: Color::Rgb(236, 230, 215),
                sky: Color::Rgb(160, 170, 181),
                warn: Color::Rgb(198, 173, 117),
                danger: Color::Rgb(196, 132, 123),
                tag: Color::Rgb(218, 207, 184),
                metadata: Color::Rgb(190, 158, 96),
                id: Color::Rgb(144, 164, 176),
                query: Color::Rgb(232, 224, 207),
                attention: Color::Rgb(205, 190, 156),
                relation: Color::Rgb(144, 164, 176),
                count: Color::Rgb(190, 158, 96),
                selection: Color::Rgb(72, 70, 70),
                selection_text: Color::Rgb(241, 238, 230),
                text: Color::Rgb(241, 238, 230),
                muted: Color::Rgb(157, 151, 143),
            },
            Self::Archive => Theme {
                background: Color::Rgb(42, 34, 29),
                surface: Color::Rgb(61, 50, 43),
                surface_alt: Color::Rgb(86, 72, 61),
                border: Color::Rgb(139, 119, 91),
                accent: Color::Rgb(219, 186, 124),
                sky: Color::Rgb(142, 166, 175),
                warn: Color::Rgb(205, 146, 74),
                danger: Color::Rgb(178, 82, 76),
                tag: Color::Rgb(219, 186, 124),
                metadata: Color::Rgb(190, 132, 70),
                id: Color::Rgb(130, 172, 172),
                query: Color::Rgb(224, 200, 150),
                attention: Color::Rgb(211, 174, 112),
                relation: Color::Rgb(130, 172, 172),
                count: Color::Rgb(219, 186, 124),
                selection: Color::Rgb(86, 72, 61),
                selection_text: Color::Rgb(242, 225, 196),
                text: Color::Rgb(242, 225, 196),
                muted: Color::Rgb(173, 151, 123),
            },
            Self::Signal => Theme {
                background: Color::Rgb(4, 6, 9),
                surface: Color::Rgb(17, 19, 24),
                surface_alt: Color::Rgb(31, 34, 41),
                border: Color::Rgb(83, 91, 104),
                accent: Color::Rgb(238, 243, 246),
                sky: Color::Rgb(98, 198, 219),
                warn: Color::Rgb(255, 176, 52),
                danger: Color::Rgb(255, 72, 88),
                tag: Color::Rgb(98, 198, 219),
                metadata: Color::Rgb(255, 176, 52),
                id: Color::Rgb(238, 243, 246),
                query: Color::Rgb(174, 238, 249),
                attention: Color::Rgb(132, 222, 235),
                relation: Color::Rgb(98, 198, 219),
                count: Color::Rgb(238, 243, 246),
                selection: Color::Rgb(31, 34, 41),
                selection_text: Color::Rgb(226, 232, 236),
                text: Color::Rgb(226, 232, 236),
                muted: Color::Rgb(124, 134, 145),
            },
            Self::TokyoMind => Theme {
                background: Color::Rgb(26, 27, 38),
                surface: Color::Rgb(36, 40, 59),
                surface_alt: Color::Rgb(48, 54, 82),
                border: Color::Rgb(65, 72, 104),
                accent: Color::Rgb(187, 154, 247),
                sky: Color::Rgb(122, 162, 247),
                warn: Color::Rgb(224, 175, 104),
                danger: Color::Rgb(247, 118, 142),
                tag: Color::Rgb(158, 206, 106),
                metadata: Color::Rgb(255, 158, 100),
                id: Color::Rgb(125, 207, 255),
                query: Color::Rgb(187, 154, 247),
                attention: Color::Rgb(203, 166, 247),
                relation: Color::Rgb(115, 218, 202),
                count: Color::Rgb(192, 202, 245),
                selection: Color::Rgb(44, 60, 104),
                selection_text: Color::Rgb(192, 202, 245),
                text: Color::Rgb(192, 202, 245),
                muted: Color::Rgb(86, 95, 137),
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
                tag: Color::Rgb(191, 198, 206),
                metadata: Color::Rgb(181, 169, 151),
                id: Color::Rgb(151, 178, 199),
                query: Color::Rgb(222, 226, 232),
                attention: Color::Rgb(185, 197, 210),
                relation: Color::Rgb(151, 178, 199),
                count: Color::Rgb(191, 198, 206),
                selection: Color::Rgb(29, 33, 39),
                selection_text: Color::Rgb(231, 235, 240),
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
                tag: Color::Rgb(67, 245, 162),
                metadata: Color::Rgb(255, 213, 92),
                id: Color::Rgb(64, 219, 255),
                query: Color::Rgb(188, 255, 120),
                attention: Color::Rgb(103, 255, 188),
                relation: Color::Rgb(64, 219, 255),
                count: Color::Rgb(255, 213, 92),
                selection: Color::Rgb(17, 33, 29),
                selection_text: Color::Rgb(215, 255, 236),
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
