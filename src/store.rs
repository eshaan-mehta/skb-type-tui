//! Settings, the confirmed layout, manual edits, and a per-revision layout cache, on disk.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::layout::{Edit, Layout};
use crate::stats::{Stats, Targets};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    /// Learn your layout a few letters at a time.
    Progressive,
    /// Practice the letters you're slowest or least accurate on.
    WeakKeys,
    /// Drill common letter sequences ("th", "ing", "tion") a few at a time.
    Ngrams,
    /// Random common words.
    Words,
}

impl Mode {
    pub const ALL: [Mode; 4] = [Mode::Progressive, Mode::WeakKeys, Mode::Ngrams, Mode::Words];

    pub fn name(self) -> &'static str {
        match self {
            Mode::Progressive => "progressive",
            Mode::WeakKeys => "weak keys",
            Mode::Ngrams => "n-grams",
            Mode::Words => "words",
        }
    }

    /// Whether the mode works from the letters on the user's layout.
    pub fn needs_layout(self) -> bool {
        matches!(self, Mode::Progressive | Mode::WeakKeys)
    }
}

/// The keyboard the app is set up for.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Keyboard {
    /// ZSA Voyager: its layout comes from Oryx, and it reports key presses and layers live.
    Voyager,
    /// Any other keyboard. Not supported yet (QMK and ZMK boards are planned).
    Other,
}

impl Keyboard {
    pub const ALL: [Keyboard; 2] = [Keyboard::Voyager, Keyboard::Other];

    pub fn name(self) -> &'static str {
        match self {
            Keyboard::Voyager => "ZSA Voyager",
            Keyboard::Other => "Other (currently unsupported)",
        }
    }

    pub fn supported(self) -> bool {
        self == Keyboard::Voyager
    }
}

/// The shape of the typing cursor.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CursorStyle {
    /// A thin bar before the next letter.
    Line,
    /// A box over the next letter.
    Block,
    /// A line under the next letter.
    Underscore,
}

impl CursorStyle {
    pub const ALL: [CursorStyle; 3] = [CursorStyle::Line, CursorStyle::Block, CursorStyle::Underscore];

