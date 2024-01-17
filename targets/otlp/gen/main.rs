fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = prost_build::Config::new();

    config.out_dir("../src/data/generated");

    config.compile_protos(
        &[
            "./opentelemetry/proto/collector/logs/v1/logs_service.proto",
            "./opentelemetry/proto/collector/trace/v1/trace_service.proto",
            "./opentelemetry/proto/collector/metrics/v1/metrics_service.proto",
        ],
        &["./"],
    )?;

    Ok(())
}
