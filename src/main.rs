use clap::Parser;
use walkdir::WalkDir;
use rayon::prelude::*;
use rexif::{parse_buffer, ExifTag};
use std::collections::HashSet;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None, arg_required_else_help = true)]
struct Args {
    /// Input directory
    #[arg(short = 'i', long)]
    in_dir: PathBuf,

    /// Output directory
    #[arg(short = 'o', long)]
    out_dir: PathBuf,

    /// Read the entire file to find EXIF data. Slower but more reliable.
    #[arg(short, long, default_value_t = false)]
    full_scan: bool,
}

fn get_date_taken(path: &Path, full_scan: bool) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
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
        .map(|entry| entry.into_path())
        .collect();

    let multi_pb = MultiProgress::new();
    let style = ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({percent}%) {msg}")?
        .progress_chars("#>");

    let failed_files = Mutex::new(Vec::new());

    // --- Phase 1: Parallel EXIF Parsing ---
    let pb1 = multi_pb.add(ProgressBar::new(jpeg_files.len() as u64));
    pb1.set_style(style.clone());
    pb1.set_message("Parsing files");

    let parsed_data: Vec<(PathBuf, String)> = jpeg_files
        .par_iter()
        .filter_map(|path| {
            let result = match get_date_taken(path, args.full_scan) {
                Ok(date_str) => Some((path.clone(), date_str)),
                Err(e) => {
                    let error_msg = format!("Skipping {:?}: Could not get date taken - {}", path, e);
                    failed_files.lock().unwrap().push(error_msg);
                    None
                }
            };
            pb1.inc(1);
            result
        })
        .collect();

    pb1.finish_with_message("Parsing complete!");

    // --- Phase 2: Sequential Destination Planning ---
    let mut planned_moves = Vec::new();
    let mut used_names = HashSet::new();
    for (source_path, date_str) in parsed_data {
        let base_name = date_str.replace(':', "-").replace(' ', "_");
        let mut counter = 0;
        let dest_path = loop {
            let out_name = if counter == 0 {
                format!("{}.jpg", base_name)
            } else {
                format!("{}_{}.jpg", base_name, counter)
            };
            let dest_path_candidate = args.out_dir.join(&out_name);
            if !dest_path_candidate.exists() && used_names.insert(out_name) {
                break dest_path_candidate;
            }
            counter += 1;
        };
        planned_moves.push((source_path, dest_path));
    }

    // --- Phase 3: Parallel I/O Execution ---
    let pb2 = multi_pb.add(ProgressBar::new(planned_moves.len() as u64));
    pb2.set_style(style);
    pb2.set_message("Moving files");

    planned_moves
        .par_iter()
        .for_each(|(source_path, dest_path)| {
            if let Err(e) = fs::rename(source_path, dest_path) {
                let error_msg = format!("Failed to rename {:?}: {}", source_path, e);
                failed_files.lock().unwrap().push(error_msg);
            }
            pb2.inc(1);
        });

    pb2.finish_with_message("Done!");

    let final_failed_files = failed_files.into_inner().unwrap();
    if !final_failed_files.is_empty() {
        eprintln!("\n--- Summary of Errors ---");
        for error in final_failed_files {
            eprintln!("{}", error);
        }
    }

    Ok(())
}
