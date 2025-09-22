use clap::Parser;
use walkdir::WalkDir;
use rexif::parse_buffer;
use rexif::ExifTag;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use indicatif::{ProgressBar, ProgressStyle};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Input directory
    #[arg(short, long)]
    in_dir: PathBuf,

    /// Output directory
    #[arg(short, long)]
    out_dir: PathBuf,
}

fn get_date_taken(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let mut file = fs::File::open(path)?;
    // Read only the first 64KB, which is usually enough for EXIF data.
    let mut buffer = vec![0; 64 * 1024];
    let n = file.read(&mut buffer)?;
    let exif = parse_buffer(&buffer[..n])?;

    for entry in exif.entries {
        if entry.tag == ExifTag::DateTimeOriginal {
            return Ok(entry.value.to_string());
        }
    }
    Err("Could not find date taken".into())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    if !args.in_dir.is_dir() {
        eprintln!("Error: Input directory does not exist or is not a directory: {:?}", args.in_dir);
        return Err("Input directory error".into());
    }

    fs::create_dir_all(&args.out_dir)?;

    let walker = WalkDir::new(&args.in_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|entry| entry.file_type().is_file() && entry.path().extension().map_or(false, |ext| {
            ext.eq_ignore_ascii_case("jpg") || ext.eq_ignore_ascii_case("jpeg")
        }));

    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::default_spinner()
        .template("{spinner:.green} [{elapsed_precise}] {pos} files processed {msg}")?
    );

    for entry in walker {
        let f = entry.path();
        pb.inc(1);
        let date_str = match get_date_taken(f) {
            Ok(d) => d,
            Err(e) => {
                pb.set_message(format!("Skipping {:?}: Could not get date taken - {}", f, e));
                continue;
            }
        };

        let base_name = date_str.replace(':', "-").replace(' ', "_");
        let mut out_name = format!("{}.jpg", base_name);
        let mut counter = 0;

        loop {
            let out_path = args.out_dir.join(&out_name);
            if !out_path.is_file() {
                match fs::rename(f, &out_path) {
                    Ok(_) => pb.set_message(format!("Renamed {:?} to {:?}", f.file_name().unwrap_or_default(), out_path.file_name().unwrap_or_default())),
                    Err(e) => pb.set_message(format!("Failed to rename {:?}: {}", f, e)),
                }
                break;
            }
            counter += 1;
            out_name = format!("{}_{}.jpg", base_name, counter);
        }
    }

    pb.finish_with_message("Done!");

    Ok(())
}