    pub fn name(self) -> &'static str {
        match self {
            CursorStyle::Line => "line",
            CursorStyle::Block => "block",
            CursorStyle::Underscore => "underscore",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub mode: Mode,
    pub hints: bool,
    pub cursor: CursorStyle,
    /// End the test on the first wrong key.
    pub instant_death: bool,
    pub word_count: usize,
    /// Per-letter speed needed to unlock the next letter in progressive mode, and test speed
    /// needed to move on in n-grams mode.
    pub target_wpm: u32,
    /// Accuracy (percent) needed alongside the speed.
    pub target_accuracy: u32,
    /// Progressive tests a letter has to be the focus of before it can count as learned.
    pub focus_tests: u32,
    /// The user's own word list (absolute path); None uses the built-in words.
    pub word_file: Option<String>,
}

pub const TARGET_WPM_RANGE: std::ops::RangeInclusive<u32> = 10..=200;
pub const TARGET_ACCURACY_RANGE: std::ops::RangeInclusive<u32> = 50..=100;
pub const WORD_COUNT_RANGE: std::ops::RangeInclusive<u32> = 5..=500;
pub const FOCUS_TESTS_RANGE: std::ops::RangeInclusive<u32> = 0..=20;

impl Default for Settings {
    fn default() -> Self {
        Settings {
            mode: Mode::Progressive,
            hints: true,
            cursor: CursorStyle::Underscore,
            instant_death: false,
            word_count: 25,
            target_wpm: 30,
            target_accuracy: 95,
            focus_tests: 10,
            word_file: None,
        }
    }
}

impl Settings {
    pub fn targets(&self) -> Targets {
        let wpm = self.target_wpm.clamp(*TARGET_WPM_RANGE.start(), *TARGET_WPM_RANGE.end());
        let acc = self.target_accuracy.clamp(*TARGET_ACCURACY_RANGE.start(), *TARGET_ACCURACY_RANGE.end());
        let focus_tests = self.focus_tests.min(*FOCUS_TESTS_RANGE.end());
        Targets { wpm: wpm as f64, accuracy: acc as f64 / 100.0, focus_tests }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayoutRef {
    pub layout_id: String,
    pub revision_id: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Saved {
    pub settings: Settings,
    /// The keyboard the user picked; None until they do.
    pub keyboard: Option<Keyboard>,
    /// The layout revision the user last confirmed.
    pub confirmed: Option<LayoutRef>,
    /// The user's corrections, applied on top of whichever revision is confirmed.
    pub edits: Vec<Edit>,
}

impl Saved {
    /// Fill in what older saves didn't record.
    fn upgraded(mut self) -> Self {
        // A layout confirmed before there was a choice of keyboard was a Voyager's.
        if self.keyboard.is_none() && self.confirmed.is_some() {
            self.keyboard = Some(Keyboard::Voyager);
        }
        self
    }
}

pub struct Store {
    dir: PathBuf,
    pub saved: Saved,
}

/// The app's folder in the config directory.
const DIR_NAME: &str = "skb-type-tui";
/// What it was called before, when the app was zsa-typer-tui.
const OLD_DIR_NAME: &str = "zsa-typer-tui";

/// The app's folder in `config`, after moving the old one over if that's all there is, so
/// settings, layouts and stats carry over from before the rename.
fn app_dir(config: &Path) -> PathBuf {
    let dir = config.join(DIR_NAME);
    let old = config.join(OLD_DIR_NAME);
    if !dir.exists() && old.is_dir() {
        let _ = fs::rename(&old, &dir);
    }
    dir
}

impl Store {
    pub fn open() -> Self {
        let dir = app_dir(&dirs::config_dir().unwrap_or_else(|| PathBuf::from(".")));
        let saved = read_json::<Saved>(&dir.join("state.json")).unwrap_or_default().upgraded();
        Store { dir, saved }
    }

    pub fn save(&self) -> Result<()> {
        write_json(&self.dir.join("state.json"), &self.saved)
    }

    fn layout_path(&self, layout_id: &str, revision_id: &str) -> PathBuf {
        self.dir.join("layouts").join(format!("{layout_id}-{revision_id}.json"))
    }

    /// A previously fetched layout revision (without edits).
    pub fn cached_layout(&self, layout_id: &str, revision_id: &str) -> Option<Layout> {
        let layout: Layout = read_json(&self.layout_path(layout_id, revision_id))?;
        layout.validate().ok()?;
        Some(layout)
    }

    pub fn cache_layout(&self, layout: &Layout) -> Result<()> {
        write_json(&self.layout_path(&layout.layout_id, &layout.revision_id), layout)
    }

    pub fn load_stats(&self) -> Stats {
        read_json(&self.dir.join("stats.json")).unwrap_or_default()
    }

    pub fn save_stats(&self, stats: &Stats) -> Result<()> {
        write_json(&self.dir.join("stats.json"), stats)
    }

    /// The confirmed layout with the user's edits applied.
    pub fn confirmed_layout(&self) -> Option<Layout> {
        let r = self.saved.confirmed.as_ref()?;
        Some(self.cached_layout(&r.layout_id, &r.revision_id)?.with_edits(&self.saved.edits))
    }
}

fn read_json<T: DeserializeOwned>(path: &Path) -> Option<T> {
    serde_json::from_str(&fs::read_to_string(path).ok()?).ok()
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let parent = path.parent().context("bad path")?;
    fs::create_dir_all(parent).with_context(|| format!("couldn't create {}", parent.display()))?;
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, serde_json::to_string_pretty(value)?)?;
    fs::rename(&tmp, path).with_context(|| format!("couldn't write {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn older_saves_were_for_a_voyager() {
        let confirmed = Some(LayoutRef { layout_id: "a".into(), revision_id: "b".into() });
        assert_eq!(Saved { confirmed, ..Default::default() }.upgraded().keyboard, Some(Keyboard::Voyager));
        assert_eq!(Saved::default().upgraded().keyboard, None, "a fresh start asks");
    }

    #[test]
    fn data_moves_over_from_the_old_name() {
        let config = std::env::temp_dir().join(format!("skb-type-config-{}", std::process::id()));
        let _ = fs::remove_dir_all(&config);
        fs::create_dir_all(config.join(OLD_DIR_NAME)).unwrap();
        fs::write(config.join(OLD_DIR_NAME).join("stats.json"), "{}").unwrap();
        let dir = app_dir(&config);
        assert_eq!(dir, config.join(DIR_NAME));
        assert!(dir.join("stats.json").exists() && !config.join(OLD_DIR_NAME).exists());
        // Once there's a new folder, an old one is left alone.
        fs::create_dir_all(config.join(OLD_DIR_NAME)).unwrap();
        app_dir(&config);
        assert!(config.join(OLD_DIR_NAME).exists() && dir.join("stats.json").exists());
        fs::remove_dir_all(&config).unwrap();
    }
}
