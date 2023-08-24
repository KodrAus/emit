use core::{cmp, fmt, str, str::FromStr, time::Duration};

use crate::value::{ToValue, Value};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Timestamp(Duration);

impl Timestamp {
    pub fn new(unix_time: Duration) -> Self {
        Timestamp(unix_time)
    }

    pub fn as_unix_time(&self) -> &Duration {
        &self.0
    }

    pub fn duration_since(self, earlier: Self) -> Option<Duration> {
        self.0.checked_sub(earlier.0)
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

#[derive(Debug)]
pub struct ParseTimestampError {}

fn parse_rfc3339(fmt: &str) -> Result<Timestamp, ParseTimestampError> {
    /*
    Original implementation: https://github.com/tokio-rs/prost/blob/master/prost-types/src/datetime.rs

    Licensed under Apache 2.0
    */

    if fmt.len() > 30 || fmt.len() < 19 {
        unimplemented!("invalid len {}", fmt.len());
    }

    if *fmt.as_bytes().last().unwrap() != b'Z' {
        unimplemented!("non-UTC")
    }

    let years = i64::from_str_radix(&fmt[0..4], 10).unwrap();
    let months = u8::from_str_radix(&fmt[5..7], 10).unwrap();
    let days = u8::from_str_radix(&fmt[8..10], 10).unwrap();
    let hours = u8::from_str_radix(&fmt[11..13], 10).unwrap();
    let minutes = u8::from_str_radix(&fmt[14..16], 10).unwrap();
    let seconds = u8::from_str_radix(&fmt[17..19], 10).unwrap();
    let nanos = if fmt.len() > 19 {
        let subsecond = &fmt[20..fmt.len() - 1];
        u32::from_str_radix(subsecond, 10).unwrap() * 10u32.pow(9 - subsecond.len() as u32)
    } else {
        0
    };

    let is_leap;
    let start_of_year;
    let year = years - 1900;

    // Fast path for years 1900 - 2038.
    if year as u64 <= 138 {
        let mut leaps: i64 = (year - 68) >> 2;
        if (year - 68).trailing_zeros() >= 2 {
            leaps -= 1;
            is_leap = true;
        } else {
            is_leap = false;
        }

        start_of_year = i128::from(31_536_000 * (year - 70) + 86400 * leaps);
    } else {
        let centuries: i64;
        let mut leaps: i64;

        let mut cycles: i64 = (year - 100) / 400;
        let mut rem: i64 = (year - 100) % 400;

        if rem < 0 {
            cycles -= 1;
            rem += 400
        }
        if rem == 0 {
            is_leap = true;
            centuries = 0;
            leaps = 0;
        } else {
            if rem >= 200 {
                if rem >= 300 {
                    centuries = 3;
                    rem -= 300;
                } else {
                    centuries = 2;
                    rem -= 200;
                }
            } else if rem >= 100 {
                centuries = 1;
                rem -= 100;
            } else {
                centuries = 0;
            }
            if rem == 0 {
                is_leap = false;
                leaps = 0;
            } else {
                leaps = rem / 4;
                rem %= 4;
                is_leap = rem == 0;
            }
        }
        leaps += 97 * cycles + 24 * centuries - i64::from(is_leap);

        start_of_year =
            i128::from((year - 100) * 31_536_000) + i128::from(leaps * 86400 + 946_684_800 + 86400);
    }

    let seconds_within_month = 86400 * u32::from(days - 1)
        + 3600 * u32::from(hours)
        + 60 * u32::from(minutes)
        + u32::from(seconds);

    let mut seconds_within_year = [
        0,           // Jan
        31 * 86400,  // Feb
        59 * 86400,  // Mar
        90 * 86400,  // Apr
        120 * 86400, // May
        151 * 86400, // Jun
        181 * 86400, // Jul
        212 * 86400, // Aug
        243 * 86400, // Sep
        273 * 86400, // Oct
        304 * 86400, // Nov
        334 * 86400, // Dec
    ][usize::from(months - 1)]
        + seconds_within_month;

    if is_leap && months > 2 {
        seconds_within_year += 86400
    }

    Ok(Timestamp::new(Duration::new(
        (start_of_year + i128::from(seconds_within_year))
            .try_into()
            .map_err(|_| ParseTimestampError {})?,
        nanos,
    )))
}

fn fmt_rfc3339(ts: Timestamp, f: &mut fmt::Formatter) -> fmt::Result {
    /*
    Original implementation: https://github.com/tokio-rs/prost/blob/master/prost-types/src/datetime.rs

    Licensed under Apache 2.0
    */

    let dur = ts.0;
    let secs: i64 = dur.as_secs().try_into().map_err(|_| fmt::Error)?;
    let nanos = dur.subsec_nanos();

    // 2000-03-01 (mod 400 year, immediately after feb29
    const LEAPOCH: i64 = 946_684_800 + 86400 * (31 + 29);
    const DAYS_PER_400Y: i32 = 365 * 400 + 97;
    const DAYS_PER_100Y: i32 = 365 * 100 + 24;
    const DAYS_PER_4Y: i32 = 365 * 4 + 1;
    const DAYS_IN_MONTH: [u8; 12] = [31, 30, 31, 30, 31, 31, 30, 31, 30, 31, 31, 29];

    // Note(dcb): this bit is rearranged slightly to avoid integer overflow.
    let mut days: i64 = (secs / 86_400) - (LEAPOCH / 86_400);
    let mut remsecs: i32 = (secs % 86_400) as i32;
    if remsecs < 0i32 {
        remsecs += 86_400;
        days -= 1
    }

    let mut qc_cycles: i32 = (days / i64::from(DAYS_PER_400Y)) as i32;
    let mut remdays: i32 = (days % i64::from(DAYS_PER_400Y)) as i32;
    if remdays < 0 {
        remdays += DAYS_PER_400Y;
        qc_cycles -= 1;
    }

    let mut c_cycles: i32 = remdays / DAYS_PER_100Y;
    if c_cycles == 4 {
        c_cycles -= 1;
    }
    remdays -= c_cycles * DAYS_PER_100Y;

    let mut q_cycles: i32 = remdays / DAYS_PER_4Y;
    if q_cycles == 25 {
        q_cycles -= 1;
    }
    remdays -= q_cycles * DAYS_PER_4Y;

    let mut remyears: i32 = remdays / 365;
    if remyears == 4 {
        remyears -= 1;
    }
    remdays -= remyears * 365;

    let mut years: i64 = i64::from(remyears)
        + 4 * i64::from(q_cycles)
        + 100 * i64::from(c_cycles)
        + 400 * i64::from(qc_cycles);

    let mut months: i32 = 0;
    while i32::from(DAYS_IN_MONTH[months as usize]) <= remdays {
        remdays -= i32::from(DAYS_IN_MONTH[months as usize]);
        months += 1
    }

    if months >= 10 {
        months -= 12;
        years += 1;
    }

    let years = years + 2000;
    let months = months + 3;
    let days = remdays + 1;
    let hours = remsecs / 3600;
    let minutes = remsecs / 60 % 60;
    let seconds = remsecs % 60;

    const BUF_INIT: [u8; 30] = *b"0000-00-00T00:00:00.000000000Z";

    let mut buf: [u8; 30] = BUF_INIT;
    buf[0] = b'0' + (years / 1000) as u8;
    buf[1] = b'0' + (years / 100 % 10) as u8;
    buf[2] = b'0' + (years / 10 % 10) as u8;
    buf[3] = b'0' + (years % 10) as u8;
    buf[5] = b'0' + (months / 10) as u8;
    buf[6] = b'0' + (months % 10) as u8;
    buf[8] = b'0' + (days / 10) as u8;
    buf[9] = b'0' + (days % 10) as u8;
    buf[11] = b'0' + (hours / 10) as u8;
    buf[12] = b'0' + (hours % 10) as u8;
    buf[14] = b'0' + (minutes / 10) as u8;
    buf[15] = b'0' + (minutes % 10) as u8;
    buf[17] = b'0' + (seconds / 10) as u8;
    buf[18] = b'0' + (seconds % 10) as u8;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timestamp_roundtrip() {
        let ts = Timestamp::new(Duration::new(1691961703, 17532));

        let fmt = ts.to_string();

        let parsed: Timestamp = fmt.parse().unwrap();

        assert_eq!(ts, parsed, "{}", fmt);
    }
}
