pub trait Numeric {
    fn is_numeric(&self) -> bool;
}

impl Numeric for String {
    fn is_numeric(&self) -> bool {
        !self.is_empty() && self.chars().all(|c| c.is_ascii_digit())
    }
}

impl Numeric for &str {
    fn is_numeric(&self) -> bool {
        !self.is_empty() && self.chars().all(|c| c.is_ascii_digit())
    }
}

impl<'a> Numeric for std::borrow::Cow<'a, str> {
    fn is_numeric(&self) -> bool {
        !self.is_empty() && self.chars().all(|c| c.is_ascii_digit())
    }
}

#[macro_export]
macro_rules! define_simple_adapter {
    ($name:ident, $rule_name:ident, $trait_name:ident, $code:expr, $msg:expr) => {
        pub fn $name<T: garde::rules::$rule_name::$trait_name>(v: &T, _ctx: &()) -> garde::Result {
            garde::rules::$rule_name::apply(v, ())
                .map_err(|_| garde::Error::new(format!("{} | {}", $code, $msg)))
        }
    };
}

#[macro_export]
macro_rules! define_length_adapter {
    ($name:ident, $mode:ident, $trait_name:ident, $code:expr, $msg:expr) => {
        pub fn $name<T: garde::rules::length::$mode::$trait_name>(
            min: usize,
            max: usize,
        ) -> impl Fn(&T, &()) -> garde::Result {
            let code = $code.to_string();
            let msg = $msg.to_string();
            move |v, _| {
                garde::rules::length::$mode::apply(v, (min, max)).map_err(|_| {
                    let unit = match stringify!($mode) {
                        "bytes" => "bytes",
                        "chars" => "characters",
                        "graphemes" => "graphemes",
                        "utf16" => "UTF-16 units",
                        _ => "units",
                    };
                    let detail = if min > 0 && max < usize::MAX {
                        format!("{} to {} {} required", min, max, unit)
                    } else if min > 0 {
                        format!("at least {} {} required", min, unit)
                    } else {
                        format!("up to {} {} allowed", max, unit)
                    };
                    garde::Error::new(format!("{} | {} ({})", code, msg, detail))
                })
            }
        }
    };
}

#[macro_export]
macro_rules! define_range_adapter {
    ($name:ident, $code:expr, $msg:expr) => {
        pub fn $name<T: garde::rules::range::Bounds + Copy>(
            min: Option<T::Size>,
            max: Option<T::Size>,
        ) -> impl Fn(&T, &()) -> garde::Result {
            let code = $code.to_string();
            let msg = $msg.to_string();
            move |v, _| {
                garde::rules::range::apply(v, (min, max)).map_err(|_| {
                    let detail = match (min, max) {
                        (Some(min), Some(max)) => format!("range {} to {}", min, max),
                        (Some(min), None) => format!("at least {}", min),
                        (None, Some(max)) => format!("up to {}", max),
                        (None, None) => "invalid range".to_string(),
                    };
                    garde::Error::new(format!("{} | {} ({})", code, msg, detail))
                })
            }
        }
    };
}

#[macro_export]
macro_rules! define_datetime_adapter {
    ($name:ident, $format:expr, $code:expr, $msg:expr) => {
        pub fn $name<T: AsRef<str>>(v: &T, _ctx: &()) -> garde::Result {
            let s = v.as_ref();
            if chrono::NaiveDateTime::parse_from_str(s, $format).is_ok() {
                Ok(())
            } else {
                Err(garde::Error::new(format!(
                    "{} | {} (Expected format: {})",
                    $code, $msg, $format
                )))
            }
        }
    };
}

#[macro_export]
macro_rules! define_numeric_adapter {
    ($name:ident, $code:expr, $msg:expr) => {
        pub fn $name<T: $crate::mode::rt::rterr::Numeric>(v: &T, _ctx: &()) -> garde::Result {
            if v.is_numeric() {
                Ok(())
            } else {
                Err(garde::Error::new(format!("{} | {}", $code, $msg)))
            }
        }
    };
}

