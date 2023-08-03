use core::{cmp, fmt, ops::Range, str, str::FromStr, time::Duration};

use crate::{
    empty::Empty,
    value::{ToValue, Value},
};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Timestamp(Duration);

impl Timestamp {
    pub fn new(elapsed_since_unix_epoch: Duration) -> Self {
        Timestamp(elapsed_since_unix_epoch)
    }

    pub fn to_unix(&self) -> Duration {
        self.0
    }

    #[cfg(feature = "std")]
    pub fn to_system_time(&self) -> std::time::SystemTime {
        std::time::SystemTime::UNIX_EPOCH + self.0
    }
}

impl fmt::Debug for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt_rfc3339(*self, f)
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_rfc3339(*self, f)
    }
}

impl FromStr for Timestamp {
    type Err = ParseTimestampError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_rfc3339(s)
    }
}

impl ToValue for Timestamp {
    fn to_value(&self) -> Value {
        Value::capture_display(self)
    }
}

impl<'v> Value<'v> {
    pub fn to_timestamp(&self) -> Option<Timestamp> {
        self.downcast_ref::<Timestamp>()
            .copied()
            .or_else(|| self.parse())
    }
}

#[derive(Clone)]
pub struct Extent(ExtentInner);

#[derive(Clone)]
enum ExtentInner {
    Point(Range<Timestamp>),
    Span(Range<Timestamp>),
}

impl fmt::Debug for Extent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            ExtentInner::Point(ref ts) => fmt::Debug::fmt(&ts.start, f),
            ExtentInner::Span(ref ts) => fmt::Debug::fmt(ts, f),
        }
    }
}

impl fmt::Display for Extent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            ExtentInner::Point(ref ts) => fmt::Display::fmt(&ts.end, f),
            ExtentInner::Span(ref ts) => {
                fmt::Display::fmt(&ts.start, f)?;
                write!(f, "..")?;
                fmt::Display::fmt(&ts.end, f)
            }
        }
    }
}

impl Extent {
    pub fn point(ts: Timestamp) -> Extent {
        Extent(ExtentInner::Point(ts..ts))
    }

    pub fn span(ts: Range<Timestamp>) -> Extent {
        if ts.start == ts.end {
            Extent::point(ts.end)
        } else {
            Extent(ExtentInner::Span(ts))
        }
    }

    pub fn as_span(&self) -> &Range<Timestamp> {
        match self.0 {
            ExtentInner::Point(ref ts) => ts,
            ExtentInner::Span(ref ts) => ts,
        }
    }

    pub fn to_point(&self) -> Option<&Timestamp> {
        match self.0 {
            ExtentInner::Point(ref ts) => Some(&ts.start),
            ExtentInner::Span(_) => None,
        }
    }
}

impl From<Timestamp> for Extent {
    fn from(point: Timestamp) -> Extent {
        Extent::point(point)
    }
}

impl<'a> From<&'a Timestamp> for Extent {
    fn from(point: &'a Timestamp) -> Extent {
        Extent::point(*point)
    }
}

impl From<Range<Timestamp>> for Extent {
    fn from(span: Range<Timestamp>) -> Extent {
        Extent::span(span)
    }
}

impl<'a> From<&'a Range<Timestamp>> for Extent {
    fn from(span: &'a Range<Timestamp>) -> Extent {
        Extent::span(span.clone())
    }
}

impl<'a> From<&'a Extent> for Extent {
    fn from(extent: &'a Extent) -> Extent {
        extent.clone()
    }
}

pub trait Clock {
    fn now(&self) -> Option<Timestamp>;
}

impl<'a, T: Clock + ?Sized> Clock for &'a T {
    fn now(&self) -> Option<Timestamp> {
        (**self).now()
    }
}

impl<'a, T: Clock> Clock for Option<T> {
    fn now(&self) -> Option<Timestamp> {
        if let Some(time) = self {
            time.now()
        } else {
            Empty.now()
        }
    }
}

#[cfg(feature = "alloc")]
impl<'a, T: Clock + ?Sized + 'a> Clock for alloc::boxed::Box<T> {
    fn now(&self) -> Option<Timestamp> {
        (**self).now()
    }
}

impl Clock for Empty {
    fn now(&self) -> Option<Timestamp> {
        None
    }
}

mod internal {
    use super::Timestamp;

    pub trait DispatchClock {
        fn dispatch_now(&self) -> Option<Timestamp>;
    }

    pub trait SealedTime {
        fn erase_clock(&self) -> crate::internal::Erased<&dyn DispatchClock>;
    }
}

pub trait ErasedClock: internal::SealedTime {}

impl<T: Clock> ErasedClock for T {}

impl<T: Clock> internal::SealedTime for T {
    fn erase_clock(&self) -> crate::internal::Erased<&dyn internal::DispatchClock> {
        crate::internal::Erased(self)
    }
}

impl<T: Clock> internal::DispatchClock for T {
    fn dispatch_now(&self) -> Option<Timestamp> {
        self.now()
    }
}

