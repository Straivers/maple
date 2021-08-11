use std::fs;

const VERTEX_SHADER_SOURCE: &str = include_str!("shaders/tri.vert");
const FRAGMENT_SHADER_SOURCE: &str = include_str!("shaders/tri.frag");

fn main() {
    println!("cargo:rerun-if-changed=shaders/");

    let mut compiler = shaderc::Compiler::new().unwrap();
    let mut options = shaderc::CompileOptions::new().unwrap();
    options.set_target_spirv(shaderc::SpirvVersion::V1_5);
    options.set_generate_debug_info();
    options.set_warnings_as_errors();

    let vertex_shader_binary = compiler
        .compile_into_spirv(
            VERTEX_SHADER_SOURCE,
            shaderc::ShaderKind::Vertex,
            "tri.vert",
            "main",
            Some(&options),
        )
        .unwrap();
    fs::write("shaders/tri.vert.spv", &vertex_shader_binary.as_binary_u8()).unwrap();

    let fragment_shader_binary = compiler
        .compile_into_spirv(
            FRAGMENT_SHADER_SOURCE,
            shaderc::ShaderKind::Fragment,
            "tri.vert",
            "main",
            Some(&options),
        )
        .unwrap();
    fs::write("shaders/tri.frag.spv", &fragment_shader_binary.as_binary_u8()).unwrap();
}
