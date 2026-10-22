//! The edit model: every adjustment a photo can carry.
//!
//! Everything is plain data with `serde` defaults, so presets and sidecar
//! files may contain only the fields they care about.

use serde::{Deserialize, Serialize};

/// A control point on a tone curve. Both axes are normalised to `0..=1`.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

/// Which tone curve channel.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CurveChannel {
    Master,
    Red,
    Green,
    Blue,
}

impl CurveChannel {
    pub const ALL: [CurveChannel; 4] = [Self::Master, Self::Red, Self::Green, Self::Blue];

    pub fn label(self) -> &'static str {
        match self {
            Self::Master => "RGB",
            Self::Red => "Red",
            Self::Green => "Green",
            Self::Blue => "Blue",
        }
    }
}

fn linear_curve() -> Vec<Point> {
    vec![Point::new(0.0, 0.0), Point::new(1.0, 1.0)]
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Curves {
    pub master: Vec<Point>,
    pub red: Vec<Point>,
    pub green: Vec<Point>,
    pub blue: Vec<Point>,
}

impl Default for Curves {
    fn default() -> Self {
        Self { master: linear_curve(), red: linear_curve(), green: linear_curve(), blue: linear_curve() }
    }
}

impl Curves {
    pub fn get(&self, ch: CurveChannel) -> &Vec<Point> {
        match ch {
            CurveChannel::Master => &self.master,
            CurveChannel::Red => &self.red,
            CurveChannel::Green => &self.green,
            CurveChannel::Blue => &self.blue,
        }
    }

    pub fn get_mut(&mut self, ch: CurveChannel) -> &mut Vec<Point> {
        match ch {
            CurveChannel::Master => &mut self.master,
            CurveChannel::Red => &mut self.red,
            CurveChannel::Green => &mut self.green,
            CurveChannel::Blue => &mut self.blue,
        }
    }

    pub fn is_identity(&self) -> bool {
        CurveChannel::ALL.iter().all(|c| crate::curve::is_identity(self.get(*c)))
    }
}

/// The eight colour bands of the HSL mixer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Band {
    Red,
    Orange,
    Yellow,
    Green,
    Aqua,
    Blue,
    Purple,
    Magenta,
}

impl Band {
    pub const ALL: [Band; 8] =
        [Self::Red, Self::Orange, Self::Yellow, Self::Green, Self::Aqua, Self::Blue, Self::Purple, Self::Magenta];

    /// Centre hue of the band in degrees. Bands blend smoothly between
    /// neighbouring centres so there are no hard edges in gradients.
    pub const CENTERS: [f32; 8] = [0.0, 30.0, 60.0, 115.0, 175.0, 225.0, 270.0, 315.0];

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Red => "Red",
            Self::Orange => "Orange",
            Self::Yellow => "Yellow",
            Self::Green => "Green",
            Self::Aqua => "Aqua",
            Self::Blue => "Blue",
            Self::Purple => "Purple",
            Self::Magenta => "Magenta",
        }
    }

    /// Swatch colour for the UI.
    pub fn swatch(self) -> &'static str {
        match self {
            Self::Red => "#e5484d",
            Self::Orange => "#f76b15",
            Self::Yellow => "#ffc53d",
            Self::Green => "#46a758",
            Self::Aqua => "#12a594",
            Self::Blue => "#3e63dd",
            Self::Purple => "#8e4ec6",
            Self::Magenta => "#d6409f",
        }
    }
}

/// Per-band hue / saturation / luminance shift, each `-100..=100`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct HslShift {
    pub hue: f32,
    pub saturation: f32,
    pub luminance: f32,
}

impl HslShift {
    pub fn is_zero(&self) -> bool {
        self.hue == 0.0 && self.saturation == 0.0 && self.luminance == 0.0
    }
}

/// Tonal zones used by colour grading.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Zone {
    Shadows,
    Midtones,
    Highlights,
}

impl Zone {
    pub const ALL: [Zone; 3] = [Self::Shadows, Self::Midtones, Self::Highlights];

