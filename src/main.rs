use clap::Parser;
use walkdir::WalkDir;
use rexif::{parse_buffer, ExifTag};
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

    /// Read the entire file to find EXIF data. Slower but more reliable.
    #[arg(short, long, default_value_t = false)]
    full_scan: bool,
}

fn get_date_taken(path: &Path, full_scan: bool) -> Result<String, Box<dyn std::error::Error>> {
    let mut file = fs::File::open(path)?;
    let exif = if full_scan {
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;
        parse_buffer(&buffer)?
    } else {
        // Read only the first 64KB, which is usually enough for EXIF data.
        let mut buffer = vec![0; 64 * 1024];
        let n = file.read(&mut buffer)?;
        parse_buffer(&buffer[..n])?
    };

    for entry in exif.entries {
        if entry.tag == ExifTag::DateTimeOriginal {
            return Ok(entry.value.to_string());
        }
    }
    Err("Could not find DateTimeOriginal EXIF tag".into())
}

fn find_available_path(out_dir: &Path, date_str: &str) -> PathBuf {
    let base_name = date_str.replace(':', "-").replace(' ', "_");
    let mut counter = 0;
    loop {
        let out_name = if counter == 0 {
            format!("{}.jpg", base_name)
        } else {
            format!("{}_{}.jpg", base_name, counter)
        };
        let out_path = out_dir.join(&out_name);
        if !out_path.exists() {
            return out_path;
        }
        counter += 1;
    }
}

fn process_file(
    path: &Path,
    out_dir: &Path,
    full_scan: bool,
) -> Result<(), String> {
    let date_str = get_date_taken(path, full_scan)
        .map_err(|e| format!("Skipping {:?}: Could not get date taken - {}", path, e))?;

    let out_path = find_available_path(out_dir, &date_str);

    fs::rename(path, &out_path)
        .map_err(|e| format!("Failed to rename {:?}: {}", path, e))?;

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    if !args.in_dir.is_dir() {
        eprintln!("Error: Input directory does not exist or is not a directory: {:?}", args.in_dir);
        return Err("Input directory error".into());
    }

    if args.out_dir.starts_with(&args.in_dir) {
        eprintln!("Error: Output directory cannot be a subdirectory of the input directory.");
        return Err("Output directory error".into());
    }

    fs::create_dir_all(&args.out_dir)?;

    let walker = WalkDir::new(&args.in_dir).into_iter().filter_map(|e| e.ok());

    let jpeg_files: Vec<_> = walker
        .filter(|entry| {
            entry.file_type().is_file()
                && entry.path().extension().map_or(false, |ext| {
                    ext.eq_ignore_ascii_case("jpg") || ext.eq_ignore_ascii_case("jpeg")
                })
        })
        .collect();

    let pb = ProgressBar::new(jpeg_files.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({percent}%) {msg}")?
            .progress_chars("#>-"),
    );

    let mut failed_files: Vec<String> = Vec::new();

    for entry in jpeg_files {
        let path = entry.path();
        
        if let Err(e) = process_file(path, &args.out_dir, args.full_scan) {
            failed_files.push(e);
        }
        
        pb.inc(1);
    }

    pb.finish_with_message("Done!");

    if !failed_files.is_empty() {
        eprintln!("\n--- Summary of Errors ---");
        for error in failed_files {
            eprintln!("{}", error);
        }
    }

    Ok(())
}
