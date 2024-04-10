#[macro_use]
pub(crate) mod macros;

pub mod conversion;
pub mod jam;
pub mod pcboard;
pub mod qwk;
pub mod util;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
