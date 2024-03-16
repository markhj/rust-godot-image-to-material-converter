use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use image::{DynamicImage, ImageResult};
use image::io::Reader as ImageReader;
use regex;
use regex::Regex;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "Godot Image to Material Converter")]
#[command(version = "0.1.2")]
struct Options {
    /// Regular expression applied on every file found
    search_pattern: String,

    /// Overwrite output files which already exists
    #[arg(short, long, default_value_t = false)]
    allow_overwrites: bool,

    /// The subdirectory where the output files should be located
    #[arg(short, long)]
    destination: Option<String>,

    /// Preview what will happen with the current configuration
    /// No files will be converted when this flag is on
    #[arg(short, long, default_value_t = false)]
    preview: bool,

    /// Generate a Godot StandardMaterial3D based on the converted files
    /// This requires that the filenames contain hints such as "albedo" or "normal"
    #[arg(short, long, default_value_t = false)]
    material: bool,
}

/// Error types for the ``convert_file`` method.
/// We want to handle errors differently, for instance when a file exists, it should
/// not be excluded from the list which is passed onto the material generator
enum ConversionError {
    FailedToDecode,
    FailedToConvert,
    FileExists,
}

fn main() {
    process(Options::parse());
}

/// Generate the ``Regex`` instance based on the provided ``search_pattern``
/// from the ``Options`` struct
fn generate_filename_regex(pattern: String) -> Regex {
    // Set up the regular expression pattern as a string
    let full_pattern: String = format!(r"^{}$", pattern);

    // Build the Regex instance based on the full_pattern string
    Regex::new(&*full_pattern).expect("Invalid regex pattern")
}

/// Run the processing.
/// First, files are collected with the ``get_files`` method, which is also
/// responsible for filtering files according to ``options``.
///
/// Then, a number of checks are made, such as whether the file already exists.
/// If all checks pass, the file will be converted.
fn process(options: Options) {
    let files = get_files(&options);

    create_destination_directory(&options).expect("Failed to create destination");

    // If file list is empty, we notify the user
    if files.is_empty() {
        eprintln!("{} {}",
                  "File list is empty.",
                  "Review the search pattern and make sure you're in the right directory.");
    }

    // The list of converted (or existing conversion), which will be passed
    // to the material generator
    let mut converted_files: Vec<PathBuf> = Vec::new();

    // Iterate over each file and attempt to convert them
    for path in files {
        // Store the original filename
        let original = path.file_name().unwrap().to_str().unwrap();

        match convert_file(&path, &options) {
            Ok(new_path) => {
                if options.preview {
                    println!("File {} would converted and moved to: {}", original, new_path.to_str().unwrap())
                } else {
                    println!("OK: {}", original)
                }
                converted_files.push(new_path);
            },
            Err(ConversionError::FailedToDecode) => eprintln!("Failed to decode: {}", original),
            Err(ConversionError::FailedToConvert) => eprintln!("Failed to convert: {}", original),
            Err(ConversionError::FileExists) => {
                let new_path = generate_new_filename(&path, &options.destination);
                println!("Exists: {}", new_path.file_name().unwrap().to_str().unwrap());
                converted_files.push(new_path);
            },
        }
    }

    if options.material {
        generate_godot_material(options, converted_files);
    }
}

/// Retrieve the compiled material data and store it in a file
/// When in preview mode, instead show where the file would be located
fn generate_godot_material(options: Options, converted_files: Vec<PathBuf>) {
    let mat_data: Result<String, String> = material::generate(converted_files);
    let mat_path = PathBuf::from("material.tres");

    if mat_data.is_err() {
        eprintln!("{}", mat_data.err().unwrap())
    } else if !options.allow_overwrites && mat_path.exists() {
        println!("Material file ({}) exists and will not be overwritten", mat_path.to_str().unwrap());
    } else if options.preview {
        println!("Material file would be generated at: {}", mat_path.to_str().unwrap());
    } else {
        fs::write(mat_path.clone(), mat_data.unwrap())
            .expect("Failed to generate material");
        println!("Generated material: {}", mat_path.to_str().unwrap());
    }
}

/// If the user has requested a destination directory, we will first
/// check if that directory exists -- and if not, we will create it
fn create_destination_directory(options: &Options) -> Result<(), String> {
    let dest = &options.destination;

    // If not destination is requested, return OK
    if dest.is_none() {
        return Ok(());
    }

    let dir: String = dest.clone().unwrap();
    let dir_path: &Path = Path::new(&dir);

    // If the directory already exists, return OK
    if dir_path.is_dir() {
        return Ok(());
    }

    if options.preview {
        return Ok(());
    }

    // Abort, if we failed to create the directory
    if let Err(err) = fs::create_dir(dir_path) {
        return Err(format!("Error creating directory: {}", err));
    }

    Ok(())
}

/// Convert file
/// The image is loaded into a ``DynamicImage`` instance, which can then be used
/// to save the image as a new format
fn convert_file(path: &PathBuf, options: &Options) -> Result<PathBuf, ConversionError> {
    let allow_overwrites = options.allow_overwrites;
    let destination = &options.destination;

    // Attempt to read the file
    let img: ImageResult<DynamicImage> = ImageReader::open(path.clone()).unwrap().decode();

    // If reading the file failed, we'll abort
    if img.is_err() {
        return Err(ConversionError::FailedToDecode);
    }

    // Generate the new filepath
    let new_path: PathBuf = generate_new_filename(&path, &destination);

    // If the path exists, and overwrites are not allowed, we abort
    if new_path.exists() && !allow_overwrites {
        return Err(ConversionError::FileExists);
    }

    // If in preview mode, we will abort here to avoid carrying
    // out actual actions. Instead, we just return OK
    if options.preview {
        return Ok(new_path.clone());
    }

    // Attempt to save the file (the changed extension will automatically
    // make Image library encode in that format)
    let res: ImageResult<()> = img.unwrap().save(new_path.clone());

    // If saving failed, we abort
    if res.is_err() {
        return Err(ConversionError::FailedToConvert);
    }

    Ok(new_path.clone())
}

/// Retrieves the list of files according to ``search_pattern``.
/// The regular expression for matching filenames is generated witht ``generate_filename_regex``.
/// Then the files of the current working directory are loaded, and afterward
/// filtered using the generated ``Regex``.
fn get_files(options: &Options) -> Vec<PathBuf> {
    // Generate the Regex instance based on the search pattern provided by the user
    let regex = generate_filename_regex(options.search_pattern.clone());

    // Load current direction and list of files
    let current_dir = env::current_dir().expect("Failed to retrieve directory");
    let files = fs::read_dir(&current_dir).expect("Failed to read files in directory");

    // Return list of files filtered by the regular expression instance
    files
        .filter_map(|entry| {
            entry.ok().and_then(|dir_entry| {
                let path = dir_entry.path();

                // If path is a file and its basename/filename matches the pattern
                // we want to keep it in the list of files
                if path.is_file() && regex.is_match(path.file_name().unwrap().to_str().unwrap()) {
                    Some(path)
                } else {
                    None
                }
            })
        })
        .collect()
}

/// Generates the output filename, based on options/configuration and
/// the input filename.
fn generate_new_filename(current: &PathBuf, destination: &Option<String>) -> PathBuf {
    let mut path = current.clone();

    // If destination is requested, we insert the directory name between the filename
    // and spot before the filename in the original path
    if destination.is_some() {
        path.pop();
        path.push(destination.clone().unwrap());
        path.push(current.file_name().unwrap());
    }

    return path.with_extension("png");
}
