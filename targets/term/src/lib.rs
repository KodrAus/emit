#![feature(proc_macro_hygiene, stmt_expr_attributes)]

use core::{fmt, str, time::Duration};
use std::{cell::RefCell, io::Write, sync::Mutex};

use emit::{metrics::MetricKind, well_known::WellKnown, Event};
use emit_metrics::{Bucketing, Histogram, MetricsCollector};
use termcolor::{Buffer, BufferWriter, Color, ColorChoice, ColorSpec, WriteColor};

pub fn stdout() -> Stdout {
    Stdout {
        writer: BufferWriter::stdout(ColorChoice::Auto),
        metrics_collector: None,
    }
}

pub struct Stdout {
    writer: BufferWriter,
    metrics_collector: Option<Mutex<MetricsCollector>>,
}

impl Stdout {
    pub fn plot_metrics_by_time(mut self, bucket_size: Duration) -> Self {
        self.metrics_collector = Some(Mutex::new(MetricsCollector::new(Bucketing::ByTime(
            bucket_size,
        ))));
        self
    }

    pub fn plot_metrics_by_count(mut self, nbuckets: usize) -> Self {
        self.metrics_collector = Some(Mutex::new(MetricsCollector::new(Bucketing::ByCount(
            nbuckets,
        ))));
        self
    }
}

thread_local! {
    static STDOUT: RefCell<Option<Buffer>> = RefCell::new(None);
}

fn with_shared_buf(writer: &BufferWriter, with_buf: impl FnOnce(&BufferWriter, &mut Buffer)) {
    STDOUT.with(|buf| {
        match buf.try_borrow_mut() {
            // If there are no overlapping references then use the cached buffer
            Ok(mut slot) => {
                match &mut *slot {
                    // If there's a cached buffer then clear it and print using it
                    Some(buf) => {
                        buf.clear();
                        with_buf(&writer, buf);
                    }
                    // If there's no cached buffer then create one and use it
                    // It'll be cached for future callers on this thread
                    None => {
                        let mut buf = writer.buffer();
                        with_buf(&writer, &mut buf);

                        *slot = Some(buf);
                    }
                }
            }
            // If there are overlapping references then just create a
            // buffer on-demand to use
            Err(_) => {
                with_buf(&writer, &mut writer.buffer());
            }
        }
    });
}

impl emit::emitter::Emitter for Stdout {
    fn emit<P: emit::Props>(&self, evt: &emit::Event<P>) {
        if let Some(ref metrics_collector) = self.metrics_collector {
            if metrics_collector.lock().unwrap().record_metric(evt) {
                return;
            }
        }

        with_shared_buf(&self.writer, |writer, buf| print_event(writer, buf, evt));
    }

    fn blocking_flush(&self, _: Duration) {
        if let Some(ref metrics_collector) = self.metrics_collector {
            for (metric, histogram) in metrics_collector.lock().unwrap().take_sums() {
                if histogram.is_empty() {
                    continue;
                }

                let histogram = histogram.compute();

                with_shared_buf(&self.writer, |writer, buf| {
                    print_histogram(writer, buf, &*metric, MetricKind::Sum, &histogram);
                });
            }
        }
    }
}

fn trace_id_color(trace_id: &emit::TraceId) -> u8 {
    let mut hash = 0;

    for b in trace_id.to_u128().to_le_bytes() {
        hash ^= b;
    }

    hash
}

fn span_id_color(span_id: &emit::SpanId) -> u8 {
    let mut hash = 0;

    for b in span_id.to_u64().to_le_bytes() {
        hash ^= b;
    }

    hash
}

fn level_color(level: &emit::Level) -> Option<u8> {
    match level {
        emit::Level::Debug => Some(244),
        emit::Level::Info => None,
        emit::Level::Warn => Some(202),
        emit::Level::Error => Some(124),
    }
}

fn hex_slice<'a>(hex: &'a [u8]) -> impl fmt::Display + 'a {
    struct HexSlice<'a>(&'a [u8]);

    impl<'a> fmt::Display for HexSlice<'a> {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str(str::from_utf8(&self.0[..4]).unwrap())
        }
    }

    HexSlice(hex)
}

