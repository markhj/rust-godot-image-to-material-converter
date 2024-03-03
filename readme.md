![Godot image to material converter banner](https://res.cloudinary.com/drfztvfdh/image/upload/v1709453938/Github/godot-image-to-material_mgrft2.jpg)

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

## 🚧 Installation

Clone the repo:

````bash
git clone https://github.com/markhj/XXXXXX
````

Build the code with your Rust tool/IDE.

## 🌎 Environment variable

Depending on your operating system, you will have to update your ``PATH``
environment variable, so it points to the location where the compiled
executable resides.

## ▶️ Usage

Navigate to the directory, where your ``.tiff`` files are.

Replace ``[program]`` with the name of the executable.

````bash
[program] *.tiff
````

## 🚚 Todo

* Generate Godot ``StandardMaterial3D`` material automatically
* Pipeline which builds executables for various platforms
* Installer which registers the environment variable
* More output formats
* Config file with instructions on how to convert the files
* Improve argument/option parsing in CLI
* Options
  * Allow overwrites
  * Put into folder
  * Change filename according to pattern
