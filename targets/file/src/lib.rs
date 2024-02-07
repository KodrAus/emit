#![feature(stmt_expr_attributes, proc_macro_hygiene)]

use std::{
    fs::{self, File},
    io::{self, Write},
    mem,
    ops::ControlFlow,
    path::{Path, PathBuf},
    thread,
};

use emit::well_known::{MSG_KEY, TPL_KEY, TS_KEY, TS_START_KEY};
use emit_batcher::BatchError;

pub type Error = Box<dyn std::error::Error + Send + Sync>;

pub fn set(file_set: impl AsRef<Path>) -> FileSetBuilder {
    FileSetBuilder::new(file_set.as_ref())
}

pub struct FileSetBuilder {
    file_set: PathBuf,
    roll_by: RollBy,
    max_files: usize,
    max_file_size_bytes: usize,
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
            max_file_size_bytes: 1024 * 1024 * 1024, // 1GiB
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

    pub fn max_file_size_bytes(mut self, max_file_size_bytes: usize) -> Self {
        self.max_file_size_bytes = max_file_size_bytes;
        self
    }

    pub fn reuse_files(mut self, reuse_files: bool) -> Self {
        self.reuse_files = reuse_files;
        self
    }

    pub fn spawn(self) -> Result<FileSet, Error> {
        let (dir, file_prefix, file_ext) = dir_prefix_ext(self.file_set)?;

        let mut worker = Worker::new(
            dir,
            file_prefix,
            file_ext,
            self.roll_by,
            self.reuse_files,
            self.max_files,
            self.max_file_size_bytes,
        );

        let (sender, receiver) = emit_batcher::bounded(10_000);

        let handle = thread::spawn(move || {
            let _ = receiver.blocking_exec(|batch| worker.on_batch(batch));
        });

        Ok(FileSet {
            sender,
            _handle: handle,
        })
    }
}

pub struct FileSet {
    sender: emit_batcher::Sender<EventBatch>,
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
        emit_batcher::sync::blocking_flush(&self.sender, timeout)
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

struct EventBatch {
    bufs: Vec<String>,
    remaining_bytes: usize,
    index: usize,
}

impl emit_batcher::Channel for EventBatch {
    type Item = String;

    fn new() -> Self {
        EventBatch {
            bufs: Vec::new(),
            remaining_bytes: 0,
            index: 0,
        }
    }

    fn push<'a>(&mut self, item: Self::Item) {
        self.remaining_bytes += item.len();
        self.bufs.push(item);
    }

    fn remaining(&self) -> usize {
        self.bufs.len() - self.index
    }

    fn clear(&mut self) {
        self.bufs.clear()
    }
}

impl EventBatch {
    fn current(&self) -> Option<&str> {
        self.bufs.get(self.index).map(|buf| &**buf)
    }

    fn advance(&mut self) {
        let advanced = mem::take(&mut self.bufs[self.index]);

        self.index += 1;
        self.remaining_bytes -= advanced.len();
    }
}

struct Worker {
    active_file: Option<ActiveFile>,
    roll_by: RollBy,
    max_files: usize,
    max_file_size_bytes: usize,
    reuse_files: bool,
    dir: String,
    file_prefix: String,
    file_ext: String,
}

impl Worker {
    fn new(
        dir: String,
        file_prefix: String,
        file_ext: String,
        roll_by: RollBy,
        reuse_files: bool,
        max_files: usize,
        max_file_size_bytes: usize,
    ) -> Self {
        Worker {
            active_file: None,
            roll_by,
            max_files,
            max_file_size_bytes,
            reuse_files,
            dir,
            file_prefix,
            file_ext,
        }
    }

    fn on_batch(&mut self, mut batch: EventBatch) -> Result<(), BatchError<EventBatch>> {
        use emit::{Emit as _, StartTimer as _};

        let rt = emit::runtime::internal();

        let timer = rt.start_timer();

        let now = std::time::UNIX_EPOCH.elapsed().unwrap();
        let ts = emit::Timestamp::new(now).unwrap();
        let parts = ts.to_parts();

        let file_ts = file_ts(self.roll_by, parts);

        let mut file = self.active_file.take();
        let mut file_set = ActiveFileSet::empty(&self.dir);

        if file.is_none() {
            if let Err(err) = fs::create_dir_all(&self.dir) {
                rt.warn(
                    emit::tpl!("failed to create root directory {path}: {err}"),
                    emit::props! {
                        #[emit::as_debug] path: &self.dir,
                        err,
                    },
                );

                return Err(emit_batcher::BatchError::retry(err, batch));
            }

            let _ = file_set
                .read(&self.file_prefix, &self.file_ext)
                .map_err(|err| {
                    rt.warn(
                        emit::tpl!("failed to files in read {path}: {err}"),
                        emit::props! {
                            #[emit::as_debug] path: &file_set.dir,
                            err,
                        },
                    );

                    err
                });

            if self.reuse_files {
                if let Some(file_name) = file_set.current_file_name() {
                    let mut path = PathBuf::from(&self.dir);
                    path.push(file_name);

                    file = ActiveFile::try_open_reuse(&path)
                        .map_err(|err| {
                            rt.warn(
                                emit::tpl!("failed to open {path}: {err}"),
                                emit::props! {
                                    #[emit::as_debug] path,
                                    err,
                                },
                            );

                            err
                        })
                        .ok()
                }
            }
        }

        file = file.filter(|file| {
            file.file_size_bytes + batch.remaining_bytes <= self.max_file_size_bytes
                && file.file_ts == file_ts
        });

        let mut file = if let Some(file) = file {
            file
        } else {
            // Leave room for the file we're about to create
            file_set.apply_retention(self.max_files.saturating_sub(1));

            let mut path = PathBuf::from(self.dir.clone());

            let file_id = file_id(rolling_millis(self.roll_by, ts, parts), rolling_id());

            path.push(file_name(
                &self.file_prefix,
                &self.file_ext,
                &file_ts,
                &file_id,
            ));

            match ActiveFile::try_open_create(&path) {
                Ok(file) => {
                    rt.debug(
                        emit::tpl!("created {path}"),
                        emit::props! {
                            #[emit::as_debug] path: file.file_path,
                        },
                    );

                    file
                }
                Err(err) => {
                    rt.warn(
                        emit::tpl!("failed to create {path}: {err}"),
                        emit::props! {
                            #[emit::as_debug] path,
                            err,
                        },
                    );

                    return Err(emit_batcher::BatchError::retry(err, batch));
                }
            }
        };

        let written_bytes = batch.remaining_bytes;

        while let Some(buf) = batch.current() {
            if let Err(err) = file.write_event(buf.as_bytes()) {
                rt.warn(
                    emit::tpl!("failed to write event to {path}: {err}"),
                    emit::props! {
                        #[emit::as_debug] path: file.file_path,
                        err,
                    },
                );

                return Err(emit_batcher::BatchError::retry(err, batch));
            }

            batch.advance();
        }

        file.file
            .flush()
            .map_err(|e| emit_batcher::BatchError::no_retry(e))?;
        file.file
            .sync_all()
            .map_err(|e| emit_batcher::BatchError::no_retry(e))?;

        rt.emit(&emit::debug_event!(
            extent: timer,
            "wrote {written_bytes} bytes to {path}",
            written_bytes,
            #[emit::as_debug]
            path: file.file_path,
        ));

        // Set the active file so the next batch can attempt to use it
        // At this point the file is expected to be valid
        self.active_file = Some(file);

        Ok(())
    }
}