impl<'a> Clock for dyn ErasedClock + 'a {
    fn now(&self) -> Option<Timestamp> {
        self.erase_clock().0.dispatch_now()
    }
}

impl<'a> Clock for dyn ErasedClock + Send + Sync + 'a {
    fn now(&self) -> Option<Timestamp> {
        (self as &(dyn ErasedClock + 'a)).now()
    }
}

fn parse_rfc3339(fmt: &str) -> Result<Timestamp, ParseTimestampError> {
    todo!()
}

pub struct ParseTimestampError {}

fn fmt_rfc3339(ts: Timestamp, f: &mut fmt::Formatter) -> fmt::Result {
    /*
    Original implementation: https://github.com/tailhook/humantime

    Copyright (c) 2016 The humantime Developers

    Includes parts of http date with the following copyright:
    Copyright (c) 2016 Pyfisch

    Includes portions of musl libc with the following copyright:
    Copyright © 2005-2013 Rich Felker


    Permission is hereby granted, free of charge, to any person obtaining a copy
    of this software and associated documentation files (the "Software"), to deal
    in the Software without restriction, including without limitation the rights
    to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
    copies of the Software, and to permit persons to whom the Software is
    furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all
    copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
    IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
    FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
    AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
    LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
    OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
    SOFTWARE.
    */

    let dur = ts.0;
    let secs_since_epoch = dur.as_secs();
    let nanos = dur.subsec_nanos();

    if secs_since_epoch >= 253_402_300_800 {
        // year 9999
        return Err(fmt::Error);
    }

    /* 2000-03-01 (mod 400 year, immediately after feb29 */
    const LEAPOCH: i64 = 11017;
    const DAYS_PER_400Y: i64 = 365 * 400 + 97;
    const DAYS_PER_100Y: i64 = 365 * 100 + 24;
    const DAYS_PER_4Y: i64 = 365 * 4 + 1;

    let days = (secs_since_epoch / 86400) as i64 - LEAPOCH;
    let secs_of_day = secs_since_epoch % 86400;

    let mut qc_cycles = days / DAYS_PER_400Y;
    let mut remdays = days % DAYS_PER_400Y;

    if remdays < 0 {
        remdays += DAYS_PER_400Y;
        qc_cycles -= 1;
    }

    let mut c_cycles = remdays / DAYS_PER_100Y;
    if c_cycles == 4 {
        c_cycles -= 1;
    }
    remdays -= c_cycles * DAYS_PER_100Y;

    let mut q_cycles = remdays / DAYS_PER_4Y;
    if q_cycles == 25 {
        q_cycles -= 1;
    }
    remdays -= q_cycles * DAYS_PER_4Y;

    let mut remyears = remdays / 365;
    if remyears == 4 {
        remyears -= 1;
    }
    remdays -= remyears * 365;

    let mut year = 2000 + remyears + 4 * q_cycles + 100 * c_cycles + 400 * qc_cycles;

    let months = [31, 30, 31, 30, 31, 31, 30, 31, 30, 31, 31, 29];
    let mut mon = 0;
    for mon_len in months.iter() {
        mon += 1;
        if remdays < *mon_len {
            break;
        }
        remdays -= *mon_len;
    }
    let mday = remdays + 1;
    let mon = if mon + 2 > 12 {
        year += 1;
        mon - 10
    } else {
        mon + 2
    };

    const BUF_INIT: [u8; 30] = *b"0000-00-00T00:00:00.000000000Z";

    let mut buf: [u8; 30] = BUF_INIT;
    buf[0] = b'0' + (year / 1000) as u8;
    buf[1] = b'0' + (year / 100 % 10) as u8;
    buf[2] = b'0' + (year / 10 % 10) as u8;
    buf[3] = b'0' + (year % 10) as u8;
    buf[5] = b'0' + (mon / 10) as u8;
    buf[6] = b'0' + (mon % 10) as u8;
    buf[8] = b'0' + (mday / 10) as u8;
    buf[9] = b'0' + (mday % 10) as u8;
    buf[11] = b'0' + (secs_of_day / 3600 / 10) as u8;
    buf[12] = b'0' + (secs_of_day / 3600 % 10) as u8;
    buf[14] = b'0' + (secs_of_day / 60 / 10 % 6) as u8;
    buf[15] = b'0' + (secs_of_day / 60 % 10) as u8;
    buf[17] = b'0' + (secs_of_day / 10 % 6) as u8;
    buf[18] = b'0' + (secs_of_day % 10) as u8;

    let i = match f.precision() {
        Some(0) => 19,
        precision => {
            let mut i = 20;
            let mut divisor = 100_000_000;
            let end = i + cmp::min(9, precision.unwrap_or(9));

            while i < end {
                buf[i] = b'0' + (nanos / divisor % 10) as u8;

                i += 1;
                divisor /= 10;
            }

            i
        }
    };

    buf[i] = b'Z';

    // we know our chars are all ascii
    f.write_str(str::from_utf8(&buf[..=i]).expect("Conversion to utf8 failed"))
}
