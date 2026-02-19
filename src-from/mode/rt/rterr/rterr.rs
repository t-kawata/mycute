// ================================
// システムエラー
// ================================
pub const ERR_UNEXPECTED: &str = "E0001";
pub const ERR_DATABASE: &str = "E0002";
pub const ERR_AUTH: &str = "E0003";
pub const ERR_VALIDATION: &str = "E0004";
pub const ERR_INVALID_REQUEST: &str = "E0005";
pub const ERR_NOT_FOUND: &str = "E0012";

// ================================
// 具体バリデーションエラー
// ================================

// `#[garde(custom(required_err))]`
define_simple_adapter!(required_err, required, Required, "E0006", "Required.");
// For String fields that are required (length min = 1)
define_length_adapter!(required_simple_err, simple, Simple, "E0006", "Required.");

// `#[garde(custom(ascii_err))]`
define_simple_adapter!(ascii_err, ascii, Ascii, "E0007", "ASCII only.");

// `#[garde(custom(alphanumeric_err))]`
define_simple_adapter!(alphanumeric_err, alphanumeric, Alphanumeric, "E0008", "Alphanumeric only.");

// `#[garde(custom(email_err))]`
define_simple_adapter!(email_err, email, Email, "E0009", "Invalid email.");

// `#[garde(custom(url_err))]`
define_simple_adapter!(url_err, url, Url, "E0010", "Invalid URL.");

// `#[garde(custom(ip_v4_err))]` / `#[garde(custom(ip_v6_err))]`
pub mod ip_internal {
    define_garde_err_adapter!(p, "E0011", "Invalid IP.");
}
pub use ip_internal::p::{ip as ip_err, ipv4 as ip_v4_err, ipv6 as ip_v6_err};

// `#[garde(custom(credit_card_err))]`
define_simple_adapter!(credit_card_err, credit_card, CreditCard, "E0014", "Invalid credit card.");

// `#[garde(custom(phone_number_err))]`
define_simple_adapter!(phone_number_err, phone_number, PhoneNumber, "E0015", "Invalid phone number.");

// ## length モード一覧
// - `simple`: デフォルト。文字列はバイト数、コレクションは要素数で判定するため、日本語は1文字3〜4バイト換算となります。
// - `bytes`: 文字列のバイト数（`v.len()`）で判定し、DBのカラム長制限などの物理的なサイズ制約に合わせる際に使用します。
// - `chars`: Unicodeスカラ値数（`v.chars().count()`）で判定するため、標準的な日本語の文字数制限に最も適しています。
// - `graphemes`: 書記素クラスタ数で判定し、絵文字や結合文字を人間が見た通りの1文字として数えます（`unicode`機能が必要）。
// - `utf16`: UTF-16コード単位数で判定し、UTF-16ベースの外部システムやAPIと文字数定義を揃える必要がある際に使用します。
// ## 使い方
// `#[garde(custom(length_simple_err(min, max)))]`
// ## Length adapters
define_length_adapter!(length_simple_err, simple, Simple, "E0016", "Invalid length.");
define_length_adapter!(length_bytes_err, bytes, Bytes, "E0016", "Invalid length.");
define_length_adapter!(length_chars_err, chars, Chars, "E0016", "Invalid length.");
define_length_adapter!(length_graphemes_err, graphemes, Graphemes, "E0016", "Invalid length.");
define_length_adapter!(length_utf16_err, utf16, Utf16CodeUnits, "E0016", "Invalid length.");

// `#[garde(custom(range_err(Some(min), Some(max))))]`
// ## Range adapter
define_range_adapter!(range_err, "E0017", "Out of range.");

// `#[garde(custom(contains_err("pat")))]`
// `#[garde(custom(prefix_err("pat")))]`
// `#[garde(custom(suffix_err("pat")))]`
// `#[garde(custom(pattern_err(&REGEX)))]`
// ## String pattern adapters
pub mod pattern_internal {
    define_garde_err_adapter!(p, "E0018", "Invalid pattern.");
}
pub use pattern_internal::p::{
    contains as contains_err, pattern as pattern_err, prefix as prefix_err, suffix as suffix_err,
};

// `#[garde(custom(numeric_err))]` - 半角数字のみ
define_numeric_adapter!(numeric_err, "E0022", "Must be numeric.");

// `#[garde(custom(datetime_err))]` - 日時形式 "YYYY-MM-DDThh:mm:ss"
define_datetime_adapter!(datetime_err, "%Y-%m-%dT%H:%M:%S", "E0023", "Invalid datetime format.");
