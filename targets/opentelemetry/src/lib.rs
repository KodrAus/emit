use std::time::Duration;

use opentelemetry_api::logs::Logger as _;
use opentelemetry_sdk::logs::Logger;

mod record;

pub fn logger(logger: Logger) -> OpenTelemetryTarget {
    OpenTelemetryTarget(logger)
}

pub struct OpenTelemetryTarget(Logger);

impl emit_core::target::Target for OpenTelemetryTarget {
    fn emit_event<P: emit_core::props::Props>(&self, evt: &emit_core::event::Event<P>) {
        self.0.emit(record::to_record(evt));
    }

    // TODO: Respect the timeout
    fn blocking_flush(&self, _: Duration) {
        if let Some(provider) = self.0.provider() {
            let _ = provider.force_flush();
        }
    }
}
