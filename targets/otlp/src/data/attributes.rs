use std::{collections::HashSet, ops::ControlFlow};

use super::{EmitValue, KeyValue};

pub(crate) fn stream_attributes<'sval>(
    stream: &mut (impl sval::Stream<'sval> + ?Sized),
    props: &'sval impl emit_core::props::Props,
    mut for_each: impl FnMut(&emit_core::key::Key, &emit_core::value::Value) -> bool,
) -> sval::Result {
    stream.seq_begin(None)?;

    let mut seen = HashSet::new();
    props.for_each(|k, v| {
        if !for_each(&k, &v) && seen.insert(k.to_cow()) {
            stream
                .seq_value_begin()
                .map(|_| ControlFlow::Continue(()))
                .unwrap_or(ControlFlow::Break(()))?;

            sval_ref::stream_ref(
                &mut *stream,
                KeyValue {
                    key: k,
                    value: EmitValue(v),
                },
            )
            .map(|_| ControlFlow::Continue(()))
            .unwrap_or(ControlFlow::Break(()))?;

            stream
                .seq_value_end()
                .map(|_| ControlFlow::Continue(()))
                .unwrap_or(ControlFlow::Break(()))?;
        }

        ControlFlow::Continue(())
    });

    stream.seq_end()
}
