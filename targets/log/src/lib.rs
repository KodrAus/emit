use std::{collections::HashMap, ops::ControlFlow, time::Duration};

pub fn global_logger() -> GlobalLogger {
    GlobalLogger {}
}

pub struct GlobalLogger {}

impl emit::Emitter for GlobalLogger {
    fn emit<P: emit::Props>(&self, evt: &emit::Event<P>) {
        log::logger().log(
            &log::RecordBuilder::new()
                .args(format_args!("{}", evt.msg()))
                .level(
                    match evt
                        .props()
                        .pull::<emit::Level, _>(emit::well_known::LVL_KEY)
                    {
                        Some(emit::Level::Debug) => log::Level::Debug,
                        Some(emit::Level::Info) => log::Level::Info,
                        Some(emit::Level::Warn) => log::Level::Warn,
                        Some(emit::Level::Error) => log::Level::Error,
                        None => log::Level::Info,
                    },
                )
                .module_path(Some(&evt.module().to_cow().into_owned()))
                .key_values(&EmitSource::collect(evt.props()))
                .build(),
        );
    }

    fn blocking_flush(&self, _: Duration) {
        // NOTE: Doesn't respect the timeout
        log::logger().flush();
    }
}

// TODO: Avoid allocating when possible
struct EmitSource(HashMap<emit::Str<'static>, emit::value::OwnedValue>);

impl EmitSource {
    fn collect<P: emit::Props>(props: P) -> Self {
        let mut map = HashMap::new();

        props.for_each(|key, value| {
            map.insert(key.to_owned(), value.to_owned());

            ControlFlow::Continue(())
        });

        EmitSource(map)
    }
}

impl log::kv::Source for EmitSource {
    fn visit<'kvs>(
        &'kvs self,
        visitor: &mut dyn log::kv::VisitSource<'kvs>,
    ) -> Result<(), log::kv::Error> {
        for (key, value) in &self.0 {
            visitor.visit_pair(
                log::kv::Key::from_str(key.as_str()),
                log::kv::Value::from_sval(value),
            )?;
        }

        Ok(())
    }
}
