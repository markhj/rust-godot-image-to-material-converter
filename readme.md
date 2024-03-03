![Godot image to material converter banner](https://res.cloudinary.com/drfztvfdh/image/upload/v1709453938/Github/godot-image-to-material_mgrft2.jpg)

![GitHub Tag](https://img.shields.io/github/v/tag/markhj/rust-godot-image-to-material-converter?label=version)
[![Codacy Badge](https://app.codacy.com/project/badge/Grade/41f3cda5f9354dab94854950903f154f)](https://app.codacy.com/gh/markhj/rust-godot-image-to-material-converter/dashboard?utm_source=gh&utm_medium=referral&utm_content=&utm_campaign=Badge_grade)
[![Minimum rustc version](https://img.shields.io/badge/rustc-1.74+-lightgray.svg)](https://github.com/markhj/rust-config-reader)
![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg?label=license)

**Image to Material Converter for Godot** is a CLI application which converts TIFF images to JPEG, and optionally generates Godot materials.

The project was created to make the process of converting images to supported types, as well as generating PBR
materials from them smoother, faster and easier.

## 📢 Requirements

> ⚠️ The project is still in early development phase, so we don't yet offer compiled versions.
> This is on the to-do list.

You are required to have a functional Rust/Cargo build system.

The code is written in **Rust v. 1.74**.

## Dependencies
* ``Image 0.24.9``
* ``Regex 1.10.3``
* ``Clap 4.5.1``

## 🚧 Installation

Clone the repo:

````bash
git clone https://github.com/markhj/rust-godot-image-to-material-converter
````

Build the code with your Rust tool/IDE.

## 🌎 Environment variable

Depending on your operating system, you will have to update your ``PATH``
environment variable, so it points to the location where the compiled
executable resides.

## ▶️ Usage

Navigate to the directory, where your ``.tiff`` files are.

````bash
gim *.tiff
````

### Allow overwrites

If you want to overwrite existing files when converting, use the ``--allow-overwrites`` or ``-a`` flag:

````bash
gim *.tiff --a
````

### Destination directory

To put the files into a subdirectory, use ``--destination`` or ``-d``:

````bash
gim *.tiff -d"subfolder"
````

### Preview

To see what the program intends to do, before actually carrying out any changes, you use
the ``--preview`` or ``-p`` flag:

````bash
gim *.tiff -p
````

### Generate material

To generate a Godot ``StandardMaterial3D`` use:

````bash
gim *.tiff -m
````

## 🚚 Todo

* Generate Godot ``StandardMaterial3D`` material automatically
* Pipeline which builds executables for various platforms
* Installer which registers the environment variable
* More output formats
* Config file with instructions on how to convert the files
* Options
  * Change filename according to pattern
