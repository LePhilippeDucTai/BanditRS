//! INI subset of `configparser.ConfigParser`: `[section]`, `key = value` / `key: value`, keys lower-cased, full-line `#`/`;` comments, indented continuation lines, `[DEFAULT]` merged, duplicate keys/sections → error, BasicInterpolation (`%(k)s`, `%%`). `parse_ini_file(path)` returns the `[bandit]` section or `None` with the warning `Unable to parse config file %s or missing [bandit] section`.
//!
//! Status: stub (see PLAN.md).
