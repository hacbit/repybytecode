use super::common::*;
use super::decompile::*;
use super::parse_opcode::*;
use colored::Colorize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub struct App {
    // files order
    files: Vec<PathBuf>,
    // the resource of the bytecode
    resources: HashMap<PathBuf, CodeObjectMap>,
    // the out file name
    output_files: Vec<PathBuf>,
    // the output of the decompiled code
    output: Vec<Result<DecompiledCode>>,
}

#[allow(unused)]
impl App {
    pub fn new() -> Self {
        Self {
            files: Vec::new(),
            resources: HashMap::new(),
            output_files: Vec::new(),
            output: Vec::new(),
        }
    }

    pub fn insert_resource<P: Into<PathBuf>>(&mut self, path: P) -> &mut Self {
        let path = path.into();
        if path.is_file() {
            let resource = fs::read_to_string(&path).unwrap();
            let code_object_map = resource.parse().unwrap();
            if self
                .resources
                .insert(path.clone(), code_object_map)
                .is_none()
            {
                self.files.push(path);
            } else {
                println!(
                    "{}",
                    format!("Warning: {} is already in the resources", path.display())
                        .bright_yellow()
                );
            }
        } else {
            println!(
                "{}",
                format!("Warning: {} is not a file", path.display()).bright_yellow()
            );
        }
        self
    }

    pub fn insert_resources<P: Into<PathBuf>>(&mut self, paths: Vec<P>) -> &mut Self {
        for path in paths {
            self.insert_resource(path);
        }
        self
    }

    pub fn with_file<P: Into<PathBuf>>(&mut self, path: P) -> &mut Self {
        let path = path.into();
        if !path.exists() && !self.output_files.contains(&path) {
            self.output_files.push(path);
        } else {
            println!(
                "{}",
                format!("Warning: {} is already in the output files", path.display())
                    .bright_yellow()
            );
        }
        self
    }

    pub fn with_files<P: Into<PathBuf>>(&mut self, paths: Vec<P>) -> &mut Self {
        for path in paths {
            self.with_file(path);
        }
        self
    }

    pub fn run(&mut self) -> Result<&mut Self> {
        for (path, code_object_map) in &self.resources {
            println!("{}", format!("Try to decompile {}", path.display()).green());
            let decompiled_result = code_object_map.decompile();
            self.output.push(decompiled_result);
        }
        Ok(self)
    }

    pub fn output(&mut self) -> Result<()> {
        self.output
            .iter_mut()
            .enumerate()
            .for_each(|(i, decompiled_result)| {
                if let Some(file) = self.output_files.get(i) {
                    if let Ok(decompiled_code) = decompiled_result {
                        decompiled_code.iter().write_file(file).unwrap();
                    } else {
                        eprintln!(
                            "{}",
                            format!("The file {} decompiled failed", self.files[i].display())
                                .bright_red()
                        );
                    }
                } else {
                    if let Ok(decompiled_code) = decompiled_result {
                        decompiled_code.iter().write_console().unwrap();
                    } else {
                        eprintln!(
                            "{}",
                            format!("The file {} decompiled failed", self.files[i].display())
                                .bright_red()
                        );
                    }
                }
            });
        Ok(())
    }
}