#[macro_export]
macro_rules! define_garde_err_adapter {
    ($adapter_name:ident, $code:expr, $msg:expr) => {
        pub mod $adapter_name {
            #[allow(unused_imports)]
            use super::*;

            // Simple rules
            define_simple_adapter!(required, required, Required, $code, $msg);
            define_simple_adapter!(ascii, ascii, Ascii, $code, $msg);
            define_simple_adapter!(alphanumeric, alphanumeric, Alphanumeric, $code, $msg);
            define_simple_adapter!(email, email, Email, $code, $msg);
            define_simple_adapter!(url, url, Url, $code, $msg);
            define_simple_adapter!(credit_card, credit_card, CreditCard, $code, $msg);
            define_simple_adapter!(phone_number, phone_number, PhoneNumber, $code, $msg);

            // IP rules
            pub fn ip<T: garde::rules::ip::Ip>(
                kind: garde::rules::ip::IpKind,
            ) -> impl Fn(&T, &()) -> garde::Result {
                let code = $code.to_string();
                let msg = $msg.to_string();
                move |v, _| {
                    garde::rules::ip::apply(v, (kind,)).map_err(|_| {
                        let detail = match kind {
                            garde::rules::ip::IpKind::Any => "IP",
                            garde::rules::ip::IpKind::V4 => "IPv4",
                            garde::rules::ip::IpKind::V6 => "IPv6",
                        };
                        garde::Error::new(format!("{} | {} (Expected {})", code, msg, detail))
                    })
                }
            }

            pub fn ipv4<T: garde::rules::ip::Ip>(v: &T, _ctx: &()) -> garde::Result {
                ip(garde::rules::ip::IpKind::V4)(v, _ctx)
            }

            pub fn ipv6<T: garde::rules::ip::Ip>(v: &T, _ctx: &()) -> garde::Result {
                ip(garde::rules::ip::IpKind::V6)(v, _ctx)
            }

            // Length rules
            define_length_adapter!(length_simple, simple, Simple, $code, $msg);
            define_length_adapter!(length_bytes, bytes, Bytes, $code, $msg);
            define_length_adapter!(length_chars, chars, Chars, $code, $msg);
            define_length_adapter!(length_graphemes, graphemes, Graphemes, $code, $msg);
            define_length_adapter!(length_utf16, utf16, Utf16CodeUnits, $code, $msg);

            // Range rule
            define_range_adapter!(range, $code, $msg);

            // Contains rule
            pub fn contains<T: garde::rules::contains::Contains>(
                pat: &'static str,
            ) -> impl Fn(&T, &()) -> garde::Result {
                let code = $code.to_string();
                let msg = $msg.to_string();
                move |v, _| {
                    garde::rules::contains::apply(v, (pat,))
                        .map_err(|_| garde::Error::new(format!("{} | {} (Must contain \"{}\")", code, msg, pat)))
                }
            }

            // Prefix rule
            pub fn prefix<T: garde::rules::prefix::Prefix>(
                pat: &'static str,
            ) -> impl Fn(&T, &()) -> garde::Result {
                let code = $code.to_string();
                let msg = $msg.to_string();
                move |v, _| {
                    garde::rules::prefix::apply(v, (pat,))
                        .map_err(|_| garde::Error::new(format!("{} | {} (Must start with \"{}\")", code, msg, pat)))
                }
            }

            // Suffix rule
            pub fn suffix<T: garde::rules::suffix::Suffix>(
                pat: &'static str,
            ) -> impl Fn(&T, &()) -> garde::Result {
                let code = $code.to_string();
                let msg = $msg.to_string();
                move |v, _| {
                    garde::rules::suffix::apply(v, (pat,))
                        .map_err(|_| garde::Error::new(format!("{} | {} (Must end with \"{}\")", code, msg, pat)))
                }
            }

            // Pattern rule
            pub fn pattern<T: garde::rules::pattern::Pattern, M: garde::rules::pattern::Matcher>(
                matcher: &'static M,
            ) -> impl Fn(&T, &()) -> garde::Result {
                let code = $code.to_string();
                let msg = $msg.to_string();
                move |v, _| {
                    garde::rules::pattern::apply(v, (matcher,))
                        .map_err(|_| garde::Error::new(format!("{} | {} (Invalid pattern)", code, msg)))
                }
            }

            // Numeric rule
            define_numeric_adapter!(numeric, $code, $msg);
        }
    };
}
