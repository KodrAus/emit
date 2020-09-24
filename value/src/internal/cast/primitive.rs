/*
This module generates code to try efficiently convert some arbitrary `T: 'static` into
a `Primitive`.

In the future when `min_specialization` is stabilized we could use it instead and avoid needing
the `'static` bound altogether.
*/

use crate::internal::Primitive;

// Use consts to match a type with a conversion fn
pub(super) fn from_any<'v, T: 'static>(value: &'v T) -> Option<Primitive<'v>> {
    /*
    When the `const_type_id` feature is available, we can use it to determine
    a function to run at compile-time. It's like an emulated form of specialization.

    This approach is zero-cost at runtime.
    */
    #[cfg(value_bag_const_type_id)]
    {
        use crate::std::any::TypeId;

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

    /*
    When the `const_type_id` feature is not available, we create a static
    list of sorted type ids to check through a binary search.

    This approach has a small cost at runtime.
    */
    #[cfg(not(value_bag_const_type_id))]
    {
        #![allow(unused_unsafe)]

        use ctor::ctor;

        use crate::std::{
            any::{
                Any,
                TypeId,
            },
            cmp::Ordering,
        };

        macro_rules! type_ids {
            ($($ty:ty,)*) => {
                [
                    $(
                        (
                            std::any::TypeId::of::<$ty>(),
                            (|value| unsafe {
                                debug_assert_eq!(value.type_id(), std::any::TypeId::of::<$ty>());

                                // SAFETY: We verify the value is $ty before casting
                                let value = *(value as *const dyn std::any::Any as *const $ty);
                                Primitive::from(value)
                            }) as for<'a> fn(&'a (dyn std::any::Any + 'static)) -> Primitive<'a>
                        ),
                    )*
                    $(
                        (
                            std::any::TypeId::of::<Option<$ty>>(),
                            (|value| unsafe {
                                debug_assert_eq!(value.type_id(), std::any::TypeId::of::<Option<$ty>>());

                                // SAFETY: We verify the value is Option<$ty> before casting
                                let value = *(value as *const dyn std::any::Any as *const Option<$ty>);
                                if let Some(value) = value {
                                    Primitive::from(value)
                                } else {
                                    Primitive::None
                                }
                            }) as for<'a> fn(&'a (dyn std::any::Any + 'static)) -> Primitive<'a>
                        ),
                    )*
                ]
            };
        }

        // From: https://github.com/servo/rust-quicksort
        // We use this algorithm instead of the standard library's `sort_by` because it
        // works in no-std environments
        fn quicksort_helper<T, F>(arr: &mut [T], left: isize, right: isize, compare: &F)
        where F: Fn(&T, &T) -> Ordering {
            if right <= left {
                return
            }

            let mut i: isize = left - 1;
            let mut j: isize = right;
            let mut p: isize = i;
            let mut q: isize = j;
            unsafe {
                let v: *mut T = &mut arr[right as usize];
                loop {
                    i += 1;
                    while compare(&arr[i as usize], &*v) == Ordering::Less {
                        i += 1
                    }
                    j -= 1;
                    while compare(&*v, &arr[j as usize]) == Ordering::Less {
                        if j == left {
                            break
                        }
                        j -= 1;
                    }
                    if i >= j {
                        break
                    }
                    arr.swap(i as usize, j as usize);
                    if compare(&arr[i as usize], &*v) == Ordering::Equal {
                        p += 1;
                        arr.swap(p as usize, i as usize)
                    }
                    if compare(&*v, &arr[j as usize]) == Ordering::Equal {
                        q -= 1;
                        arr.swap(j as usize, q as usize)
                    }
                }
            }

            arr.swap(i as usize, right as usize);
            j = i - 1;
            i += 1;
            let mut k: isize = left;
            while k < p {
                arr.swap(k as usize, j as usize);
                k += 1;
                j -= 1;
                assert!(k < arr.len() as isize);
            }
            k = right - 1;
            while k > q {
                arr.swap(i as usize, k as usize);
                k -= 1;
                i += 1;
                assert!(k != 0);
            }

            quicksort_helper(arr, left, j, compare);
            quicksort_helper(arr, i, right, compare);
        }

        fn quicksort_by<T, F>(arr: &mut [T], compare: F) where F: Fn(&T, &T) -> Ordering {
            if arr.len() <= 1 {
                return
            }

            let len = arr.len();
            quicksort_helper(arr, 0, (len - 1) as isize, &compare);
        }

        #[ctor]
        static TYPE_IDS: [(TypeId, for<'a> fn(&'a (dyn std::any::Any + 'static)) -> Primitive<'a>); 30] = {
            // NOTE: The types here *must* match the ones used above when `const_type_id` is available
            let mut type_ids = type_ids![
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
                &'static str,
            ];

            quicksort_by(&mut type_ids, |&(ref a, _), &(ref b, _)| a.cmp(b));

            type_ids
        };

        if let Ok(i) = TYPE_IDS.binary_search_by_key(&value.type_id(), |&(k, _)| k) {
            Some((TYPE_IDS[i].1)(value))
        } else {
            None
        }
    }
}
