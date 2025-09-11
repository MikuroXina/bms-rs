//! Diagnostics functionality usage example
//!
//! This example shows how to use bms-rs's diagnostics functionality to parse BMS files and display beautiful diagnostic information.

use bms_rs::bms::prelude::*;

fn main() {
    // An example BMS file containing warnings
    let bms_source = r#"#TITLE Test Song
#ARTIST Test Composer
#INVALID_COMMAND this will cause a warning
#TOTAL 100

#00111:01010101
#00211:02020202
"#;

    println!("Parsing BMS file and displaying diagnostic information...\n");

    // Parse BMS file
    let output = parse_bms::<KeyLayoutBeat>(bms_source);

    // Display parsing results
    println!(
        "Parsing successful! Found {} warnings",
        output.warnings.len()
    );

    // Use diagnostics functionality to output beautiful diagnostic information
    if !output.warnings.is_empty() {
        println!("\n=== Diagnostic Information ===");
        emit_bms_warnings("example.bms", bms_source, &output.warnings);
    }

    // Can also handle each warning manually
    println!("\n=== Manual Warning Handling Example ===");
    let source = SimpleSource::new("example.bms", bms_source);
    let ariadne_source = ariadne::Source::from(bms_source);

    for warning in &output.warnings {
        let report = warning.to_report(&source);
        let _ = report.print((source.name(), ariadne_source.clone()));
    }

    println!("\nBMS parsing completed!");
}
