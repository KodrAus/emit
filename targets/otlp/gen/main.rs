fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = prost_build::Config::new();

    config.out_dir("../src/proto");

    config.compile_protos(
        &[
            "./opentelemetry/proto/collector/logs/v1/logs_service.proto",
            "./opentelemetry/proto/collector/trace/v1/trace_service.proto",
        ],
        &["./"],
    )?;

    Ok(())
}
