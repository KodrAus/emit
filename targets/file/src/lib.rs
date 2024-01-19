use std::{
    fs::{self, File},
    io::Write,
    ops::ControlFlow,
    path::{Path, PathBuf},
    sync::{Arc, Condvar, Mutex},
    thread,
};

use emit::well_known::{MSG_KEY, TPL_KEY, TS_KEY, TS_START_KEY};

pub type Error = Box<dyn std::error::Error + Send + Sync>;

pub fn set(file_set: impl AsRef<Path>) -> Result<FileSetBuilder, Error> {
    let file_set = file_set.as_ref();

    let dir = if let Some(parent) = file_set.parent() {
        parent
            .to_str()
            .ok_or_else(|| "paths must be valid UTF8")?
            .to_owned()
    } else {
        String::new()
    };

    let prefix = file_set
        .file_stem()
        .ok_or_else(|| "paths must include a file name")?
        .to_str()
        .ok_or_else(|| "paths must be valid UTF8")?
        .to_owned();

    let ext = if let Some(ext) = file_set.extension() {
        ext.to_str()
            .ok_or_else(|| "paths must be valid UTF8")?
            .to_owned()
    } else {
        String::from("log")
    };

    Ok(FileSetBuilder::new(dir, prefix, ext))
}

pub struct FileSetBuilder {
    dir: String,
    prefix: String,
    ext: String,
}

impl FileSetBuilder {
    pub fn new(dir: impl Into<String>, prefix: impl Into<String>, ext: impl Into<String>) -> Self {
        FileSetBuilder {
            dir: dir.into(),
            prefix: prefix.into(),
            ext: ext.into(),
        }
    }

    pub fn spawn(self) -> FileSet {
        let (sender, receiver) = emit_batcher::bounded(10_000);

        let handle = thread::spawn(move || {
            let mut active_file = None;

            let _ = receiver.blocking_exec(|batch: Buffer| {
                let mut file = match active_file.take() {
                    Some(file) => file,
                    None => {
                        let id = uuid::Uuid::now_v7();

                        let mut path = PathBuf::from(self.dir.clone());

                        fs::create_dir_all(&path)
                            .map_err(|e| emit_batcher::BatchError::no_retry(e))?;

                        path.push(format!("{}.{}.{}", self.prefix, id.simple(), self.ext));

                        File::create(path).map_err(|e| emit_batcher::BatchError::no_retry(e))?
                    }
                };

                for buf in batch.bufs {
                    file.write_all(buf.as_bytes())
                        .map_err(|e| emit_batcher::BatchError::no_retry(e))?;
                }

                file.flush()
                    .map_err(|e| emit_batcher::BatchError::no_retry(e))?;
                file.sync_all()
                    .map_err(|e| emit_batcher::BatchError::no_retry(e))?;

                active_file = Some(file);

                Ok(())
            });
        });

        FileSet { sender, handle }
    }
}

struct Buffer {
    bufs: Vec<String>,
}

impl emit_batcher::Channel for Buffer {
    type Item = String;

    fn new() -> Self {
        Buffer { bufs: Vec::new() }
    }

    fn push<'a>(&mut self, item: Self::Item) {
        self.bufs.push(item);
    }

    fn len(&self) -> usize {
        self.bufs.len()
    }

    fn clear(&mut self) {
        self.bufs.clear()
    }
}

pub struct FileSet {
    sender: emit_batcher::Sender<Buffer>,
    handle: thread::JoinHandle<()>,
}

impl emit::Emitter for FileSet {
    fn emit<P: emit::Props>(&self, evt: &emit::Event<P>) {
        if let Ok(mut s) = sval_json::stream_to_string(EventValue(evt)) {
            s.push('\n');
            self.sender.send(s);
        }
    }

    fn blocking_flush(&self, timeout: std::time::Duration) {
        let blocker = Arc::new((Mutex::new(false), Condvar::new()));

        self.sender.on_next_flush({
            let blocker = blocker.clone();

            move || {
                *blocker.0.lock().unwrap() = true;
                blocker.1.notify_all();
            }
        });

        let mut flushed = blocker.0.lock().unwrap();
        while !*flushed {
            match blocker.1.wait_timeout(flushed, timeout).unwrap() {
                (next_flushed, r) if !r.timed_out() => {
                    flushed = next_flushed;
                    continue;
                }
                _ => return,
            }
        }
    }
}

struct EventValue<'a, P>(&'a emit::Event<'a, P>);

impl<'a, P: emit::Props> sval::Value for EventValue<'a, P> {
    fn stream<'sval, S: sval::Stream<'sval> + ?Sized>(&'sval self, stream: &mut S) -> sval::Result {
        stream.record_begin(None, None, None, None)?;

        if let Some(extent) = self.0.extent() {
            let range = extent.as_range();

            if range.end != range.start {
                stream.record_value_begin(None, &sval::Label::new(TS_START_KEY))?;
                sval::stream_display(&mut *stream, &range.start)?;
                stream.record_value_end(None, &sval::Label::new(TS_START_KEY))?;
            }

            stream.record_value_begin(None, &sval::Label::new(TS_KEY))?;
            sval::stream_display(&mut *stream, &range.end)?;
            stream.record_value_end(None, &sval::Label::new(TS_KEY))?;
        }

        stream.record_value_begin(None, &sval::Label::new(MSG_KEY))?;
        sval::stream_display(&mut *stream, self.0.msg())?;
        stream.record_value_end(None, &sval::Label::new(MSG_KEY))?;

        stream.record_value_begin(None, &sval::Label::new(TPL_KEY))?;
        sval::stream_display(&mut *stream, self.0.tpl())?;
        stream.record_value_end(None, &sval::Label::new(TPL_KEY))?;

        self.0.props().for_each(|k, v| {
            match (|| {
                stream.record_value_begin(None, &sval::Label::new_computed(k.as_str()))?;
                stream.value_computed(&v)?;
                stream.record_value_end(None, &sval::Label::new_computed(k.as_str()))?;

                Ok::<(), sval::Error>(())
            })() {
                Ok(()) => ControlFlow::Continue(()),
                Err(_) => ControlFlow::Break(()),
            }
        });

        stream.record_end(None, None, None)
    }
}

struct FileWriter {
    receiver: emit_batcher::Receiver<Buffer>,
}
