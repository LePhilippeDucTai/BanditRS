//! Small, exact ports of Python standard-library behaviours that bandit's
//! observable output depends on. These modules are independent of bandit.

pub mod configparser;
pub mod csv;
pub mod datetime;
pub mod encoding;
pub mod fnmatch;
pub mod html;
pub mod json;
pub mod path;
pub mod pyformat;
pub mod splitlines;
pub mod unicode_escape;
pub mod urlquote;
pub mod xml;
pub mod yaml_emit;
pub mod yaml_load;
