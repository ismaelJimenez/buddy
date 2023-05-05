use clap::{Parser, Subcommand};
use colored::*;
use serde::Deserialize;
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::io::{self, BufRead};
use std::path::Path;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use which::which;

fn new_package(package_name: &str) -> std::io::Result<()> {
    if !Path::new(package_name).exists() {
        println!(
            "    {} binary (application) `{}` package",
            "Created".green(),
            package_name
        );
        fs::create_dir(package_name)?;
        fs::create_dir(PathBuf::from(package_name).join("src"))?;

        let mut file = File::create(PathBuf::from(package_name).join("WORKSPACE"))?;
        file.write_all(b"")?;

        let mut file = File::create(PathBuf::from(package_name).join("Buddy.toml"))?;
        write!(
            file,
            r#"[package]
name = "{}"
version = "0.1.0"
edition = "2023"

[dependencies]"#,
            package_name
        )?;

        let mut file = File::create(PathBuf::from(package_name).join("src").join("BUILD"))?;

        write!(
            file,
            r#"load("@rules_cc//cc:defs.bzl", "cc_binary")

cc_binary(
    name = "{}",
    srcs = ["main.cc"],
)"#,
            package_name
        )?;

        let mut file = File::create(PathBuf::from(package_name).join("src").join("main.cc"))?;

        write!(
            file,
            r#"#include <ctime>
#include <string>
#include <iostream>

std::string get_greet(const std::string& who) {{
  return "Hello " + who;
}}

void print_localtime() {{
  std::time_t result = std::time(nullptr);
  std::cout << std::asctime(std::localtime(&result));
}}

int main(int argc, char** argv) {{
  std::string who = "world";
  if (argc > 1) {{
    who = argv[1];
  }}
  std::cout << get_greet(who) << std::endl;
  print_localtime();
  return 0;
}}"#
        )?;

        Ok(())
    } else {
        println!(
            "{}: destination `{}` already exixts",
            "error".red(),
            package_name
        );
        Ok(())
    }
}

fn build(bazel_bin: &PathBuf, args: &[String]) -> Result<(), Box<dyn Error>> {
    let mut cmd = Command::new(bazel_bin);

    cmd.arg("--output_base=target/build");
    cmd.arg("build");
    cmd.arg("--symlink_prefix=target/");

    if args.len() != 0 {
        for arg in args {
            cmd.arg(arg);
        }
    } else {
        cmd.arg("//src/...");
    }

    let mut child = cmd
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to execute command");

    let stderr = child.stderr.take().unwrap();
    let reader = io::BufReader::new(stderr);

    for line in reader.lines() {
        let line = line.unwrap();
        if line.starts_with("INFO:") {
            let (_, message) = line.split_at(6);
            println!("{} {}", "INFO:".green(), message);
        } else {
            println!("{}", line);
        }
    }

    // Not sure why is still being generated. Eitherway, we get rid of it.
    let folder_path = Path::new("bazel-out");
    if folder_path.exists() {
        fs::remove_dir_all(folder_path).expect("Failed to delete folder");
    }

    Ok(())
}

fn run(bazel_bin: &PathBuf, args: &[String], config: &Config) -> Result<(), Box<dyn Error>> {
    let mut cmd = Command::new(bazel_bin);

    cmd.arg("--output_base=target/build");
    cmd.arg("run");
    cmd.arg("--symlink_prefix=target/");

    if args.len() != 0 {
        for arg in args {
            cmd.arg(arg);
        }
    } else {
        cmd.arg(format!("//src:{}", config.package.name));
    }

    let mut child = cmd
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to execute command");

    let stderr = child.stderr.take().unwrap();
    let reader = io::BufReader::new(stderr);

    for line in reader.lines() {
        let line = line.unwrap();
        if line.starts_with("INFO:") {
            let (_, message) = line.split_at(6);
            println!("{} {}", "INFO:".green(), message);
        } else {
            println!("{}", line);
        }
    }

    // Not sure why is still being generated. Eitherway, we get rid of it.
    let folder_path = Path::new("bazel-out");
    if folder_path.exists() {
        fs::remove_dir_all(folder_path).expect("Failed to delete folder");
    }

    Ok(())
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new buddy package
    New { path: String },

    /// Compile the current package
    Build { targets: Vec<String> },

    /// Run a binary or example of the local package
    Run { targets: Vec<String> },
}

#[derive(Debug, Deserialize)]
struct Package {
    name: String,
    version: String,
    edition: String,
}

#[derive(Debug, Deserialize)]
struct Dependency {
    name: String,
    version: String,
}

#[derive(Debug, Deserialize)]
struct Config {
    package: Package,
    dependencies: std::collections::BTreeMap<String, Dependency>,
}

fn main() {
    let cli = Cli::parse();

    let bazel_bin = match which("bazel") {
        Ok(path) => path,
        Err(_) => panic!("Bazel binary not found. See https://bazel.build/install"),
    };

    let file_path = "Buddy.toml";
    let config: Config = match fs::read_to_string(file_path) {
        Ok(content) => toml::from_str(&content).unwrap(),
        Err(_) => Config {
            package: Package {
                name: "default".to_string(),
                version: "0.1.0".to_string(),
                edition: "2021".to_string(),
            },
            dependencies: std::collections::BTreeMap::new(),
        },
    };

    println!("{:#?}", config);

    match &cli.command {
        Commands::New { path } => new_package(&path).unwrap(),
        Commands::Build { targets } => build(&bazel_bin, &targets).unwrap(),
        Commands::Run { targets } => run(&bazel_bin, &targets, &config).unwrap(),
    }
}
