// Test support for inspecting values

use crate::std::{fmt, str, string::String};

use super::internal;
use super::{Error, ValueBag};

pub(crate) trait IntoValueBag<'v> {
    fn into_value_bag(self) -> ValueBag<'v>;
}

impl<'v, T> IntoValueBag<'v> for T
where
    T: Into<ValueBag<'v>>,
{
    fn into_value_bag(self) -> ValueBag<'v> {
        self.into()
    }
}

#[derive(Debug, PartialEq)]
pub(crate) enum Token {
    U64(u64),
    I64(i64),
    F64(f64),
    Char(char),
    Bool(bool),
    Str(String),
    None,

    #[cfg(feature = "std")]
    Error,

    #[cfg(feature = "sval")]
    Sval,

    #[cfg(feature = "serde")]
    Serde,
}

#[cfg(test)]
impl<'v> ValueBag<'v> {
    pub(crate) fn to_token(&self) -> Token {
        struct TestVisitor(Option<Token>);

        impl<'v> internal::Visitor<'v> for TestVisitor {
            fn debug(&mut self, v: &dyn fmt::Debug) -> Result<(), Error> {
                self.0 = Some(Token::Str(format!("{:?}", v)));
                Ok(())
            }

            fn u64(&mut self, v: u64) -> Result<(), Error> {
                self.0 = Some(Token::U64(v));
                Ok(())
            }

            fn i64(&mut self, v: i64) -> Result<(), Error> {
                self.0 = Some(Token::I64(v));
                Ok(())
            }

            fn f64(&mut self, v: f64) -> Result<(), Error> {
                self.0 = Some(Token::F64(v));
                Ok(())
            }

            fn bool(&mut self, v: bool) -> Result<(), Error> {
                self.0 = Some(Token::Bool(v));
                Ok(())
            }

            fn char(&mut self, v: char) -> Result<(), Error> {
                self.0 = Some(Token::Char(v));
                Ok(())
            }

            fn str(&mut self, v: &str) -> Result<(), Error> {
                self.0 = Some(Token::Str(v.into()));
                Ok(())
            }

            fn none(&mut self) -> Result<(), Error> {
                self.0 = Some(Token::None);
                Ok(())
            }

            #[cfg(feature = "std")]
            fn error(&mut self, _: &dyn internal::error::Error) -> Result<(), Error> {
                self.0 = Some(Token::Error);
                Ok(())
            }

            #[cfg(feature = "sval")]
            fn sval(&mut self, _: &dyn internal::sval::Value) -> Result<(), Error> {
                self.0 = Some(Token::Sval);
                Ok(())
            }

            #[cfg(feature = "serde")]
            fn serde(&mut self, _: &dyn internal::serde::Serialize) -> Result<(), Error> {
                self.0 = Some(Token::Serde);
                Ok(())
            }
        }

        let mut visitor = TestVisitor(None);
        self.visit(&mut visitor).unwrap();

        visitor.0.unwrap()
    }
}
