use core::{fmt, time::Duration};
use std::{cell::RefCell, io::Write};

use termcolor::{Buffer, BufferWriter, Color, ColorChoice, ColorSpec, WriteColor};

pub fn stdout() -> Stdout {
    Stdout(BufferWriter::stdout(ColorChoice::Auto))
}

pub struct Stdout(BufferWriter);

thread_local! {
    static STDOUT: RefCell<Option<Buffer>> = RefCell::new(None);
}

impl emit::target::Target for Stdout {
    fn emit_event<P: emit::Props>(&self, evt: &emit::Event<P>) {
        STDOUT.with(|buf| {
            match buf.try_borrow_mut() {
                // If there are no overlapping references then use the cached buffer
                Ok(mut slot) => {
                    match &mut *slot {
                        // If there's a cached buffer then clear it and print using it
                        Some(buf) => {
                            buf.clear();
                            print(&self.0, buf, evt);
                        }
                        // If there's no cached buffer then create one and use it
                        // It'll be cached for future callers on this thread
                        None => {
                            let mut buf = self.0.buffer();
                            print(&self.0, &mut buf, evt);

                            *slot = Some(buf);
                        }
                    }
                }
                // If there are overlapping references then just create a
                // buffer on-demand to use
                Err(_) => {
                    print(&self.0, &mut self.0.buffer(), evt);
                }
            }
        });
    }

    fn blocking_flush(&self, _: Duration) {
        // Events are emitted synchronously
    }
}

fn print(out: &BufferWriter, buf: &mut Buffer, evt: &emit::Event<impl emit::Props>) {
    let props = evt.props();

    if let Ok(_) = evt.tpl().with_props(props).write(Writer { buf }) {
        let _ = buf.write(b"\n");
        let _ = out.print(&buf);
    }
}

struct Writer<'a> {
    buf: &'a mut Buffer,
}

impl<'a> sval_fmt::TokenWrite for Writer<'a> {
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
}

const TEXT: Color = Color::Ansi256(69);
const NUMBER: Color = Color::Ansi256(135);
const ATOM: Color = Color::Ansi256(168);
const IDENT: Color = Color::Ansi256(170);
const FIELD: Color = Color::Ansi256(174);

impl<'a> Writer<'a> {
    fn write(&mut self, v: impl fmt::Display, color: Color) {
        let _ = self.buf.set_color(ColorSpec::new().set_fg(Some(color)));
        let _ = write!(&mut self.buf, "{}", v);
        let _ = self.buf.reset();
    }
}
