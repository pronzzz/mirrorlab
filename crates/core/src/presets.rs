//! Built-in looks and the `.mlpreset` file format.

use serde::{Deserialize, Serialize};

use crate::model::{Adjustments, Band, ColorGrading, Curves, HslShift, Point, Wheel};

pub const FILE_EXTENSION: &str = "mlpreset";
const FORMAT_VERSION: u32 = 2;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Preset {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub builtin: bool,
    pub look: Adjustments,
}

#[derive(Serialize, Deserialize)]
struct PresetFile {
    mirrorlab_preset: u32,
    name: String,
    adjustments: Adjustments,
}

impl Preset {
    pub fn custom(name: &str, look: &Adjustments) -> Self {
        let slug: String = name
            .to_lowercase()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect();
        Self {
            id: format!("user-{}-{}", slug.trim_matches('-'), simple_hash(&look.to_json())),
            name: name.trim().to_string(),
            builtin: false,
            look: Adjustments { geometry: Default::default(), ..look.clone() },
        }
    }

    /// Serialise to a shareable `.mlpreset` file.
    pub fn to_file(&self) -> String {
        serde_json::to_string_pretty(&PresetFile {
            mirrorlab_preset: FORMAT_VERSION,
            name: self.name.clone(),
            adjustments: self.look.clone(),
        })
        .unwrap_or_default()
    }

    /// Parse a `.mlpreset` file. Version 1 files from the TypeScript app are
    /// accepted too: they were a bare adjustments object.
    pub fn from_file(text: &str, fallback_name: &str) -> Result<Preset, String> {
        if let Ok(f) = serde_json::from_str::<PresetFile>(text) {
            return Ok(Preset::custom(&f.name, &f.adjustments));
        }
        legacy::parse(text).map(|a| Preset::custom(fallback_name, &a))
    }
}

fn simple_hash(s: &str) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{:08x}", h as u32)
}

fn curve(points: &[(f32, f32)]) -> Vec<Point> {
    points.iter().map(|&(x, y)| Point::new(x, y)).collect()
}

fn look(id: &str, name: &str, a: Adjustments) -> Preset {
    Preset { id: id.into(), name: name.into(), builtin: true, look: a }
}

