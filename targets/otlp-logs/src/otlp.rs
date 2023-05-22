#[path = ""]
pub(crate) mod logs {
    #[path = "./otlp/opentelemetry.proto.logs.v1.rs"]
    pub(crate) mod v1;
}

#[path = ""]
pub(crate) mod common {
    #[path = "./otlp/opentelemetry.proto.common.v1.rs"]
    pub(crate) mod v1;
}

#[path = ""]
pub(crate) mod resource {
    #[path = "./otlp/opentelemetry.proto.resource.v1.rs"]
    pub(crate) mod v1;
}

#[path = ""]
pub(crate) mod collector {
    #[path = ""]
    pub(crate) mod logs {
        #[path = "./otlp/opentelemetry.proto.collector.logs.v1.rs"]
        pub(crate) mod v1;
    }
}
