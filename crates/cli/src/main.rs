//! `mirrorlab` – apply MirrorLab looks to many photos at once.
//!
//! ```text
//! mirrorlab presets
//! mirrorlab edit photos/*.jpg --preset golden-hour --set exposure=0.3 -o out/
//! mirrorlab edit shoot/*.NEF --settings look.mlpreset --long-edge 2048 --format jpg -q 88
//! ```

use std::path::{Path, PathBuf};
use std::time::Instant;

use clap::{Parser, Subcommand, ValueEnum};
use image::codecs::jpeg::JpegEncoder;
use image::ImageEncoder;
use mirror_core::decode;
use mirror_core::presets::{self, Preset};
use mirror_core::{render, Adjustments, Param, RenderOptions};

#[derive(Parser)]
#[command(name = "mirrorlab", version, about = "Batch photo editing with MirrorLab looks")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// List the built-in looks.
    Presets,
    /// List every adjustable setting and its range.
    Settings,
    /// Show what MirrorLab sees in a file.
    Info { file: PathBuf },
    /// Edit photos and write the results.
    Edit(EditArgs),
}

#[derive(Parser)]
struct EditArgs {
    /// Photos to edit (JPEG, PNG, WebP, TIFF or camera RAW).
    #[arg(required = true)]
    inputs: Vec<PathBuf>,
    /// Built-in look id (see `mirrorlab presets`) or a `.mlpreset` file.
    #[arg(short, long)]
    preset: Option<String>,
    /// Full settings file (JSON adjustments or `.mlpreset`), applied before --preset.
    #[arg(long)]
    settings: Option<PathBuf>,
    /// Override one setting, e.g. `--set exposure=0.5 --set vignette=-30`.
    #[arg(long = "set", value_name = "KEY=VALUE")]
    sets: Vec<String>,
    /// Output folder (created if missing).
    #[arg(short, long, default_value = "mirrorlab-out")]
    out: PathBuf,
    #[arg(short, long, value_enum, default_value_t = Format::Jpg)]
    format: Format,
    /// JPEG quality, 1–100.
    #[arg(short, long, default_value_t = 92)]
    quality: u8,
    /// Scale so the long edge is at most this many pixels.
    #[arg(long)]
    long_edge: Option<u32>,
    /// Suffix added to output file names.
    #[arg(long, default_value = "-edit")]
    suffix: String,
}

#[derive(Clone, Copy, ValueEnum)]
enum Format {
    Jpg,
    Png,
    Tiff,
}

impl Format {
    fn ext(self) -> &'static str {
        match self {
            Format::Jpg => "jpg",
            Format::Png => "png",
            Format::Tiff => "tiff",
        }
    }
}

fn main() {
    if let Err(e) = run(Cli::parse()) {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> Result<(), String> {
    match cli.command {
        Command::Presets => {
            for p in presets::builtin() {
                println!("{:<16} {}", p.id, p.name);
            }
            Ok(())
        }
        Command::Settings => {
            for p in Param::ALL {
                let s = p.spec();
                println!("{:<18} {:>6} … {:<6} (default {})", p.key(), s.min, s.max, s.default);
            }
            Ok(())
        }
        Command::Info { file } => {
            let bytes = std::fs::read(&file).map_err(|e| format!("{}: {e}", file.display()))?;
            let t = Instant::now();
            let d = decode::decode(&bytes, &file.to_string_lossy())?;
            println!("file      {}", file.display());
            println!("size      {} × {}", d.image.width, d.image.height);
            println!("source    {:?}", d.source);
            if let Some(c) = d.camera {
                println!("camera    {c}");
            }
            println!("decoded   {:.0} ms", t.elapsed().as_secs_f64() * 1000.0);
            Ok(())
        }
        Command::Edit(args) => edit(args),
    }
}

fn load_look(args: &EditArgs) -> Result<Adjustments, String> {
    let mut look = Adjustments::default();
    if let Some(path) = &args.settings {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        look = match Adjustments::from_json(&text) {
            Ok(a) if !text.contains("mirrorlab_preset") => a,
            _ => Preset::from_file(&text, "settings")?.look,
        };
    }
    if let Some(name) = &args.preset {
        let found = presets::builtin().into_iter().find(|p| p.id == *name || p.name.eq_ignore_ascii_case(name));
        let preset = match found {
            Some(p) => p,
            None => {
                let text = std::fs::read_to_string(name)
                    .map_err(|_| format!("unknown preset '{name}' (try `mirrorlab presets`)"))?;
                Preset::from_file(&text, name)?
            }
        };
        look = look.with_look(&preset.look);
    }
    for s in &args.sets {
        let (k, v) = s.split_once('=').ok_or_else(|| format!("--set expects KEY=VALUE, got '{s}'"))?;
        let param = Param::from_key(k.trim()).ok_or_else(|| format!("unknown setting '{k}' (try `mirrorlab settings`)"))?;
        let v: f32 = v.trim().parse().map_err(|_| format!("'{v}' is not a number"))?;
        param.set(&mut look, v);
    }
    Ok(look)
}

fn edit(args: EditArgs) -> Result<(), String> {
    let look = load_look(&args)?;
    std::fs::create_dir_all(&args.out).map_err(|e| format!("{}: {e}", args.out.display()))?;
    let total = Instant::now();
    let mut failed = 0;
    for input in &args.inputs {
        match edit_one(input, &look, &args) {
            Ok((out, ms)) => println!("✓ {} → {} ({ms:.0} ms)", input.display(), out.display()),
            Err(e) => {
                failed += 1;
                eprintln!("✗ {}: {e}", input.display());
            }
        }
    }
    println!("{} photo(s) in {:.1}s", args.inputs.len() - failed, total.elapsed().as_secs_f64());
    if failed > 0 {
        Err(format!("{failed} photo(s) failed"))
    } else {
        Ok(())
    }
}

fn edit_one(input: &Path, look: &Adjustments, args: &EditArgs) -> Result<(PathBuf, f64), String> {
    let t = Instant::now();
    let bytes = std::fs::read(input).map_err(|e| e.to_string())?;
    let decoded = decode::decode(&bytes, &input.to_string_lossy())?;
    let out = render(&decoded.image, look, RenderOptions { long_edge: args.long_edge });
    let stem = input.file_stem().and_then(|s| s.to_str()).unwrap_or("photo");
    let path = args.out.join(format!("{stem}{}.{}", args.suffix, args.format.ext()));
    let rgb: Vec<u8> = out.data.chunks_exact(4).flat_map(|p| [p[0], p[1], p[2]]).collect();
    let file = std::fs::File::create(&path).map_err(|e| e.to_string())?;
    let w = std::io::BufWriter::new(file);
    let ct = image::ExtendedColorType::Rgb8;
    match args.format {
        Format::Jpg => JpegEncoder::new_with_quality(w, args.quality.clamp(1, 100)).write_image(&rgb, out.width, out.height, ct),
        Format::Png => image::codecs::png::PngEncoder::new(w).write_image(&rgb, out.width, out.height, ct),
        Format::Tiff => image::codecs::tiff::TiffEncoder::new(w).write_image(&rgb, out.width, out.height, ct),
    }
    .map_err(|e| e.to_string())?;
    Ok((path, t.elapsed().as_secs_f64() * 1000.0))
}
