//! Converting standard types into `ValueBag`s.

use super::ValueBag;

impl<'v> From<&'v str> for ValueBag<'v> {
    fn from(value: &'v str) -> Self {
        ValueBag::from_primitive(value)
    }
}

macro_rules! impl_from_primitive {
    ($($into_ty:ty,)*) => {
        $(
            impl<'v> From<$into_ty> for ValueBag<'v> {
                fn from(value: $into_ty) -> Self {
                    ValueBag::from_primitive(value)
                }
            }
        )*
    };
}

impl_from_primitive![
    (),
    usize,
    u8,
    u16,
    u32,
    u64,
    isize,
    i8,
    i16,
    i32,
    i64,
    f32,
    f64,
    char,
    bool,
];

#[cfg(test)]
mod tests {
    use crate::{
        std::{borrow::ToOwned, string::ToString},
        test::{IntoValueBag, Token},
    };

    #[test]
    fn test_into_display() {
        assert_eq!(42u64.into_value_bag().to_string(), "42");
        assert_eq!(42i64.into_value_bag().to_string(), "42");
        assert_eq!(42.01f64.into_value_bag().to_string(), "42.01");
        assert_eq!(true.into_value_bag().to_string(), "true");
        assert_eq!('a'.into_value_bag().to_string(), "a");
        assert_eq!(
            "a loong string".into_value_bag().to_string(),
            "a loong string"
        );
        assert_eq!(().into_value_bag().to_string(), "None");
    }

    #[test]
    fn test_into_structured() {
        assert_eq!(42u64.into_value_bag().to_token(), Token::U64(42));
        assert_eq!(42i64.into_value_bag().to_token(), Token::I64(42));
        assert_eq!(42.01f64.into_value_bag().to_token(), Token::F64(42.01));
        assert_eq!(true.into_value_bag().to_token(), Token::Bool(true));
        assert_eq!('a'.into_value_bag().to_token(), Token::Char('a'));
        assert_eq!(
            "a loong string".into_value_bag().to_token(),
            Token::Str("a loong string".to_owned())
        );
        assert_eq!(().into_value_bag().to_token(), Token::None);
    }
}
