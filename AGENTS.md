# Project guidelines

- Check that the project compiles and that tests pass at every step, not just at
  the end of a task. Run `cargo check --workspace` (and `cargo test --workspace`
  once tests exist) after each meaningful change before moving on to the next one.
- Unless a method is in a performance-sensitive path, return values in some
  canonical, predictable order (e.g. sorted) rather than whatever order the
  underlying source happened to produce (hash map iteration, D-Bus reply
  order, etc.). Predictable ordering makes output stable across runs, which
  matters for visual inspection, diffing, and tests.
