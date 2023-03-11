use std::collections::HashSet;
use std::error::Error;
use std::process::Command;

fn main() -> Result<(), Box<dyn Error>> {

    // Tell the build script to only run again if we change our source shaders
    println!("cargo:rerun-if-changed=shaders");

    // Create destination path if necessary
    for entry in std::fs::read_dir("shaders")? {
        let entry = entry?;

        if entry.file_type()?.is_file() {
            let in_path = entry.path();
            let path_str = in_path.to_str().unwrap();
            let stat = Command::new("glslc").args(&[path_str, "--target-env=vulkan1.2", "-o"])
                .arg(&format!("spv/{}.spv", entry.file_name().into_string().unwrap()))
                .status()
                .expect(&*format!("Failed to invoke glslc"));

            if !stat.success() {
                panic!("Failed to compile shader {:?}.", path_str);
            }

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