struct ActiveFileSet<'a> {
    dir: &'a str,
    file_set: Vec<String>,
}

impl<'a> ActiveFileSet<'a> {
    fn empty(dir: &'a str) -> Self {
        ActiveFileSet {
            dir,
            file_set: Vec::new(),
        }
    }

    fn read(&mut self, file_prefix: &str, file_ext: &str) -> Result<(), io::Error> {
        self.file_set = Vec::new();

        let read_dir = fs::read_dir(&self.dir)?;

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

            if file_name.starts_with(&file_prefix) && file_name.ends_with(&file_ext) {
                file_set.push(file_name.to_owned());
            }
        }

        file_set.sort_by(|a, b| a.cmp(b).reverse());

        self.file_set = file_set;

        Ok(())
    }

    fn current_file_name(&self) -> Option<&str> {
        // NOTE: If the clock shifts back (either jitters or through daylight savings)
        // Then we may return a file from the future here instead of one that better
        // matches the current timestamp. In these cases we'll end up creating a new file
        // instead of potentially reusing one that does match.
        self.file_set.first().map(|file_name| &**file_name)
    }

    fn apply_retention(&mut self, max_files: usize) {
        while self.file_set.len() >= max_files {
            let mut path = PathBuf::from(self.dir);
            path.push(self.file_set.pop().unwrap());

            let _ = fs::remove_file(path);
        }
    }
}

struct ActiveFile {
    file: File,
    file_path: PathBuf,
    file_ts: String,
    file_needs_recovery: bool,
    file_size_bytes: usize,
}

impl ActiveFile {
    fn try_open_reuse(file_path: impl AsRef<Path>) -> Result<ActiveFile, io::Error> {
        let file_path = file_path.as_ref();

        let file_ts = read_file_path_ts(file_path)?.to_owned();

        let file = fs::OpenOptions::new()
            .read(false)
            .append(true)
            .open(file_path)?;

        let file_size_bytes = file.metadata()?.len() as usize;

        Ok(ActiveFile {
            file,
            file_ts,
            file_path: file_path.into(),
            // The file is in an unknown state, so defensively assume
            // it needs to be recovered
            file_needs_recovery: true,
            file_size_bytes,
        })
    }

    fn try_open_create(file_path: impl AsRef<Path>) -> Result<ActiveFile, io::Error> {
        let file_path = file_path.as_ref();

        let file_ts = read_file_path_ts(file_path)?.to_owned();

        let file = fs::OpenOptions::new()
            .create_new(true)
            .read(false)
            .append(true)
            .open(file_path)?;

        Ok(ActiveFile {
            file,
            file_ts,
            file_path: file_path.into(),
            file_needs_recovery: false,
            file_size_bytes: 0,
        })
    }

    fn write_event(&mut self, event_buf: &[u8]) -> Result<(), io::Error> {
        // If the file may be correupted then terminate
        // any previously written content with a separator.
        // This ensures the event that's about to be written
        // isn't mangled together with an incomplete one written
        // previously
        if self.file_needs_recovery {
            self.file_size_bytes += 1;
            self.file.write_all(b"\n")?;
        }

        self.file_needs_recovery = true;

        self.file_size_bytes += event_buf.len();
        self.file.write_all(event_buf)?;

        self.file_needs_recovery = false;
        Ok(())
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

fn read_file_name_ts(file_name: &str) -> Result<&str, io::Error> {
    file_name.split('.').skip(1).next().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::Other,
            "could not determine timestamp from filename",
        )
    })
}

fn read_file_path_ts(path: &Path) -> Result<&str, io::Error> {
    let file_name = path
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "unable to determine filename"))?
        .to_str()
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "file names must be valid UTF8"))?;

    read_file_name_ts(file_name)
}

fn file_name(file_prefix: &str, file_ext: &str, ts: &str, id: &str) -> String {
    format!("{}.{}.{}.{}", file_prefix, ts, id, file_ext)
}
