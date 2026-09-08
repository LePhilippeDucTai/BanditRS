//! PyYAML `safe_load` semantics on top of `saphyr_parser` events: plain scalars resolved with the YAML 1.1 implicit resolvers (`yes/no/on/off/true/false` → bool, `0o17`/`0x1f`/`0b1`/`1_000`/sexagesimal → int, floats, `~`/`null`/empty → null; everything else str), quoted scalars are always strings; mappings keep insertion order (`ConfigValue::Map`). Errors → `ConfigError("Error parsing file.")` at the caller.
//!
//! Status: stub (see PLAN.md).
