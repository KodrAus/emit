#![feature(proc_macro_hygiene, stmt_expr_attributes)]

use core::{fmt, str, time::Duration};
use std::{cell::RefCell, cmp, io::Write};

use emit::well_known::{KEY_LVL, KEY_METRIC_NAME, KEY_METRIC_VALUE, KEY_SPAN_ID, KEY_TRACE_ID};
use termcolor::{Buffer, BufferWriter, Color, ColorChoice, ColorSpec, WriteColor};

pub fn stdout() -> Stdout {
    Stdout {
        writer: BufferWriter::stdout(ColorChoice::Auto),
    }
}

pub struct Stdout {
    writer: BufferWriter,
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
    fn emit<P: emit::props::Props>(&self, evt: &emit::event::Event<P>) {
        with_shared_buf(&self.writer, |writer, buf| print_event(writer, buf, evt));
    }

    fn blocking_flush(&self, _: Duration) {}
}

impl emit::runtime::InternalEmitter for Stdout {}

fn trace_id_color(trace_id: &emit::trace::TraceId) -> u8 {
    let mut hash = 0;

    for b in trace_id.to_u128().to_le_bytes() {
        hash ^= b;
    }

    hash
}

fn span_id_color(span_id: &emit::trace::SpanId) -> u8 {
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

fn hex_slice<'a>(hex: &'a [u8], len: usize) -> impl fmt::Display + 'a {
    struct HexSlice<'a>(&'a [u8], usize);

    impl<'a> fmt::Display for HexSlice<'a> {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str(str::from_utf8(&self.0[..self.1]).unwrap())
        }
    }

    HexSlice(hex, len)
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

fn print_event(
    out: &BufferWriter,
    buf: &mut Buffer,
    evt: &emit::event::Event<impl emit::props::Props>,
) {
    if let Some(span_id) = evt.props().pull::<emit::trace::SpanId, _>(KEY_SPAN_ID) {
        if let Some(trace_id) = evt.props().pull::<emit::trace::TraceId, _>(KEY_TRACE_ID) {
            let trace_id_color = trace_id_color(&trace_id);

            write_fg(buf, "▓", Color::Ansi256(trace_id_color));
            write_plain(buf, " ");
            write_plain(buf, hex_slice(&trace_id.to_hex(), 6));
            write_plain(buf, " ");
        } else {
            write_plain(buf, "░      ");
        }

        let span_id_color = span_id_color(&span_id);

        write_fg(buf, "▓", Color::Ansi256(span_id_color));
        write_plain(buf, " ");
        write_plain(buf, hex_slice(&span_id.to_hex(), 4));
        write_plain(buf, " ");
    }

    if let Some(extent) = evt.extent() {
        if extent.is_span() {
            if let Some(len) = extent.len() {
                write_timestamp(buf, *extent.as_point());
                write_plain(buf, " ");
                write_duration(buf, len);
            } else {
                write_timestamp(buf, extent.as_range().start);
                write_plain(buf, "..");
                write_timestamp(buf, extent.as_range().end);
            }
        } else {
            write_timestamp(buf, *extent.as_point());
        }

        write_plain(buf, " ");
    }

    if evt.props().get(KEY_METRIC_NAME).is_some() {
        write_fg(buf, "metric", METRIC);
        write_plain(buf, " ");
    }

    if let Some(level) = evt.props().pull::<emit::Level, _>(KEY_LVL) {
        if let Some(level_color) = level_color(&level) {
            write_fg(buf, level, Color::Ansi256(level_color));
            write_plain(buf, " ");
        }
    }

    write_fg(buf, format_args!("{} ", evt.module()), MODULE);

    let _ = evt.msg().write(Writer { buf });
    write_plain(buf, "\n");

    if let Some(value) = evt.props().get(KEY_METRIC_VALUE) {
        let buckets = value.as_f64_sequence();

        if !buckets.is_empty() {
            write_histogram(buf, &buckets);
        }
    }

    let _ = out.print(&buf);
}

fn write_histogram(buf: &mut Buffer, buckets: &[f64]) {
    const BLOCKS: [&'static str; 7] = ["▁", "▂", "▃", "▄", "▅", "▆", "▇"];

    let mut bucket_min = f64::NAN;
    let mut bucket_max = -f64::NAN;

    for v in buckets {
        bucket_min = cmp::min_by(*v, bucket_min, f64::total_cmp);
        bucket_max = cmp::max_by(*v, bucket_max, f64::total_cmp);
    }

    for v in buckets {
        let idx = (((v - bucket_min) / (bucket_max - bucket_min)) * ((BLOCKS.len() - 1) as f64))
            .ceil() as usize;
        let _ = buf.write(BLOCKS[idx].as_bytes());
    }

    let _ = buf.write(b"\n");
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
    fn write_hole_value(&mut self, _: &str, value: emit::Value) -> fmt::Result {
        sval_fmt::stream_to_token_write(self, value)
    }

    fn write_hole_fmt(
        &mut self,
        _: &str,
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

const MODULE: Color = Color::Ansi256(244);
const METRIC: Color = Color::Ansi256(174);

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