struct LocalTime {
    h: u8,
    m: u8,
    s: u8,
    ms: u16,
}

fn local_ts(ts: emit::Timestamp) -> Option<LocalTime> {
    // See: https://github.com/rust-lang/rust/issues/27970
    //
    // On Linux and OSX, this will fail to get the local offset in
    // any multi-threaded program. It needs to be fixed in the standard
    // library and propagated through libraries like `time`. Until then,
    // you probably won't get local timestamps outside of Windows.
    let local = time::OffsetDateTime::from_unix_timestamp_nanos(
        ts.to_unix_time().as_nanos().try_into().ok()?,
    )
    .ok()?;
    let local = local.checked_to_offset(time::UtcOffset::local_offset_at(local).ok()?)?;

    let (h, m, s, ms) = local.time().as_hms_milli();

    Some(LocalTime { h, m, s, ms })
}

fn write_timestamp(buf: &mut Buffer, ts: emit::Timestamp) {
    if let Some(LocalTime { h, m, s, ms }) = local_ts(ts) {
        write_plain(
            buf,
            format_args!("{:>02}:{:>02}:{:>02}.{:>03}", h, m, s, ms),
        );
    } else {
        write_plain(buf, format_args!("{:.0}", ts));
    }
}

struct FriendlyDuration {
    pub value: u128,
    pub unit: &'static str,
}

fn friendly_duration(duration: Duration) -> FriendlyDuration {
    const NANOS_PER_MICRO: u128 = 1000;
    const NANOS_PER_MILLI: u128 = NANOS_PER_MICRO * 1000;
    const NANOS_PER_SEC: u128 = NANOS_PER_MILLI * 1000;
    const NANOS_PER_MIN: u128 = NANOS_PER_SEC * 60;

    let nanos = duration.as_nanos();

    if nanos < NANOS_PER_MICRO * 2 {
        FriendlyDuration {
            value: nanos,
            unit: "ns",
        }
    } else if nanos < NANOS_PER_MILLI * 2 {
        FriendlyDuration {
            value: nanos / NANOS_PER_MICRO,
            unit: "μs",
        }
    } else if nanos < NANOS_PER_SEC * 2 {
        FriendlyDuration {
            value: nanos / NANOS_PER_MILLI,
            unit: "ms",
        }
    } else if nanos < NANOS_PER_MIN * 2 {
        FriendlyDuration {
            value: nanos / NANOS_PER_SEC,
            unit: "s",
        }
    } else {
        FriendlyDuration {
            value: nanos / NANOS_PER_MIN,
            unit: "m",
        }
    }
}

fn write_duration(buf: &mut Buffer, duration: Duration) {
    let FriendlyDuration { value, unit } = friendly_duration(duration);

    write_fg(buf, value, NUMBER);
    write_fg(buf, unit, TEXT);
}

fn print_event(out: &BufferWriter, buf: &mut Buffer, evt: &emit::Event<impl emit::Props>) {
    if let Some(span_id) = evt.props().span_id() {
        if let Some(trace_id) = evt.props().trace_id() {
            let trace_id_color = trace_id_color(&trace_id);

            write_fg(buf, "▒", Color::Ansi256(trace_id_color));
            write_plain(buf, " ");
            write_plain(buf, hex_slice(&trace_id.to_hex()));
            write_plain(buf, " ");
        } else {
            write_plain(buf, "░      ");
        }

        let span_id_color = span_id_color(&span_id);

        write_fg(buf, "▓", Color::Ansi256(span_id_color));
        write_plain(buf, " ");
        write_plain(buf, hex_slice(&span_id.to_hex()));
        write_plain(buf, " ");
    }

    if let Some(extent) = evt.extent() {
        write_timestamp(buf, *extent.to_point());
        write_plain(buf, " ");

        let len = extent.len();
        if !len.is_zero() {
            write_duration(buf, len);
            write_plain(buf, " ");
        }
    }

    if let Some(level) = evt.props().lvl() {
        if let Some(level_color) = level_color(&level) {
            write_fg(buf, level, Color::Ansi256(level_color));
            write_plain(buf, " ");
        }
    }

    let _ = evt.msg().write(Writer { buf });
    write_plain(buf, "\n");

    let _ = out.print(&buf);
}

