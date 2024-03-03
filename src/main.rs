use std::env;
use std::fs;
use std::path::PathBuf;
use image::{DynamicImage, ImageResult};
use image::io::Reader as ImageReader;
use regex;
use regex::Regex;

/// The ``Options`` struct is used to pass parsed information provided via
/// arguments, options and configuration
struct Options {
    search_pattern: String,
    allow_overwrites: bool
}

fn main() {
    // Generate the options struct based on the vector of arguments
    // If generation goes well, we continue to processing of files
    match generate_options(env::args().skip(1).collect()) {
        Ok(options) => process(options),
        Err(err) => eprintln!("{}", err)
    }
}

fn generate_options(args: Vec<String>) -> Result<Options, String> {
    // A search pattern must be provided
    if args.is_empty() {
        return Err(String::from("You must provide a search pattern, for example: *.tif"));
    }

    // Default values
    let mut allow_overwrites: bool = false;

    // Loop over arguments
    // @todo Use library for better parsing of options
    for arg in &args {
        match arg.as_str() {
            "--allow-overwrites" => allow_overwrites = true,
            _ => {}
        }
    }

    Ok(Options {
        search_pattern: args.get(0).unwrap().clone(),
        allow_overwrites
    })
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
    let files = get_files(options.search_pattern);

    // If file list is empty, we notify the user
    if files.is_empty() {
        eprintln!("{} {}",
                  "File list is empty.",
                  "Review the search pattern and make sure you're in the right directory.");
    }

    // Iterate over each file and attempt to convert them
    for path in files {
        match convert_file(&path, options.allow_overwrites) {
            Ok(_) => println!("OK: {}", path.file_name().unwrap().to_str().unwrap()),
            Err(t) => eprintln!("{}", t),
        }
    }
}

/// Convert file
/// The image is loaded into a ``DynamicImage`` instance, which can then be used
/// to save the image as a new format
fn convert_file(path: &PathBuf, allow_overwrites: bool) -> Result<(), String> {
    // Attempt to read the file
    let img: ImageResult<DynamicImage> = ImageReader::open(path.clone()).unwrap().decode();

    // If reading the file failed, we'll abort
    if img.is_err() {
        return Err(format!("Failed to decode: {}", path.file_name().unwrap().to_str().unwrap()));
    }

    // Generate the new filepath
    let new_path: PathBuf = generate_new_filename(&path);

    // If the path exists, and overwrites are not allowed, we abort
    if new_path.exists() && !allow_overwrites {
        return Err(format!("File exists: {}",
                           new_path.file_name().unwrap().to_str().unwrap()));
    }

    // Attempt to save the file (the changed extension will automatically
    // make Image library encode in that format)
    let res: ImageResult<()> = img.unwrap().save(new_path);

    // If saving failed, we abort
    if res.is_err() {
        return Err(format!("Failed to convert: {}", path.file_name().unwrap().to_str().unwrap()));
    }

    Ok(())
}

/// Retrieves the list of files according to ``search_pattern``.
/// The regular expression for matching filenames is generated witht ``generate_filename_regex``.
/// Then the files of the current working directory are loaded, and afterward
/// filtered using the generated ``Regex``.
fn get_files(pattern: String) -> Vec<PathBuf> {
    // Generate the Regex instance based on the search pattern provided by the user
    let regex = generate_filename_regex(pattern);

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
fn generate_new_filename(current: &PathBuf) -> PathBuf {
    return current.with_extension("png");
}
