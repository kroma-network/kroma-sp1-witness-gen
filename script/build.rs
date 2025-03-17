use std::process::Command;

/// Build a native program.
fn build_native_program(program: &str) {
    let status = Command::new("cargo")
        .args([
            "build",
            "--workspace",
            "--bin",
            program,
            "--profile",
            "release-client-lto",
            "--features",
            "kroma",
        ])
        .status()
        .expect("Failed to execute cargo build command");

    if !status.success() {
        panic!("Failed to build {}", program);
    }

    println!("cargo:warning={} built with release-client-lto profile", program);
}

/// Build the native host runner to a separate target directory to avoid build lockups.
fn build_native_host_runner() {
    let metadata =
        cargo_metadata::MetadataCommand::new().exec().expect("Failed to get cargo metadata");
    let target_dir = metadata.target_directory.join("native_host_runner");

    let status = Command::new("cargo")
        .args([
            "build",
            "--workspace",
            "--bin",
            "native_host_runner",
            "--release",
            "--target-dir",
            target_dir.as_ref(),
        ])
        .status()
        .expect("Failed to execute cargo build command");
    if !status.success() {
        panic!("Failed to build native_host_runner");
    }

    println!("cargo:warning=native_host_runner built with release profile",);
}

fn main() {
    let program_name = "fault-proof";
    // Note: Don't comment this out, because the Docker program depends on the native program
    // for range being built.
    build_native_program(program_name);

    // Note: Don't comment this out, because the Docker program depends on the native host runner
    // being built.
    build_native_host_runner();
}