    pub fn label(self) -> &'static str {
        match self {
            Self::Shadows => "Shadows",
            Self::Midtones => "Midtones",
            Self::Highlights => "Highlights",
        }
    }
}

/// A colour wheel value: hue `0..360`, saturation `0..=100`, luminance `-100..=100`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Wheel {
    pub hue: f32,
    pub saturation: f32,
    pub luminance: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ColorGrading {
    pub shadows: Wheel,
    pub midtones: Wheel,
    pub highlights: Wheel,
    /// `-100..=100`, moves the shadow/highlight split point.
    pub balance: f32,
}

impl ColorGrading {
    pub fn zone(&self, z: Zone) -> &Wheel {
        match z {
            Zone::Shadows => &self.shadows,
            Zone::Midtones => &self.midtones,
            Zone::Highlights => &self.highlights,
        }
    }

    pub fn zone_mut(&mut self, z: Zone) -> &mut Wheel {
        match z {
            Zone::Shadows => &mut self.shadows,
            Zone::Midtones => &mut self.midtones,
            Zone::Highlights => &mut self.highlights,
        }
    }

    pub fn is_neutral(&self) -> bool {
        Zone::ALL.iter().all(|z| {
            let w = self.zone(*z);
            w.saturation == 0.0 && w.luminance == 0.0
        })
    }
}

/// Normalised rectangle (`0..=1` on both axes).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub const FULL: Rect = Rect { x: 0.0, y: 0.0, w: 1.0, h: 1.0 };

    pub fn center(&self) -> (f32, f32) {
        (self.x + self.w * 0.5, self.y + self.h * 0.5)
    }
}

impl Default for Rect {
    fn default() -> Self {
        Self::FULL
    }
}

/// Crop, rotation and flips. Not touched by presets.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Geometry {
    /// Quarter turns clockwise, `0..=3`.
    pub quarter_turns: u8,
    pub flip_h: bool,
    pub flip_v: bool,
    /// Fine rotation in degrees, `-45..=45`.
    pub straighten: f32,
    /// Crop rectangle in the rotated + straightened frame.
    pub crop: Rect,
    /// Locked aspect ratio (width / height of the crop in pixels), if any.
    pub aspect: Option<f32>,
}

impl Geometry {
    pub fn is_identity(&self) -> bool {
        *self == Geometry::default()
    }
}

/// Every adjustment applied to a photo.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Adjustments {
    // Light
    /// Stops, `-5..=5`.
    pub exposure: f32,
    pub contrast: f32,
    pub highlights: f32,
    pub shadows: f32,
    pub whites: f32,
    pub blacks: f32,
    // Colour
    pub temperature: f32,
    pub tint: f32,
    pub vibrance: f32,
    pub saturation: f32,
    // Effects / detail
    pub clarity: f32,
    pub dehaze: f32,
    pub sharpen: f32,
    pub vignette: f32,
    pub vignette_feather: f32,
    pub grain: f32,
    pub grain_size: f32,
    // Structured
    pub hsl: [HslShift; 8],
    pub grading: ColorGrading,
    pub curves: Curves,
    pub geometry: Geometry,
}

impl Default for Adjustments {
    fn default() -> Self {
        Self {
            exposure: 0.0,
            contrast: 0.0,
            highlights: 0.0,
            shadows: 0.0,
            whites: 0.0,
            blacks: 0.0,
            temperature: 0.0,
            tint: 0.0,
            vibrance: 0.0,
            saturation: 0.0,
            clarity: 0.0,
            dehaze: 0.0,
            sharpen: 0.0,
            vignette: 0.0,
            vignette_feather: 50.0,
            grain: 0.0,
            grain_size: 25.0,
            hsl: [HslShift::default(); 8],
            grading: ColorGrading::default(),
            curves: Curves::default(),
            geometry: Geometry::default(),
        }
    }
}

