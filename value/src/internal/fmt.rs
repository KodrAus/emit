//! Integration between `Value` and `std::fmt`.
//!
//! This module allows any `Value` to implement the `fmt::Debug` and `fmt::Display` traits,
//! and for any `fmt::Debug` or `fmt::Display` to be captured as a `Value`.

use std::fmt;

use super::{cast, Inner, Visitor};
use crate::{Error, Slot, ValueBag};

impl<'v> ValueBag<'v> {
    /// Get a value from a debuggable type.
    ///
    /// This method will attempt to capture the given value as a well-known primitive
    /// before resorting to using its `Debug` implementation.
    pub fn capture_debug<T>(value: &'v T) -> Self
    where
        T: fmt::Debug + 'static,
    {
        cast::try_from_primitive(value).unwrap_or(ValueBag {
            inner: Inner::Debug {
                value,
                type_id: Some(cast::type_id::<T>()),
            },
        })
    }

    /// Get a value from a displayable type.
    ///
    /// This method will attempt to capture the given value as a well-known primitive
    /// before resorting to using its `Display` implementation.
    pub fn capture_display<T>(value: &'v T) -> Self
    where
        T: fmt::Display + 'static,
    {
        cast::try_from_primitive(value).unwrap_or(ValueBag {
            inner: Inner::Display {
                value,
                type_id: Some(cast::type_id::<T>()),
            },
        })
    }
}

impl<'s, 'f> Slot<'s, 'f> {
    /// Fill the slot with a debuggable value.
    ///
    /// The given value doesn't need to satisfy any particular lifetime constraints.
    ///
    /// # Panics
    ///
    /// Calling more than a single `fill` method on this slot will panic.
    pub fn fill_debug<T>(&mut self, value: T) -> Result<(), Error>
    where
        T: fmt::Debug,
    {
        self.fill(|visitor| visitor.debug(&value))
    }

    /// Fill the slot with a displayable value.
    ///
    /// The given value doesn't need to satisfy any particular lifetime constraints.
    ///
    /// # Panics
    ///
    /// Calling more than a single `fill` method on this slot will panic.
    pub fn fill_display<T>(&mut self, value: T) -> Result<(), Error>
    where
        T: fmt::Display,
    {
        self.fill(|visitor| visitor.display(&value))
    }
}

pub use self::fmt::{Debug, Display};

impl<'v> fmt::Debug for ValueBag<'v> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        struct DebugVisitor<'a, 'b: 'a>(&'a mut fmt::Formatter<'b>);

        impl<'a, 'b: 'a, 'v> Visitor<'v> for DebugVisitor<'a, 'b> {
            fn debug(&mut self, v: &dyn fmt::Debug) -> Result<(), Error> {
                fmt::Debug::fmt(v, self.0)?;

                Ok(())
            }

            fn display(&mut self, v: &dyn fmt::Display) -> Result<(), Error> {
                fmt::Display::fmt(v, self.0)?;

                Ok(())
            }

            fn u64(&mut self, v: u64) -> Result<(), Error> {
                fmt::Debug::fmt(&v, self.0)?;

                Ok(())
            }

            fn i64(&mut self, v: i64) -> Result<(), Error> {
                fmt::Debug::fmt(&v, self.0)?;

                Ok(())
            }

            fn f64(&mut self, v: f64) -> Result<(), Error> {
                fmt::Debug::fmt(&v, self.0)?;

                Ok(())
            }

            fn bool(&mut self, v: bool) -> Result<(), Error> {
                fmt::Debug::fmt(&v, self.0)?;

                Ok(())
            }

            fn char(&mut self, v: char) -> Result<(), Error> {
                fmt::Debug::fmt(&v, self.0)?;

                Ok(())
            }

            fn str(&mut self, v: &str) -> Result<(), Error> {
                fmt::Debug::fmt(&v, self.0)?;

                Ok(())
            }

            fn none(&mut self) -> Result<(), Error> {
                self.debug(&format_args!("None"))
            }

            #[cfg(feature = "std")]
            fn error(&mut self, v: &dyn std::error::Error) -> Result<(), Error> {
                fmt::Debug::fmt(v, self.0)?;

                Ok(())
            }

            #[cfg(feature = "sval")]
            fn sval(&mut self, v: &dyn super::sval::Value) -> Result<(), Error> {
                super::sval::fmt(self.0, v)
            }
        }

        self.visit(&mut DebugVisitor(f)).map_err(|_| fmt::Error)?;

        Ok(())
    }
}

