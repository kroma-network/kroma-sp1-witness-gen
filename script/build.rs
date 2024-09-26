use sp1_build::{build_program_with_args, BuildArgs};

/// Build a program for the zkVM.
#[allow(dead_code)]
fn build_zkvm_program(program_path: &str, program_name: &str, out_dir: &str) {
    build_program_with_args(
        program_path,
        BuildArgs {
            output_directory: out_dir.to_string(),
            elf_name: format!("{}-elf", program_name),
            ..Default::default()
        },
    );
}

fn main() {
    build_zkvm_program("../program", "fault-proof", "program/elf");
}
