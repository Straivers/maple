use std::{fs, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=shaders/");

    compile_shaders();
}

const SHADER_DIR: &str = "shaders/";

fn compile_shaders() {
    let files = fs::read_dir(SHADER_DIR).unwrap().filter_map(|f| {
        if let Ok(entry) = f {
            let is_file = entry.file_type().map_or(false, |f| f.is_file());

            let path = entry.path();
            let extension = path.extension().unwrap_or_default();

            let is_shader = extension == "vert" || extension == "frag";

            if is_file && is_shader {
                Some((fs::read(path.as_path()).unwrap(), path))
            } else {
                None
            }
        } else {
            None
        }
    });

    let mut compiler = shaderc::Compiler::new().unwrap();
    let mut options = shaderc::CompileOptions::new().unwrap();
    options.set_target_spirv(shaderc::SpirvVersion::V1_5);
    options.set_generate_debug_info();
    options.set_warnings_as_errors();

    for shader in files {
        let kind = match shader.1.extension().unwrap().to_str().unwrap() {
            "vert" => shaderc::ShaderKind::Vertex,
            "frag" => shaderc::ShaderKind::Fragment,
            _ => panic!("Unsupported shader type"),
        };

        let binary = compiler
            .compile_into_spirv(
                std::str::from_utf8(&shader.0).unwrap(),
                kind,
                shader.1.file_name().unwrap().to_str().unwrap(),
                "main",
                Some(&options),
            )
            .unwrap();

        let mut dest_name = shader.1.file_stem().unwrap().to_str().unwrap().to_owned();
        dest_name.push('_');
        dest_name.push_str(shader.1.extension().unwrap().to_str().unwrap());

        let mut path = PathBuf::new();
        path.push(SHADER_DIR);
        path.push(dest_name);
        path.set_extension("spv");

        fs::write(path, &binary.as_binary_u8()).unwrap();
    }
}
