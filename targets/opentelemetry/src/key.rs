use opentelemetry_api::Key;

pub(crate) fn to_key(key: emit_core::key::Key) -> Key {
    if let Some(key) = key.as_static_str() {
        Key::from_static_str(key)
    } else {
        Key::new(key.as_str().to_owned())
    }
}
