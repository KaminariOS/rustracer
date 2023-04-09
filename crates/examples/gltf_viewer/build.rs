use std::error::Error;
use std::io;
use std::process::{Command, Output};

fn main() -> Result<()> {
    // Tell the build script to only run again if we change our source shaders
    println!("cargo:rerun-if-changed=shaders");

    // Create destination path if necessary
    for entry in std::fs::read_dir("shaders")? {
        let entry = entry?;

        if entry.file_type()?.is_file() {
            let in_path = entry.path();
            let path_str = in_path.to_str().unwrap();
            let stat = Command::new("glslc")
                .args(&[path_str, "--target-env=vulkan1.2", "-o"])
                .arg(&format!(
                    "../../../spv/{}.spv",
                    entry.file_name().into_string().unwrap()
                ))
                .output();

            handle_program_result(stat);
            // if !stat.success() {
            //     panic!("Failed to compile shader {:?}.", path_str);
            // }

            // Support only vertex and fragment shaders currently
            //     let shader_type = in_path.extension().and_then(|ext| {
            //         match ext.to_string_lossy().as_ref() {
            //             "vert" => Some(ShaderType::Vertex),
            //             "frag" => Some(ShaderType::Fragment),
            //             _ => None,
            //         }
            //     });
            //
            //     if let Some(shader_type) = shader_type {
            //         use std::io::Read;
            //
            //         let source = std::fs::read_to_string(&in_path)?;
            //         let mut compiled_file = glsl_to_spirv::compile(&source, shader_type)?;
            //
            //         let mut compiled_bytes = Vec::new();
            //         compiled_file.read_to_end(&mut compiled_bytes)?;
            //
            //         let out_path = format!(
            //             "assets/gen/shaders/{}.spv",
            //             in_path.file_name().unwrap().to_string_lossy()
            //         );
            //
            //         std::fs::write(&out_path, &compiled_bytes)?;
            //     }
        }
    }

    Ok(())
}

use io::Result;
fn handle_program_result(result: Result<Output>) {
    match result {
        Ok(output) => {
            if output.status.success() {
                println!("Shader compilation succedeed.");
                print!(
                    "stdout: {}",
                    String::from_utf8(output.stdout)
                        .unwrap_or_else(|_| "Failed to print program stdout".to_string())
                );
            } else {
                eprintln!("Shader compilation failed. Status: {}", output.status);
                eprint!(
                    "stdout: {}",
                    String::from_utf8(output.stdout)
                        .unwrap_or_else(|_| "Failed to print program stdout".to_string())
                );
                eprint!(
                    "stderr: {}",
                    String::from_utf8(output.stderr)
                        .unwrap_or_else(|_| "Failed to print program stderr".to_string())
                );
                panic!("Shader compilation failed. Status: {}", output.status);
            }
        }
        Err(error) => {
            panic!("Failed to compile shader. Cause: {}", error);
        }
    }
}
