//! Comprehensive diagnostics functionality example
//!
//! This example shows how to use bms-rs's diagnostics functionality to handle all types of warnings.

use bms_rs::bms::prelude::*;

fn main() {
    println!("=== BMS-RS Diagnostics Functionality Demo ===\n");

    // Demonstrate usage of all XXXWarningWithRange types
    demonstrate_warning_types();

    println!("\n=== Integration Usage Example ===\n");

    // Demonstrate complete integration workflow
    demonstrate_integration();
}

fn demonstrate_warning_types() {
    println!("1. Demonstrate ToAriadne implementation for all warning types:");

    let source_text = "#TITLE Demo\n#ARTIST Composer\n";
    let source = SimpleSource::new("demo.bms", source_text);

    // Create various types of warnings
    let warnings = [
        BmsWarning::PlayingWarning(PlayingWarning::TotalUndefined),
        BmsWarning::PlayingWarning(PlayingWarning::NoDisplayableNotes),
        BmsWarning::PlayingWarning(PlayingWarning::NoPlayableNotes),
    ];

    println!("   Created {} warnings", warnings.len());

    for (i, warning) in warnings.iter().enumerate() {
        println!("   Warning {}: {}", i + 1, warning);

        // Demonstrate ToAriadne trait usage
        let _report = warning.to_report(&source);
        println!("   -> Successfully converted to ariadne Report");
    }
}

fn demonstrate_integration() {
    println!("2. Complete integration workflow:");

    // Parse a BMS file that may produce warnings
    let bms_source = r#"#TITLE Integration Demo
#ARTIST Composer
#PLAYER 1
#GENRE Demo
#TOTAL 100

#00001:01000000
#00002:02000000
"#;

    println!("   Parsing BMS file...");
    let output = parse_bms(bms_source, default_config()).expect("must be parsed");

    println!(
        "   Parsing completed, found {} warnings",
        output.warnings.len()
    );

    if !output.warnings.is_empty() {
        println!("   Using convenience function to output warnings:");
        emit_bms_warnings("integration_demo.bms", bms_source, &output.warnings);

        println!("\n   Handling each warning manually:");
        let source = SimpleSource::new("integration_demo.bms", bms_source);
        let ariadne_source = ariadne::Source::from(bms_source);

        for (i, warning) in output.warnings.iter().enumerate() {
            println!("   Warning {}: {}", i + 1, warning);

            // Manually convert to Report and print
            let report = warning.to_report(&source);
            let _ = report.print(("integration_demo.bms".to_string(), ariadne_source.clone()));
        }
    }

    println!("\n   Successfully demonstrated complete diagnostics integration workflow!");
}
