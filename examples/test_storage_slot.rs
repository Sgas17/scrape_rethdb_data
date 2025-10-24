/// Test storage slot calculation
use scrape_rethdb_data::storage;

fn main() {
    // Test bitmap slot for word position -347 at slot 6 (tickBitmap)
    let word_pos: i16 = -347;
    let mapping_slot: u8 = storage::v3::TICK_BITMAP;

    let slot = storage::bitmap_slot(word_pos, mapping_slot);

    println!("Testing Storage Slot Calculation");
    println!("=================================");
    println!("Word position: {}", word_pos);
    println!("Mapping slot: {}", mapping_slot);
    println!("Calculated slot: {:#x}", slot);
    println!();

    // Also test a few other values
    println!("Additional tests:");
    println!("-----------------");

    let test_cases = vec![
        (100i16, storage::v3::TICK_BITMAP),
        (-1i16, storage::v3::TICK_BITMAP),
        (0i16, storage::v3::TICK_BITMAP),
    ];

    for (wp, ms) in test_cases {
        let s = storage::bitmap_slot(wp, ms);
        println!("Word pos {:4}, slot {}: {:#x}", wp, ms, s);
    }
}
