use core::{fmt, time::Duration};
use std::io::Write;

use termcolor::{Buffer, BufferWriter, Color, ColorChoice, ColorSpec, WriteColor};

pub fn stdout() -> Stdout {
    Stdout
}

pub struct Stdout;

impl emit::target::Target for Stdout {
    fn emit_event<P: emit::Props>(&self, evt: &emit::Event<P>) {
        let stdout = BufferWriter::stdout(ColorChoice::Auto);
        let mut buf = stdout.buffer();

        let props = evt.props();

        if let Ok(_) = evt.tpl().with_props(props).write(Writer { buf: &mut buf }) {
            let _ = buf.write(b"\n");
            let _ = stdout.print(&buf);
        }
    }

    fn blocking_flush(&self, _: Duration) {}
}

struct Writer<'a> {
    buf: &'a mut Buffer,
}

impl<'a> sval_fmt::TokenWrite for Writer<'a> {
    fn write_text(&mut self, text: &str) -> fmt::Result {
        self.write(text, TEXT)
    }

    fn write_number<N: fmt::Display>(&mut self, num: N) -> fmt::Result {
        self.write(num, NUMBER)
    }

    fn write_atom<A: fmt::Display>(&mut self, atom: A) -> fmt::Result {
        self.write(atom, ATOM)
    }

    fn write_ident(&mut self, ident: &str) -> fmt::Result {
        self.write(ident, IDENT)
    }

    fn write_field(&mut self, field: &str) -> fmt::Result {
        self.write(field, FIELD)
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
}

const TEXT: Color = Color::Ansi256(69);
const NUMBER: Color = Color::Ansi256(135);
const ATOM: Color = Color::Ansi256(168);
const IDENT: Color = Color::Ansi256(170);
const FIELD: Color = Color::Ansi256(174);

impl<'a> Writer<'a> {
    fn write(&mut self, v: impl fmt::Display, color: Color) -> fmt::Result {
        self.buf
            .set_color(ColorSpec::new().set_fg(Some(color)))
            .map_err(|_| fmt::Error)?;
        write!(&mut self.buf, "{}", v).map_err(|_| fmt::Error)?;
        self.buf.reset().map_err(|_| fmt::Error)?;

        Ok(())
    }
}