impl Adjustments {
    /// Replace every tonal/colour setting with `look`'s, keeping this photo's geometry.
    pub fn with_look(&self, look: &Adjustments) -> Adjustments {
        Adjustments { geometry: self.geometry, ..look.clone() }
    }

    /// True if the photo would render exactly like the original (ignoring geometry).
    pub fn is_neutral(&self) -> bool {
        self.with_look(&Adjustments::default()) == Adjustments { geometry: self.geometry, ..self.clone() }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    pub fn from_json(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
    }
}

/// How a slider track should be painted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Track {
    Plain,
    Temperature,
    Tint,
    Hue,
    Saturation,
}

/// Static description of a scalar slider.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParamSpec {
    pub label: &'static str,
    pub min: f32,
    pub max: f32,
    pub default: f32,
    pub step: f32,
    pub decimals: u8,
    pub track: Track,
}

/// Every scalar slider in the editor, so the UI and CLI can be generated from one list.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Param {
    Exposure,
    Contrast,
    Highlights,
    Shadows,
    Whites,
    Blacks,
    Temperature,
    Tint,
    Vibrance,
    Saturation,
    Clarity,
    Dehaze,
    Sharpen,
    Vignette,
    VignetteFeather,
    Grain,
    GrainSize,
    GradingBalance,
}

impl Param {
    pub const ALL: [Param; 18] = [
        Param::Exposure,
        Param::Contrast,
        Param::Highlights,
        Param::Shadows,
        Param::Whites,
        Param::Blacks,
        Param::Temperature,
        Param::Tint,
        Param::Vibrance,
        Param::Saturation,
        Param::Clarity,
        Param::Dehaze,
        Param::Sharpen,
        Param::Vignette,
        Param::VignetteFeather,
        Param::Grain,
        Param::GrainSize,
        Param::GradingBalance,
    ];

    pub fn spec(self) -> ParamSpec {
        let bi = |label| ParamSpec {
            label,
            min: -100.0,
            max: 100.0,
            default: 0.0,
            step: 1.0,
            decimals: 0,
            track: Track::Plain,
        };
        let uni = |label, default| ParamSpec {
            label,
            min: 0.0,
            max: 100.0,
            default,
            step: 1.0,
            decimals: 0,
            track: Track::Plain,
        };
        match self {
            Param::Exposure => ParamSpec { min: -5.0, max: 5.0, step: 0.01, decimals: 2, ..bi("Exposure") },
            Param::Contrast => bi("Contrast"),
            Param::Highlights => bi("Highlights"),
            Param::Shadows => bi("Shadows"),
            Param::Whites => bi("Whites"),
            Param::Blacks => bi("Blacks"),
            Param::Temperature => ParamSpec { track: Track::Temperature, ..bi("Temperature") },
            Param::Tint => ParamSpec { track: Track::Tint, ..bi("Tint") },
            Param::Vibrance => ParamSpec { track: Track::Saturation, ..bi("Vibrance") },
            Param::Saturation => ParamSpec { track: Track::Saturation, ..bi("Saturation") },
            Param::Clarity => bi("Clarity"),
            Param::Dehaze => bi("Dehaze"),
            Param::Sharpen => uni("Sharpen", 0.0),
            Param::Vignette => bi("Vignette"),
            Param::VignetteFeather => uni("Feather", 50.0),
            Param::Grain => uni("Grain", 0.0),
            Param::GrainSize => uni("Grain size", 25.0),
            Param::GradingBalance => bi("Balance"),
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Param::Exposure => "exposure",
            Param::Contrast => "contrast",
            Param::Highlights => "highlights",
            Param::Shadows => "shadows",
            Param::Whites => "whites",
            Param::Blacks => "blacks",
            Param::Temperature => "temperature",
            Param::Tint => "tint",
            Param::Vibrance => "vibrance",
            Param::Saturation => "saturation",
            Param::Clarity => "clarity",
            Param::Dehaze => "dehaze",
            Param::Sharpen => "sharpen",
            Param::Vignette => "vignette",
            Param::VignetteFeather => "vignette_feather",
            Param::Grain => "grain",
            Param::GrainSize => "grain_size",
            Param::GradingBalance => "grading_balance",
        }
    }

