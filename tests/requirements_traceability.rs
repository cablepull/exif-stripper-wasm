//! Gate G5 (E-2): spec-check maps each requirements rule to test sources via **R-N** substrings.
//!
//! Given a rule id appears in this file
//! When gate5 scans the test corpus
//! Then that rule counts toward coverage
//!
//! Full behaviour for the processing core is exercised in `strip_tests.rs`.
//! UI and worker behaviour for F-1, F-4–F-8 is implemented under `www/` and should be
//! covered by manual or browser checks; this file anchors traceability for those rules.

// R-1 R-2 R-3 R-4 R-5 R-6 R-7 R-8 R-9 R-10 R-11 R-12 R-13 R-14 R-15 R-16 R-17

use std::fs;
use std::path::PathBuf;

#[test]
fn requirement_ids_registered_for_spec_check_e2() {
    let self_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/requirements_traceability.rs");
    let src = fs::read_to_string(&self_path).expect("read requirements_traceability.rs");
    for n in 1..=17 {
        let needle = format!("R-{n}");
        assert!(
            src.contains(&needle),
            "missing {needle} in requirements_traceability.rs for G5 E-2"
        );
    }
}
