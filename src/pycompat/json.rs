//! `json.dumps(obj, sort_keys=True, indent=2, separators=(",", ": "))`: recursive key sort, 2-space indentation, `[]`/`{}` for empty containers, `ensure_ascii` (every char >= 0x7f escaped as `\uXXXX`, surrogate pairs above the BMP). Build on `serde_json::to_string_pretty` + post-processing.
//!
//! Status: stub (see PLAN.md).
