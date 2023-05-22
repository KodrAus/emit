fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = prost_build::Config::new();

    config.out_dir("../src/otlp");

    config.compile_protos(
        &["./opentelemetry/proto/collector/logs/v1/logs_service.proto"],
        &["./"],
    )?;

    Ok(())
}
