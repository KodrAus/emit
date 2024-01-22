use std::{
    fs,
    io::{self, Write},
    mem,
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
    roll_by: RollBy,
    max_files: usize,
    reuse_files: bool,
}

#[derive(Debug, Clone, Copy)]
enum RollBy {
    Day,
    Hour,
    Minute,
}

impl FileSetBuilder {
    pub fn new(file_set: impl Into<PathBuf>) -> Self {
        FileSetBuilder {
            file_set: file_set.into(),
            roll_by: RollBy::Hour,
            max_files: 32,
            reuse_files: false,
        }
    }

    pub fn roll_by_day(mut self) -> Self {
        self.roll_by = RollBy::Day;
        self
    }

    pub fn roll_by_hour(mut self) -> Self {
        self.roll_by = RollBy::Hour;
        self
    }

    pub fn roll_by_minute(mut self) -> Self {
        self.roll_by = RollBy::Minute;
        self
    }

    pub fn max_files(mut self, max_files: usize) -> Self {
        self.max_files = max_files;
        self
    }

    pub fn reuse_files(mut self, reuse_files: bool) -> Self {
        self.reuse_files = reuse_files;
        self
    }

    pub fn spawn(self) -> Result<FileSet, Error> {
        let (dir, file_prefix, file_ext) = dir_prefix_ext(self.file_set)?;

        let (sender, receiver) = emit_batcher::bounded(10_000);

        let handle = thread::spawn(move || {
            let mut active_file = None;

            let _ = receiver.blocking_exec(|mut batch: Buffer| {
                let mut file = match active_file.take() {
                    Some(file) => file,
                    None => {
                        let now = std::time::UNIX_EPOCH.elapsed().unwrap();
                        let ts = emit::Timestamp::new(now).unwrap();
                        let parts = ts.to_parts();

                        let mut path = PathBuf::from(dir.clone());

                        if let Err(e) = fs::create_dir_all(&path) {
                            return Err(emit_batcher::BatchError::retry(e, batch));
                        }

                        let file_ts = file_ts(self.roll_by, parts);

                        // Apply retention to the file set and see if there's an existing file that can be reused
                        // Files are only reused if the writer is configured to do so
                        let reuse_file_name =
                            match apply_retention(&path, &file_prefix, &file_ext, self.max_files) {
                                // If there's an existing file and:
                                // 1. We're configured to reuse files
                                // 2. The timestamp part of the file matches the current window
                                // we'll attempt to open and reuse this file
                                Ok(Some(last_file)) => {
                                    if self.reuse_files
                                        && read_file_ts(&last_file) == Some(&*file_ts)
                                    {
                                        Some(last_file)
                                    } else {
                                        None
                                    }
                                }
                                // In any other case, a new file will be created
                                Ok(None) => None,
                                Err(e) => return Err(emit_batcher::BatchError::retry(e, batch)),
                            };

                        let reuse_file = if let Some(file_name) = reuse_file_name {
                            let mut path = path.clone();
                            path.push(file_name);

                            try_open_reuse(&path).ok()
                        } else {
                            None
                        };

                        if let Some(file) = reuse_file {
                            file
                        }
                        // If there's no file to reuse then create a new one
                        else {
                            let file_id =
                                file_id(rolling_millis(self.roll_by, ts, parts), rolling_id());

                            path.push(file_name(&file_prefix, &file_ext, &file_ts, &file_id));

                            match try_open_create(path) {
                                Ok(file) => file,
                                Err(e) => return Err(emit_batcher::BatchError::retry(e, batch)),
                            }
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

        Ok(FileSet {
            sender,
            _handle: handle,
        })
    }
}

fn try_open_reuse(path: impl AsRef<Path>) -> Result<fs::File, io::Error> {
    let mut file = fs::OpenOptions::new().read(false).append(true).open(path)?;

    // Defensive newline to ensure any incomplete event is terminated
    // before any new ones are written
    // We could avoid filling files with newlines by attempting to read
    // one from the end first
    file.write_all(b"\n")?;

    Ok(file)
}

fn try_open_create(path: impl AsRef<Path>) -> Result<fs::File, io::Error> {
    fs::OpenOptions::new()
        .create_new(true)
        .read(false)
        .append(true)
        .open(path)
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

fn rolling_millis(roll_by: RollBy, ts: emit::Timestamp, parts: emit::timestamp::Parts) -> u32 {
    let truncated = match roll_by {
        RollBy::Day => emit::Timestamp::from_parts(emit::timestamp::Parts {
            years: parts.years,
            months: parts.months,
            days: parts.days,
            ..Default::default()
        })
        .unwrap(),
        RollBy::Hour => emit::Timestamp::from_parts(emit::timestamp::Parts {
            years: parts.years,
            months: parts.months,
            days: parts.days,
            hours: parts.hours,
            ..Default::default()
        })
        .unwrap(),
        RollBy::Minute => emit::Timestamp::from_parts(emit::timestamp::Parts {
            years: parts.years,
            months: parts.months,
            days: parts.days,
            hours: parts.hours,
            minutes: parts.minutes,
            ..Default::default()
        })
        .unwrap(),
    };

    ts.duration_since(truncated).unwrap().as_millis() as u32
}

fn rolling_id() -> u32 {
    rand::random()
}

fn file_ts(roll_by: RollBy, parts: emit::timestamp::Parts) -> String {
    match roll_by {
        RollBy::Day => format!(
            "{:>04}-{:>02}-{:>02}",
            parts.years, parts.months, parts.days,
        ),
        RollBy::Hour => format!(
            "{:>04}-{:>02}-{:>02}-{:>02}",
            parts.years, parts.months, parts.days, parts.hours,
        ),
        RollBy::Minute => format!(
            "{:>04}-{:>02}-{:>02}-{:>02}-{:>02}",
            parts.years, parts.months, parts.days, parts.hours, parts.minutes,
        ),
    }
}

fn file_id(rolling_millis: u32, rolling_id: u32) -> String {
    format!("{:<08}.{:<08x}", rolling_millis, rolling_id)
}

fn read_file_ts(file_name: &str) -> Option<&str> {
    file_name.split('.').skip(1).next()
}

fn file_name(file_prefix: &str, file_ext: &str, ts: &str, id: &str) -> String {
    format!("{}.{}.{}.{}", file_prefix, ts, id, file_ext)
}

fn apply_retention(
    path: impl Into<PathBuf>,
    prefix: &str,
    ext: &str,
    max_files: usize,
) -> Result<Option<String>, io::Error> {
    let path = path.into();

    let read_dir = fs::read_dir(&path)?;

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

    while file_set.len() >= max_files {
        let mut path = path.clone();
        path.push(file_set.pop().unwrap());

        let _ = fs::remove_file(path);
    }

    Ok(file_set.first_mut().map(mem::take))
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
    _handle: thread::JoinHandle<()>,
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
