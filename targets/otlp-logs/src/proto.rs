#[path = ""]
pub(crate) mod logs {
    #[path = "./proto/opentelemetry.proto.logs.v1.rs"]
    pub(crate) mod v1;
}

#[path = ""]
pub(crate) mod common {
    #[path = "./proto/opentelemetry.proto.common.v1.rs"]
    pub(crate) mod v1;
}

#[path = ""]
pub(crate) mod resource {
    #[path = "./proto/opentelemetry.proto.resource.v1.rs"]
    pub(crate) mod v1;
}

#[path = ""]
pub(crate) mod collector {
    #[path = ""]
    pub(crate) mod logs {
        #[path = "./proto/opentelemetry.proto.collector.logs.v1.rs"]
        pub(crate) mod v1;
    }
}
