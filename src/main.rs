use clap::Parser;
use walkdir::WalkDir;
use rexif::parse_buffer;
use rexif::ExifTag;
use std::fs;
use std::path::{Path, PathBuf};
use indicatif::{ProgressBar, ProgressStyle};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// 輸入目錄
    #[arg(short, long)]
    in_dir: PathBuf,

    /// 輸出目錄
    #[arg(short, long)]
    out_dir: PathBuf,
}

fn get_date_taken(path: &Path) -> Option<String> {
    let file_bytes = std::fs::read(path).ok()?;
    let exif = parse_buffer(&file_bytes).ok()?;

    for entry in exif.entries {
        if entry.tag == ExifTag::DateTimeOriginal {
            return Some(entry.value.to_string());
        }
    }
    None
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    if !args.in_dir.is_dir() {
        eprintln!("錯誤：輸入目錄不存在或不是一個目錄：{:?}", args.in_dir);
        return Ok(());
    }

    fs::create_dir_all(&args.out_dir)?;

    let files: Vec<PathBuf> = WalkDir::new(&args.in_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|entry| entry.file_type().is_file() && entry.path().extension().map_or(false, |ext| ext.eq_ignore_ascii_case("jpg")))
        .map(|entry| entry.path().to_path_buf())
        .collect();

    let pb = ProgressBar::new(files.len() as u64);
    pb.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) {msg}")?
        .progress_chars("#>-"));

    for f in files {
        pb.inc(1);
        let date_str = match get_date_taken(&f) {
            Some(d) => d,
            None => {
                pb.set_message(format!("跳過 {:?}：無法獲取拍攝日期", f));
                continue;
            }
        };

        let out_name = date_str.replace(':', "-").replace(' ', "_") + ".jpg";
        let out_path = args.out_dir.join(&out_name);

        if out_path.is_file() {
            pb.set_message(format!("跳過 {:?}：輸出檔案已存在", f));
            continue;
        }

        match fs::rename(&f, &out_path) {
            Ok(_) => pb.set_message(format!("已重新命名 {:?} 為 {:?}", f.file_name().unwrap_or_default(), out_path.file_name().unwrap_or_default
())),
            Err(e) => pb.set_message(format!("重新命名 {:?} 失敗：{}", f, e)),
        }
    }

    pb.finish_with_message("完成！");

    Ok(())
}
