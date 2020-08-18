/*
This module generates code to try efficiently convert some arbitrary `T: 'static` into
a `Primitive`. We use const evaluation to check type ids at compile time.

In the future when `min_specialization` is stabilized we could use it instead and avoid needing
the `'static` bound altogether.
*/

// Use consts to match a type with a conversion fn
pub(super) fn from_any<'v, T: ?Sized + 'static>(
    value: &'v T,
) -> Option<crate::internal::Primitive<'v>> {
    use std::any::TypeId;

    use crate::internal::Primitive;

    macro_rules! to_primitive {
        ($($ty:ty : ($const_ident:ident, $option_ident:ident),)*) => {
            trait ToPrimitive
            where
                Self: 'static,
            {
                const CALL: fn(&Self) -> Option<Primitive> = {
                    $(
                        const $const_ident: TypeId = TypeId::of::<$ty>();
                        const $option_ident: TypeId = TypeId::of::<Option<$ty>>();
                    );*

                    match TypeId::of::<Self>() {
                        $(
                            $const_ident => |v| Some(Primitive::from(unsafe { *(v as *const Self as *const $ty) })),
                            $option_ident => |v| Some({
                                let v = unsafe { *(v as *const Self as *const Option<$ty>) };
                                match v {
                                    Some(v) => Primitive::from(v),
                                    None => Primitive::None,
                                }
                            }),
                        )*

                        _ => |_| None,
                    }
                };

                fn to_primitive(&self) -> Option<Primitive> {
                    (Self::CALL)(self)
                }
            }

            impl<T: ?Sized + 'static> ToPrimitive for T {}
        }
    }

    // NOTE: The types here *must* match the ones used below when `const_type_id` is not available
    to_primitive![
        usize: (USIZE, OPTION_USIZE),
        u8: (U8, OPTION_U8),
        u16: (U16, OPTION_U16),
        u32: (U32, OPTION_U32),
        u64: (U64, OPTION_U64),

        isize: (ISIZE, OPTION_ISIZE),
        i8: (I8, OPTION_I8),
        i16: (I16, OPTION_I16),
        i32: (I32, OPTION_I32),
        i64: (I64, OPTION_I64),

        f32: (F32, OPTION_F32),
        f64: (F64, OPTION_F64),

        char: (CHAR, OPTION_CHAR),
        bool: (BOOL, OPTION_BOOL),
        &'static str: (STR, OPTION_STR),
    ];

    value.to_primitive()
}
