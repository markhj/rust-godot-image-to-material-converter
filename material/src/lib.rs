use std::{fs, io, thread};
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::time::Duration;
use rand::distributions::{Alphanumeric, DistString};
use rand::thread_rng;
use regex::Regex;

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
    uid: String,
    short_uid: String,
    source_file: String,
    property: GodotMaterialProperty,
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
pub fn generate(files: Vec<PathBuf>) -> Result<String, String> {
    let files_found = scan_for_import_files(&files);

    // Abort, if the number of .import files doesn't match number of converted files
    // This means Godot hasn't seen all files yet, or has failed to import some
    // In this case, we don't want to just go ahead and create the material
    if files_found.len() < files.len() {
        return Err(String::from("Will not wait any longer for .import files"));
    }

    // Create the list of materials discovered
    let uid_mapping = compile_material_mapping(&files_found);

    // The number of discovered materials must match the number of files
    // Otherwise, similarly to above, we risk creating a material with missing
    // properties and attributes
    if uid_mapping.len() != files_found.len() {
        return Err(String::from("UID mapping does not match number of files."));
    }

    // Generate the data and save the material file
    Ok(generate_material(&uid_mapping))
}

/// This method generates the data of the material (.tres) file, using a
/// series private helper functions.
///
/// Lastly, it saves the file - or dies trying.
fn generate_material(mapping: &Vec<GodotMaterialMapping>) -> String {
    let mut mat_data = String::new();

    generate_header(&mut mat_data);
    generate_ext_resources(&mut mat_data, mapping);
    generate_resources(&mut mat_data, mapping);

    mat_data
}

/// In order to generate the Godot material, we need to peek into the .import
/// files which Godot creates in pairs with media files such as images.
/// The .import files contain vital information such as the resource path and UID
///
/// We keep scanning (using thread sleep) because Godot doesn't immediately detect
/// new files and create the .import pairs. In fact, the user will usually have to
/// focus on the Godot window.
///
/// Once all files are found, we exit the loop and return the list
fn scan_for_import_files(files: &Vec<PathBuf>) -> Vec<PathBuf> {
    let mut files_found: Vec<PathBuf>;
    let mut iters: i8 = 0;
    let max_iters: i8 = 100;

    loop {
        files_found = Vec::new();

        // Loop over the converted files, and see if there exists an pairing
        // with the extension .import
        for f in files {
            let ext = f.extension().unwrap().to_str().unwrap();
            let import_path = f.clone().with_extension(format!("{}.import", ext));
            if import_path.exists() {
                files_found.push(import_path);
            }
        }

        // If the number of .import files doesn't match the number of converted files,
        // we will put the thread to sleep for a second, and then try again
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

        // If max iterations (time-out) is reacted, or if sufficient .import
        // files have been discovered, we break the loop
        if iters >= max_iters || files_found.len() == files.len() {
            break;
        }
    }

    files_found
}

/// Look through the contents of the .import files in order to extract the resources'
/// UID, local path, etc.
fn compile_material_mapping(files_found: &Vec<PathBuf>) -> Vec<GodotMaterialMapping> {
    let mut uid_mapping: Vec<GodotMaterialMapping> = Vec::new();

    // Set up the two regular expressions used to extract UID and source_file properties
    let uid_regex = Regex::new(r#"\buid="uid://([^"]+)""#).unwrap();
    let sf_regex = Regex::new(r#"\bsource_file="(res://[^"]+)""#).unwrap();

    // Iterate through the discovered .import files
    for import_file in files_found {
        let mut file = File::open(import_file).expect("Failed to open import file");
        let mut data = String::new();

        let mut uid: Option<String> = None;
        let mut source_file: Option<String> = None;

        // Load the file contents into the file variable
        file.read_to_string(&mut data).expect("Failed to read lines from import file");

        for line in data.lines() {
            // If the line matches the UID property, we extract the value
            if let Some(captures) = uid_regex.captures(&line) {
                if let Some(value) = captures.get(1).map(|m| m.as_str()) {
                    uid = Some(value.to_owned());
                }
            }

            // If the line matches source_file, we extract the value
            if let Some(captures) = sf_regex.captures(&line) {
                if let Some(value) = captures.get(1).map(|m| m.as_str()) {
                    source_file = Some(value.to_owned());
                }
            }
        }

        // Figure out which (if any) property the filename maps to
        // For instance if it contains "albedo" it maps to the AlbedoTexture property
        let property: Option<GodotMaterialProperty> = get_godot_property(import_file);
        if uid.is_some() && source_file.is_some() {
            uid_mapping.push(GodotMaterialMapping {
                property: property.unwrap(),
                uid: uid.unwrap(),
                source_file: source_file.unwrap(),
                short_uid: format!("{}_{}", uid_mapping.len() + 1, generate_godot_uid(5)),
            });
        }
    }

    uid_mapping
}

/// Generate the first line in the Godot material file
fn generate_header(mat_data: &mut String) {
    mat_data.push_str(
        format!("[gd_resource type=\"StandardMaterial3D\" format=3 uid=\"uid://{}\"]\n\n",
                generate_godot_uid(12)
        ).as_str()
    );
}

/// Generate the ext_resource tags for the material file
/// The ext_resource are references to the .import files
/// They are assigned an arbitrary "short uid"
fn generate_ext_resources(mat_data: &mut String, uid_mapping: &Vec<GodotMaterialMapping>) {
    for res in uid_mapping {
        mat_data.push_str(format!(
            "[ext_resource type=\"Texture2D\" path=\"{}\" uid=\"uid://{}\" id=\"{}\"]\n",
            res.source_file,
            res.uid,
            res.short_uid
        ).as_str());
    }
}

/// Generate the [resource] tag in the material file
/// This mainly consists of bool, numeric values and references to the ``ext_resource``s.
fn generate_resources(mat_data: &mut String, uid_mapping: &Vec<GodotMaterialMapping>) {
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
}

/// Based on the filename, this function will return which ``GodotMaterialProperty``
/// is a fitting choice
///
/// If no choice is made, it returns ``None``.
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

/// Generate the random Godot-like UID
fn generate_godot_uid(length: usize) -> String {
    Alphanumeric.sample_string(&mut thread_rng(), length).to_lowercase()
}