impl<'v> fmt::Display for ValueBag<'v> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        struct DisplayVisitor<'a, 'b: 'a>(&'a mut fmt::Formatter<'b>);

        impl<'a, 'b: 'a, 'v> Visitor<'v> for DisplayVisitor<'a, 'b> {
            fn debug(&mut self, v: &dyn fmt::Debug) -> Result<(), Error> {
                fmt::Debug::fmt(v, self.0)?;

                Ok(())
            }

            fn display(&mut self, v: &dyn fmt::Display) -> Result<(), Error> {
                fmt::Display::fmt(v, self.0)?;

                Ok(())
            }

            fn u64(&mut self, v: u64) -> Result<(), Error> {
                fmt::Display::fmt(&v, self.0)?;

                Ok(())
            }

            fn i64(&mut self, v: i64) -> Result<(), Error> {
                fmt::Display::fmt(&v, self.0)?;

                Ok(())
            }

            fn f64(&mut self, v: f64) -> Result<(), Error> {
                fmt::Display::fmt(&v, self.0)?;

                Ok(())
            }

            fn bool(&mut self, v: bool) -> Result<(), Error> {
                fmt::Display::fmt(&v, self.0)?;

                Ok(())
            }

            fn char(&mut self, v: char) -> Result<(), Error> {
                fmt::Display::fmt(&v, self.0)?;

                Ok(())
            }

            fn str(&mut self, v: &str) -> Result<(), Error> {
                fmt::Display::fmt(&v, self.0)?;

                Ok(())
            }

            fn none(&mut self) -> Result<(), Error> {
                self.debug(&format_args!("None"))
            }

            #[cfg(feature = "std")]
            fn error(&mut self, v: &dyn std::error::Error) -> Result<(), Error> {
                fmt::Display::fmt(v, self.0)?;

                Ok(())
            }

            #[cfg(feature = "sval")]
            fn sval(&mut self, v: &dyn super::sval::Value) -> Result<(), Error> {
                super::sval::fmt(self.0, v)
            }
        }

        self.visit(&mut DisplayVisitor(f)).map_err(|_| fmt::Error)?;

        Ok(())
    }
}

impl<'v> From<&'v (dyn fmt::Debug)> for ValueBag<'v> {
    fn from(value: &'v (dyn fmt::Debug)) -> ValueBag<'v> {
        ValueBag {
            inner: Inner::Debug {
                value,
                type_id: None,
            },
        }
    }
}

impl<'v> From<&'v (dyn fmt::Display)> for ValueBag<'v> {
    fn from(value: &'v (dyn fmt::Display)) -> ValueBag<'v> {
        ValueBag {
            inner: Inner::Display {
                value,
                type_id: None,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::{IntoValueBag, Token};

    #[test]
    fn fmt_capture() {
        assert_eq!(ValueBag::capture_debug(&1u16).to_token(), Token::U64(1));
        assert_eq!(ValueBag::capture_display(&1u16).to_token(), Token::U64(1));

        assert_eq!(
            ValueBag::capture_debug(&Some(1u16)).to_token(),
            Token::U64(1)
        );
    }

    #[test]
    fn fmt_capture_args() {
        assert_eq!(
            ValueBag::from(&format_args!("a {}", "value") as &dyn Debug).to_string(),
            "a value"
        );
    }

    #[test]
    fn fmt_cast() {
        assert_eq!(
            42u32,
            ValueBag::capture_debug(&42u64)
                .to_u32()
                .expect("invalid value")
        );

        assert_eq!(
            "a string",
            ValueBag::capture_display(&"a string")
                .to_borrowed_str()
                .expect("invalid value")
        );
    }

    #[test]
    fn fmt_downcast() {
        #[derive(Debug, PartialEq, Eq)]
        struct Timestamp(usize);

        impl fmt::Display for Timestamp {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "time is {}", self.0)
            }
        }

        let ts = Timestamp(42);

        assert_eq!(
            &ts,
            ValueBag::capture_debug(&ts)
                .downcast_ref::<Timestamp>()
                .expect("invalid value")
        );

        assert_eq!(
            &ts,
            ValueBag::capture_display(&ts)
                .downcast_ref::<Timestamp>()
                .expect("invalid value")
        );
    }

    #[test]
    fn fmt_debug() {
        assert_eq!(
            format!("{:?}", "a string"),
            format!("{:?}", "a string".into_value_bag()),
        );

        assert_eq!(
            format!("{:04?}", 42u64),
            format!("{:04?}", 42u64.into_value_bag()),
        );
    }

    #[test]
    fn fmt_display() {
        assert_eq!(
            format!("{}", "a string"),
            format!("{}", "a string".into_value_bag()),
        );

        assert_eq!(
            format!("{:04}", 42u64),
            format!("{:04}", 42u64.into_value_bag()),
        );
    }
}
