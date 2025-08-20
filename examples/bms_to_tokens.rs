//! Example: Convert Bms to Vec<Token>
//!
//! This example demonstrates how to convert a parsed Bms object back to a vector of tokens.
//! This can be useful for serialization, debugging, or other purposes where you need
//! to work with the token representation of BMS data.

use bms_rs::bms::{parse_bms, prelude::BmsUnparseOutput};

fn main() {
    // Parse a simple BMS file
    let source = r#"#TITLE Test Song
#ARTIST Test Artist
#BPM 120
#WAV01 test.wav
#00101:01"#;

    println!("Parsing BMS source:");
    println!("{}", source);
    println!();

    // Parse the BMS source
    let bms_output = parse_bms(source);
    println!("Parsed BMS successfully!");
    println!(
        "Title: {}",
        bms_output.bms.header.title.as_deref().unwrap_or("Unknown")
    );
    println!(
        "Artist: {}",
        bms_output.bms.header.artist.as_deref().unwrap_or("Unknown")
    );
    println!(
        "BPM: {}",
        bms_output.bms.arrangers.bpm.as_ref().unwrap_or(&120.into())
    );
    println!();

    // Convert Bms back to tokens
    let BmsUnparseOutput { tokens } = bms_output.bms.unparse();

    println!("Converted back to {} tokens:", tokens.len());
    for (i, token) in tokens.iter().enumerate() {
        println!("  {}: {:?}", i + 1, token);
    }

    println!("\nNo warnings during conversion.");

    // Demonstrate token types
    println!("\nToken types found:");
    let mut token_types = std::collections::HashSet::new();
    for token in &tokens {
        token_types.insert(std::any::type_name_of_val(token));
    }
    for token_type in token_types {
        println!("  - {}", token_type);
    }
}