/// The looks that ship with MirrorLab.
pub fn builtin() -> Vec<Preset> {
    let d = Adjustments::default;
    let mut hsl_teal = [HslShift::default(); 8];
    hsl_teal[Band::Orange.index()] = HslShift { hue: -6.0, saturation: 12.0, luminance: 6.0 };
    hsl_teal[Band::Aqua.index()] = HslShift { hue: 12.0, saturation: -10.0, luminance: -6.0 };
    hsl_teal[Band::Blue.index()] = HslShift { hue: -18.0, saturation: -12.0, luminance: -10.0 };
    hsl_teal[Band::Green.index()] = HslShift { hue: 30.0, saturation: -35.0, luminance: 0.0 };

    let mut hsl_golden = [HslShift::default(); 8];
    hsl_golden[Band::Yellow.index()] = HslShift { hue: -12.0, saturation: 10.0, luminance: 8.0 };
    hsl_golden[Band::Green.index()] = HslShift { hue: -25.0, saturation: -20.0, luminance: 0.0 };
    hsl_golden[Band::Blue.index()] = HslShift { hue: 0.0, saturation: -25.0, luminance: -5.0 };

    let mut hsl_moody = [HslShift::default(); 8];
    hsl_moody[Band::Green.index()] = HslShift { hue: 15.0, saturation: -45.0, luminance: -20.0 };
    hsl_moody[Band::Blue.index()] = HslShift { hue: -10.0, saturation: -30.0, luminance: -25.0 };
    hsl_moody[Band::Yellow.index()] = HslShift { hue: -10.0, saturation: -25.0, luminance: 0.0 };

    vec![
        look("natural", "Natural", Adjustments { contrast: 8.0, vibrance: 12.0, clarity: 6.0, ..d() }),
        look(
            "vivid",
            "Vivid",
            Adjustments { exposure: 0.1, contrast: 22.0, shadows: 15.0, vibrance: 35.0, saturation: 8.0, clarity: 12.0, ..d() },
        ),
        look(
            "crisp",
            "Crisp",
            Adjustments { contrast: 18.0, highlights: -25.0, shadows: 20.0, whites: 10.0, blacks: -10.0, clarity: 25.0, sharpen: 30.0, dehaze: 8.0, ..d() },
        ),
        look(
            "matte",
            "Matte",
            Adjustments {
                contrast: -12.0,
                highlights: -10.0,
                saturation: -10.0,
                curves: Curves { master: curve(&[(0.0, 0.08), (0.25, 0.27), (0.75, 0.76), (1.0, 0.94)]), ..Default::default() },
                ..d()
            },
        ),
        look(
            "soft-film",
            "Soft Film",
            Adjustments {
                exposure: 0.08,
                contrast: -8.0,
                highlights: -30.0,
                shadows: 18.0,
                temperature: 6.0,
                vibrance: -6.0,
                grain: 18.0,
                grain_size: 30.0,
                curves: Curves { master: curve(&[(0.0, 0.05), (0.3, 0.3), (0.7, 0.72), (1.0, 0.96)]), ..Default::default() },
                grading: ColorGrading {
                    shadows: Wheel { hue: 200.0, saturation: 10.0, luminance: 0.0 },
                    highlights: Wheel { hue: 40.0, saturation: 12.0, luminance: 0.0 },
                    ..Default::default()
                },
                ..d()
            },
        ),
        look(
            "golden-hour",
            "Golden Hour",
            Adjustments {
                exposure: 0.1,
                temperature: 28.0,
                tint: 6.0,
                highlights: -20.0,
                shadows: 12.0,
                vibrance: 18.0,
                hsl: hsl_golden,
                vignette: -18.0,
                grading: ColorGrading { highlights: Wheel { hue: 38.0, saturation: 22.0, luminance: 0.0 }, ..Default::default() },
                ..d()
            },
        ),
        look(
            "teal-orange",
            "Teal & Orange",
            Adjustments {
                contrast: 15.0,
                vibrance: 10.0,
                hsl: hsl_teal,
                grading: ColorGrading {
                    shadows: Wheel { hue: 190.0, saturation: 28.0, luminance: 0.0 },
                    highlights: Wheel { hue: 32.0, saturation: 24.0, luminance: 0.0 },
                    balance: 10.0,
                    ..Default::default()
                },
                ..d()
            },
        ),
        look(
            "cool-breeze",
            "Cool Breeze",
            Adjustments {
                temperature: -18.0,
                tint: -4.0,
                exposure: 0.1,
                highlights: -15.0,
                saturation: -8.0,
                curves: Curves {
                    blue: curve(&[(0.0, 0.06), (1.0, 1.0)]),
                    ..Default::default()
                },
                ..d()
            },
        ),
        look(
            "moody",
            "Moody",
            Adjustments {
                exposure: -0.25,
                contrast: 20.0,
                highlights: -35.0,
                shadows: -10.0,
                blacks: -15.0,
                saturation: -15.0,
                hsl: hsl_moody,
                clarity: 15.0,
                vignette: -30.0,
                ..d()
            },
        ),
        look(
            "faded",
            "Faded",
            Adjustments {
                contrast: -25.0,
                saturation: -25.0,
                curves: Curves { master: curve(&[(0.0, 0.15), (0.5, 0.52), (1.0, 0.9)]), ..Default::default() },
                grading: ColorGrading { shadows: Wheel { hue: 30.0, saturation: 12.0, luminance: 0.0 }, ..Default::default() },
                ..d()
            },
        ),
        look("mono", "Mono", Adjustments { saturation: -100.0, contrast: 10.0, clarity: 8.0, ..d() }),
        look(
            "mono-contrast",
            "Mono Contrast",
            Adjustments {
                saturation: -100.0,
                contrast: 45.0,
                whites: 15.0,
                blacks: -20.0,
                clarity: 20.0,
                grain: 22.0,
                vignette: -20.0,
                ..d()
            },
        ),
        look(
            "noir",
            "Noir",
            Adjustments {
                saturation: -100.0,
                exposure: -0.3,
                contrast: 35.0,
                highlights: -20.0,
                shadows: -25.0,
                curves: Curves { master: curve(&[(0.0, 0.0), (0.3, 0.18), (0.7, 0.8), (1.0, 1.0)]), ..Default::default() },
                vignette: -45.0,
                vignette_feather: 70.0,
                grain: 30.0,
                ..d()
            },
        ),
        look(
            "sepia",
            "Sepia",
            Adjustments {
                saturation: -100.0,
                contrast: 8.0,
                grading: ColorGrading {
                    shadows: Wheel { hue: 30.0, saturation: 35.0, luminance: 0.0 },
                    midtones: Wheel { hue: 36.0, saturation: 25.0, luminance: 0.0 },
                    highlights: Wheel { hue: 42.0, saturation: 18.0, luminance: 0.0 },
                    ..Default::default()
                },
                ..d()
            },
        ),
    ]
}

