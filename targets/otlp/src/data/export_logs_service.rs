use sval_derive::Value;

use super::{InstrumentationScope, LogRecord, Resource};

#[derive(Value)]
pub struct ExportLogsServiceRequest<'a, RL: ?Sized = [ResourceLogs<'a>]> {
    #[sval(index = 1)]
    pub resource_logs: &'a RL,
}

#[derive(Value)]
pub struct ResourceLogs<'a, R: ?Sized = Resource<'a>, SL: ?Sized = [ScopeLogs<'a>]> {
    #[sval(index = 1)]
    pub resource: &'a R,
    #[sval(index = 2)]
    pub scope_logs: &'a SL,
    #[sval(index = 3)]
    pub schema_url: &'a str,
}

#[derive(Value)]
pub struct ScopeLogs<'a, IS: ?Sized = InstrumentationScope<'a>, LR: ?Sized = &'a [LogRecord<'a>]> {
    #[sval(index = 1)]
    pub scope: &'a IS,
    #[sval(index = 2)]
    pub log_records: &'a LR,
    #[sval(index = 3)]
    pub schema_url: &'a str,
}
