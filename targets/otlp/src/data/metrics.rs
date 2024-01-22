use emit_batcher::BatchError;

use super::{AnyValue, MessageFormatter, MessageRenderer, PreEncoded};

pub(crate) struct EventEncoder {
    pub name: Box<MessageFormatter>,
}

impl EventEncoder {
    pub(crate) fn encode_event(
        &self,
        evt: &emit::event::Event<impl emit::props::Props>,
    ) -> Option<PreEncoded> {
        todo!()
    }
}

pub(crate) fn encode_request(
    resource: Option<&PreEncoded>,
    scope: Option<&PreEncoded>,
    log_records: &[PreEncoded],
) -> Result<PreEncoded, BatchError<Vec<PreEncoded>>> {
    todo!()
}

#[cfg(feature = "decode_responses")]
pub(crate) fn decode_response(body: Result<&[u8], &[u8]>) {
    use prost::Message;

    match body {
        Ok(body) => {
            let response =
                crate::data::generated::collector::metrics::v1::ExportMetricsServiceResponse::decode(
                    body,
                )
                .unwrap();

            emit::debug!(rt: emit::runtime::internal(), "received {#[emit::as_debug] response}");
        }
        Err(body) => {
            let response =
                crate::data::generated::collector::metrics::v1::ExportMetricsPartialSuccess::decode(body)
                    .unwrap();

            emit::warn!(rt: emit::runtime::internal(), "received {#[emit::as_debug] response}");
        }
    }
}