fn print_histogram(
    out: &BufferWriter,
    buf: &mut Buffer,
    metric_name: &str,
    metric_kind: MetricKind,
    histogram: &Histogram,
) {
    const BLOCKS: [&'static str; 7] = ["▁", "▂", "▃", "▄", "▅", "▆", "▇"];

    let x = histogram.timestamp_range();
    let y = histogram.value_range();

    let bucket_size = friendly_duration(histogram.bucket_size());

    print_event(
        out,
        buf,
        &Event::new(
            x,
            emit::tpl!("{metric_kind} of {metric_name} by {bucket_size}{bucket_size_unit} is in the range {#[emit::fmt(\".3\")] min}..={#[emit::fmt(\".3\")] max}"),
            emit::props! {
                metric_kind,
                metric_name,
                min: y.start,
                max: y.end,
                bucket_size: bucket_size.value,
                bucket_size_unit: bucket_size.unit,
            },
        ),
    );
    buf.clear();

    for v in histogram.iter_values() {
        let idx =
            (((v - y.start) / (y.end - y.start)) * ((BLOCKS.len() - 1) as f64)).ceil() as usize;
        let _ = buf.write(BLOCKS[idx].as_bytes());
    }

    let _ = buf.write(b"\n");
    let _ = out.print(buf);
}

struct Writer<'a> {
    buf: &'a mut Buffer,
}

impl<'a> sval_fmt::TokenWrite for Writer<'a> {
    fn write_text_quote(&mut self) -> fmt::Result {
        Ok(())
    }

    fn write_text(&mut self, text: &str) -> fmt::Result {
        self.write(text, TEXT);

        Ok(())
    }

    fn write_number<N: fmt::Display>(&mut self, num: N) -> fmt::Result {
        self.write(num, NUMBER);

        Ok(())
    }

    fn write_atom<A: fmt::Display>(&mut self, atom: A) -> fmt::Result {
        self.write(atom, ATOM);

        Ok(())
    }

    fn write_ident(&mut self, ident: &str) -> fmt::Result {
        self.write(ident, IDENT);

        Ok(())
    }

    fn write_field(&mut self, field: &str) -> fmt::Result {
        self.write(field, FIELD);

        Ok(())
    }
}

impl<'a> fmt::Write for Writer<'a> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        write!(&mut self.buf, "{}", s).map_err(|_| fmt::Error)
    }
}

impl<'a> emit::template::Write for Writer<'a> {
    fn write_hole_value(&mut self, value: emit::Value) -> fmt::Result {
        sval_fmt::stream_to_token_write(self, value)
    }

    fn write_hole_fmt(
        &mut self,
        value: emit::Value,
        formatter: emit::template::Formatter,
    ) -> fmt::Result {
        use sval::Value as _;

        match value.tag() {
            Some(sval::tags::NUMBER) => self.write(formatter.apply(value), NUMBER),
            _ => self.write(formatter.apply(value), TEXT),
        }

        Ok(())
    }
}

const TEXT: Color = Color::Ansi256(69);
const NUMBER: Color = Color::Ansi256(135);
const ATOM: Color = Color::Ansi256(168);
const IDENT: Color = Color::Ansi256(170);
const FIELD: Color = Color::Ansi256(174);

fn write_fg(buf: &mut Buffer, v: impl fmt::Display, color: Color) {
    let _ = buf.set_color(ColorSpec::new().set_fg(Some(color)));
    let _ = write!(buf, "{}", v);
    let _ = buf.reset();
}

fn write_plain(buf: &mut Buffer, v: impl fmt::Display) {
    let _ = write!(buf, "{}", v);
}

impl<'a> Writer<'a> {
    fn write(&mut self, v: impl fmt::Display, color: Color) {
        write_fg(&mut *self.buf, v, color);
    }
}
