use std::{
    fs,
    io::Write,
    ops::ControlFlow,
    path::{Path, PathBuf},
    sync::{Arc, Condvar, Mutex},
    thread,
};

use emit::well_known::{MSG_KEY, TPL_KEY, TS_KEY, TS_START_KEY};

pub type Error = Box<dyn std::error::Error + Send + Sync>;

pub fn set(file_set: impl AsRef<Path>) -> FileSetBuilder {
    FileSetBuilder::new(file_set.as_ref())
}

pub struct FileSetBuilder {
    file_set: PathBuf,
}

impl FileSetBuilder {
    pub fn new(file_set: impl Into<PathBuf>) -> Self {
        FileSetBuilder {
            file_set: file_set.into(),
        }
    }

    pub fn spawn(self) -> Result<FileSet, Error> {
        let (dir, prefix, ext) = dir_prefix_ext(self.file_set)?;

        let (sender, receiver) = emit_batcher::bounded(10_000);

        let handle = thread::spawn(move || {
            let mut active_file = None;

            let _ = receiver.blocking_exec(|mut batch: Buffer| {
                let now = std::time::UNIX_EPOCH.elapsed().unwrap();
                let (secs, nanos) = (now.as_secs(), now.subsec_nanos());
                let ts = emit::Timestamp::new(now).unwrap().to_parts();

                let mut file = match active_file.take() {
                    Some(file) => file,
                    None => {
                        let id = uuid::Uuid::new_v7(uuid::Timestamp::from_unix(
                            uuid::NoContext,
                            secs,
                            nanos,
                        ));

                        let mut path = PathBuf::from(dir.clone());

                        if let Err(e) = fs::create_dir_all(&path) {
                            return Err(emit_batcher::BatchError::retry(e, batch));
                        }

                        // TODO: Should probably cache this
                        let read_dir = match fs::read_dir(&path) {
                            Ok(read_dir) => read_dir,
                            Err(e) => return Err(emit_batcher::BatchError::retry(e, batch)),
                        };

                        let mut file_set = Vec::new();
                        for entry in read_dir {
                            let Ok(entry) = entry else {
                                continue;
                            };

                            if let Ok(file_type) = entry.file_type() {
                                if !file_type.is_file() {
                                    continue;
                                }
                            }

                            let file_name = entry.file_name();
                            let Some(file_name) = file_name.to_str() else {
                                continue;
                            };

                            if file_name.starts_with(&prefix) && file_name.ends_with(&ext) {
                                file_set.push(file_name.to_owned());
                            }
                        }
                        file_set.sort_by(|a, b| a.cmp(b).reverse());

                        while file_set.len() >= 4 {
                            let mut path = path.clone();
                            path.push(file_set.pop().unwrap());

                            let _ = fs::remove_file(path);
                        }

                        path.push(format!(
                            "{}.{:>04}-{:>02}-{:>02}.{}.{}",
                            prefix,
                            ts.years,
                            ts.months,
                            ts.days,
                            id.simple(),
                            ext
                        ));

                        match fs::OpenOptions::new()
                            .create_new(true)
                            .read(false)
                            .append(true)
                            .open(path)
                        {
                            Ok(file) => file,
                            Err(e) => return Err(emit_batcher::BatchError::retry(e, batch)),
                        }
                    }
                };

                while batch.index < batch.bufs.len() {
                    if let Err(e) = file.write_all(batch.bufs[batch.index].as_bytes()) {
                        return Err(emit_batcher::BatchError::retry(e, batch));
                    }

                    // Drop the buffer at this point to free some memory
                    batch.bufs[batch.index] = String::new();
                    batch.index += 1;
                }

                file.flush()
                    .map_err(|e| emit_batcher::BatchError::no_retry(e))?;
                file.sync_all()
                    .map_err(|e| emit_batcher::BatchError::no_retry(e))?;

                active_file = Some(file);

                Ok(())
            });
        });

        Ok(FileSet { sender, handle })
    }
}

fn dir_prefix_ext(file_set: impl AsRef<Path>) -> Result<(String, String, String), Error> {
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

    Ok((dir, prefix, ext))
}

struct Buffer {
    bufs: Vec<String>,
    index: usize,
}

impl emit_batcher::Channel for Buffer {
    type Item = String;

    fn new() -> Self {
        Buffer {
            bufs: Vec::new(),
            index: 0,
        }
    }

    fn push<'a>(&mut self, item: Self::Item) {
        self.bufs.push(item);
    }

    fn remaining(&self) -> usize {
        self.bufs.len() - self.index
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
            s.shrink_to_fit();
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
