# EXIF-based Photo Sorter

This is a simple command-line tool written in Rust to organize JPEG photos based on their EXIF data. It reads the "DateTimeOriginal" tag from each JPEG file in a specified input directory, renames the file according to this timestamp, and moves it to an output directory.

## Features

-   Scans a directory for JPEG files (`.jpg`, `.jpeg`).
-   Reads the "DateTimeOriginal" EXIF tag to determine when the photo was taken.
-   Renames photos to a `YYYY-MM-DD_HH-MM-SS.jpg` format.
-   Handles potential filename conflicts by appending a counter (`_1`, `_2`, etc.).
-   Moves the renamed photos to a specified output directory.
-   Uses parallel processing for faster I/O operations.
-   Provides progress bars to visualize the process.

## Usage

```bash
photo-sorter -i <input-directory> -o <output-directory>
```

### Arguments

-   `-i`, `--in-dir`: The input directory containing the JPEG files to process.
-   `-o`, `--out-dir`: The output directory where the renamed photos will be moved.
-   `--full-scan`: (Optional) Read the entire file to find EXIF data. This is slower but more reliable for files with non-standard structures.

## Example

```bash
photo-sorter -i ./my_photos -o ./sorted_photos
```

This command will scan the `my_photos` directory, rename the JPEG files based on their EXIF data, and move them to the `sorted_photos` directory.
