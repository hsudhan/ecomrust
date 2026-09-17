fn main() -> Result<(), Box<dyn std::error::Error>> {
    // No system protoc on this machine — use the vendored binary.
    let protoc = protoc_bin_vendored::protoc_bin_path()?;
    std::env::set_var("PROTOC", protoc);

    // Client stubs are generated too: shared with the smoke-test client and
    // kept in lockstep with the server (claude.md rule #1).
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos(&["proto/ecom.proto"], &["proto"])?;
    Ok(())
}
