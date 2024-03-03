use std::{env, io, thread};
use std::fs;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;
use image::{DynamicImage, ImageResult};
use image::io::Reader as ImageReader;
use regex;
use regex::Regex;
use clap::Parser;
use rand::distributions::{Alphanumeric, DistString};
use rand::thread_rng;

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

/// Godot Material Property
/// Contains the supported material property types such as albedo, normal map and roughness
enum GodotMaterialProperty {
    AlbedoTexture,
    NormalTexture,
    HeightTexture,
    RoughnessTexture,
    MetallicTexture,
    AmbientOcclusionTexture
}

/// Godot material mapping
/// A result-type object which contains all information relevant to generate
/// a Godot material such as source files, property type and UID
struct GodotMaterialMapping {
    path: PathBuf,
    uid: String,
    short_uid: String,
    source_file: String,
    property: GodotMaterialProperty,
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
        generate_material(&options, converted_files);
    }
}

/// Generate a ``StandardMaterial3D`` based on the files that have been converted
/// A requirement for this to work is that the files contain hints in their names
/// such as "albedo" or "normal"
///
/// Currently supported hints:
/// * albedo
/// * normal
/// * height
/// * roughness
/// * metallic
/// * ao (Ambient Occlusion)
fn generate_material(_options: &Options, files: Vec<PathBuf>) {
    let mut files_found: Vec<PathBuf>;
    let mut iters: i8 = 0;
    let max_iters: i8 = 100;

    loop {
        files_found = Vec::new();

        for f in &files {
            let ext = f.extension().unwrap().to_str().unwrap();
            let import_path = f.clone().with_extension(format!("{}.import", ext));
            if import_path.exists() {
                files_found.push(import_path);
            }
        }

        if files_found.len() < files.len() {
            if iters == 0 {
                print!("{} {}",
                       "Waiting for .import files.",
                       "Make Godot window active. This will prompt it to create the .import files: .",
                );
            } else {
                print!(".");
            }

            io::stdout().flush().unwrap();
            thread::sleep(Duration::from_secs(1));
        }

        iters += 1;
        if iters >= max_iters || files_found.len() == files.len() {
            break;
        }
    }

    if files_found.len() < files.len() {
        println!("Will not wait any longer for .import files");
        return;
    }

    println!("\nGenerating material...");

    let mut uid_mapping: Vec<GodotMaterialMapping> = Vec::new();

    let uid_regex = Regex::new(r#"\buid="uid://([^"]+)""#).unwrap();
    let sf_regex = Regex::new(r#"\bsource_file="(res://[^"]+)""#).unwrap();

    for import_file in &files_found {
        let mut file = File::open(import_file).expect("Failed to open import file");
        let mut data = String::new();

        let mut uid: Option<String> = None;
        let mut source_file: Option<String> = None;

        file.read_to_string(&mut data).expect("Failed to read lines from import file");

        for line in data.lines() {
            if let Some(captures) = uid_regex.captures(&line) {
                if let Some(value) = captures.get(1).map(|m| m.as_str()) {
                    uid = Some(value.to_owned());
                }
            }

            if let Some(captures) = sf_regex.captures(&line) {
                if let Some(value) = captures.get(1).map(|m| m.as_str()) {
                    source_file = Some(value.to_owned());
                }
            }
        }

        let property: Option<GodotMaterialProperty> = get_godot_property(import_file);
        if uid.is_some() && source_file.is_some() {
            uid_mapping.push(GodotMaterialMapping {
                path: import_file.clone(),
                property: property.unwrap(),
                uid: uid.unwrap(),
                source_file: source_file.unwrap(),
                short_uid: format!("{}_{}", uid_mapping.len() + 1, generate_godot_uid(5)),
            });
        }
    }

    if uid_mapping.len() != files_found.len() {
        eprintln!("UID mapping does not match number of files.");
        return;
    }

    let mut mat_data = String::new();

    mat_data.push_str(
        format!("[gd_resource type=\"StandardMaterial3D\" format=3 uid=\"uid://{}\"]\n\n",
            generate_godot_uid(12)
        ).as_str()
    );

    for res in &uid_mapping {
        mat_data.push_str(format!(
            "[ext_resource type=\"Texture2D\" path=\"{}\" uid=\"uid://{}\" id=\"{}\"]\n",
            res.source_file,
            res.uid,
            res.short_uid
        ).as_str());
    }

    mat_data.push_str("\n[resource]");

    for prop in uid_mapping {
        match prop.property {
            GodotMaterialProperty::AlbedoTexture => {
                mat_data.push_str(format!(
                    "\nalbedo_texture = ExtResource(\"{}\")",
                    prop.short_uid).as_str()
                );
            },
            GodotMaterialProperty::NormalTexture => {
                mat_data.push_str("\nnormal_enabled = true");
                mat_data.push_str(format!(
                    "\nnormal_texture = ExtResource(\"{}\")",
                    prop.short_uid).as_str()
                );
            },
            GodotMaterialProperty::RoughnessTexture => {
                mat_data.push_str(format!(
                    "\nroughness_texture = ExtResource(\"{}\")",
                    prop.short_uid).as_str()
                );
            },
            GodotMaterialProperty::HeightTexture => {
                mat_data.push_str("\nheightmap_enabled = true");
                mat_data.push_str(format!(
                    "\nheightmap_texture = ExtResource(\"{}\")",
                    prop.short_uid).as_str()
                );
            },
            GodotMaterialProperty::MetallicTexture => {
                mat_data.push_str("\nmetallic = 1.0");
                mat_data.push_str(format!(
                    "\nmetallic_texture = ExtResource(\"{}\")",
                    prop.short_uid).as_str()
                );
            },
            GodotMaterialProperty::AmbientOcclusionTexture => {
                mat_data.push_str("\nao_enabled = true");
                mat_data.push_str(format!(
                    "\nao_texture = ExtResource(\"{}\")",
                    prop.short_uid).as_str()
                );
            },
        }
    }

    fs::write("material.tres", mat_data);
}

fn get_godot_property(path: &PathBuf) -> Option<GodotMaterialProperty> {
    let filename = path.file_name().unwrap().to_str().unwrap();

    if filename.contains("albedo") {
        return Some(GodotMaterialProperty::AlbedoTexture);
    }

    if filename.contains("normal") {
        return Some(GodotMaterialProperty::NormalTexture);
    }

    if filename.contains("height") {
        return Some(GodotMaterialProperty::HeightTexture);
    }

    if filename.contains("roughness") {
        return Some(GodotMaterialProperty::RoughnessTexture);
    }

    if filename.contains("metallic") {
        return Some(GodotMaterialProperty::MetallicTexture);
    }

    if filename.contains("_ao") {
        return Some(GodotMaterialProperty::AmbientOcclusionTexture);
    }

    None
}

fn generate_godot_uid(length: usize) -> String {
    Alphanumeric.sample_string(&mut thread_rng(), length).to_lowercase()
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