    pub fn from_key(key: &str) -> Option<Param> {
        Param::ALL.into_iter().find(|p| p.key() == key)
    }

    pub fn get(self, a: &Adjustments) -> f32 {
        match self {
            Param::Exposure => a.exposure,
            Param::Contrast => a.contrast,
            Param::Highlights => a.highlights,
            Param::Shadows => a.shadows,
            Param::Whites => a.whites,
            Param::Blacks => a.blacks,
            Param::Temperature => a.temperature,
            Param::Tint => a.tint,
            Param::Vibrance => a.vibrance,
            Param::Saturation => a.saturation,
            Param::Clarity => a.clarity,
            Param::Dehaze => a.dehaze,
            Param::Sharpen => a.sharpen,
            Param::Vignette => a.vignette,
            Param::VignetteFeather => a.vignette_feather,
            Param::Grain => a.grain,
            Param::GrainSize => a.grain_size,
            Param::GradingBalance => a.grading.balance,
        }
    }

    pub fn set(self, a: &mut Adjustments, v: f32) {
        let s = self.spec();
        let v = v.clamp(s.min, s.max);
        let slot = match self {
            Param::Exposure => &mut a.exposure,
            Param::Contrast => &mut a.contrast,
            Param::Highlights => &mut a.highlights,
            Param::Shadows => &mut a.shadows,
            Param::Whites => &mut a.whites,
            Param::Blacks => &mut a.blacks,
            Param::Temperature => &mut a.temperature,
            Param::Tint => &mut a.tint,
            Param::Vibrance => &mut a.vibrance,
            Param::Saturation => &mut a.saturation,
            Param::Clarity => &mut a.clarity,
            Param::Dehaze => &mut a.dehaze,
            Param::Sharpen => &mut a.sharpen,
            Param::Vignette => &mut a.vignette,
            Param::VignetteFeather => &mut a.vignette_feather,
            Param::Grain => &mut a.grain,
            Param::GrainSize => &mut a.grain_size,
            Param::GradingBalance => &mut a.grading.balance,
        };
        *slot = v;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partial_json_fills_defaults() {
        let a = Adjustments::from_json(r#"{"exposure":0.5,"hsl":[{"hue":10}]}"#);
        // Arrays must be complete; partial arrays are rejected rather than half-applied.
        assert!(a.is_err());
        let a = Adjustments::from_json(r#"{"exposure":0.5,"contrast":12}"#).unwrap();
        assert_eq!(a.exposure, 0.5);
        assert_eq!(a.contrast, 12.0);
        assert_eq!(a.vignette_feather, 50.0);
        assert!(a.curves.is_identity());
    }

    #[test]
    fn roundtrip() {
        let mut a = Adjustments::default();
        a.hsl[Band::Blue.index()].saturation = -30.0;
        a.curves.master.push(Point::new(0.5, 0.6));
        a.geometry.quarter_turns = 1;
        let b = Adjustments::from_json(&a.to_json()).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn param_set_clamps() {
        let mut a = Adjustments::default();
        Param::Exposure.set(&mut a, 99.0);
        assert_eq!(a.exposure, 5.0);
        Param::Contrast.set(&mut a, -500.0);
        assert_eq!(a.contrast, -100.0);
        for p in Param::ALL {
            assert_eq!(Param::from_key(p.key()), Some(p));
        }
    }

    #[test]
    fn look_keeps_geometry() {
        let mut a = Adjustments::default();
        a.geometry.flip_h = true;
        let look = Adjustments { contrast: 30.0, ..Default::default() };
        let b = a.with_look(&look);
        assert!(b.geometry.flip_h);
        assert_eq!(b.contrast, 30.0);
        assert!(a.is_neutral());
        assert!(!b.is_neutral());
    }
}