/// The original TypeScript MirrorLab stored adjustments with camelCase keys,
/// 0..255 curve points and a 4-colour-grading shape. This maps that format.
mod legacy {
    use super::*;
    use serde_json::Value;

    pub fn parse(text: &str) -> Result<Adjustments, String> {
        let v: Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
        let v = v.get("adjustments").unwrap_or(&v);
        if !v.is_object() {
            return Err("Not a MirrorLab preset".into());
        }
        let num = |k: &str| v.get(k).and_then(Value::as_f64).map(|x| x as f32);
        let mut a = Adjustments::default();
        for (key, slot) in [
            ("exposure", &mut a.exposure),
            ("contrast", &mut a.contrast),
            ("highlights", &mut a.highlights),
            ("shadows", &mut a.shadows),
            ("whites", &mut a.whites),
            ("blacks", &mut a.blacks),
            ("temperature", &mut a.temperature),
            ("tint", &mut a.tint),
            ("vibrance", &mut a.vibrance),
            ("saturation", &mut a.saturation),
            ("clarity", &mut a.clarity),
            ("grain", &mut a.grain),
        ] {
            if let Some(x) = num(key) {
                *slot = x;
            }
        }
        // The old vignette darkened for positive values.
        if let Some(x) = num("vignette") {
            a.vignette = -x;
        }
        if let Some(hsl) = v.get("hsl").and_then(Value::as_object) {
            for band in Band::ALL {
                if let Some(ch) = hsl.get(&band.label().to_lowercase()) {
                    let f = |k| ch.get(k).and_then(Value::as_f64).unwrap_or(0.0) as f32;
                    a.hsl[band.index()] = HslShift { hue: f("hue"), saturation: f("saturation"), luminance: f("luminance") };
                }
            }
        }
        if let Some(cg) = v.get("colorGrading") {
            let wheel = |k: &str| {
                let z = cg.get(k)?;
                Some(Wheel {
                    hue: z.get("hue")?.as_f64()? as f32,
                    saturation: z.get("saturation")?.as_f64()? as f32,
                    luminance: 0.0,
                })
            };
            if let Some(w) = wheel("shadows") {
                a.grading.shadows = w;
            }
            if let Some(w) = wheel("highlights") {
                a.grading.highlights = w;
            }
        }
        if let Some(c) = v.get("curve") {
            let pts = |k: &str| -> Option<Vec<Point>> {
                let arr = c.get(k)?.as_array()?;
                Some(
                    arr.iter()
                        .filter_map(|p| Some(Point::new(p.get("x")?.as_f64()? as f32, p.get("y")?.as_f64()? as f32)))
                        .collect(),
                )
            };
            if let Some(p) = pts("master") {
                a.curves.master = p;
            }
            if let Some(p) = pts("red") {
                a.curves.red = p;
            }
            if let Some(p) = pts("green") {
                a.curves.green = p;
            }
            if let Some(p) = pts("blue") {
                a.curves.blue = p;
            }
        }
        Ok(a)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtins_are_unique_and_valid() {
        let all = builtin();
        let mut ids: Vec<_> = all.iter().map(|p| p.id.clone()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), all.len());
        for p in &all {
            assert!(p.look.geometry.is_identity());
            assert!(!p.look.is_neutral(), "{} does nothing", p.name);
        }
    }

    #[test]
    fn file_roundtrip() {
        let p = &builtin()[3];
        let text = p.to_file();
        let q = Preset::from_file(&text, "x").unwrap();
        assert_eq!(q.name, p.name);
        assert_eq!(q.look, p.look);
        assert!(!q.builtin);
    }

    #[test]
    fn reads_legacy_typescript_presets() {
        let old = r#"{"exposure":0.2,"contrast":20,"saturation":30,"vignette":20,
            "hsl":{"blue":{"hue":0,"saturation":10,"luminance":-10}},
            "colorGrading":{"shadows":{"hue":200,"saturation":20},"highlights":{"hue":40,"saturation":10}},
            "curve":{"master":[{"x":0,"y":0.1},{"x":1,"y":0.9}]}}"#;
        let p = Preset::from_file(old, "Old look").unwrap();
        assert_eq!(p.name, "Old look");
        assert_eq!(p.look.contrast, 20.0);
        assert_eq!(p.look.vignette, -20.0);
        assert_eq!(p.look.hsl[Band::Blue.index()].luminance, -10.0);
        assert_eq!(p.look.grading.shadows.hue, 200.0);
        assert_eq!(p.look.curves.master[0].y, 0.1);
        assert!(Preset::from_file("[1,2]", "x").is_err());
    }
}
