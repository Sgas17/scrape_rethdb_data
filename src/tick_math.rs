/// Tick math utilities for UniswapV3/V4 pools

/// Minimum and maximum tick values for full range
pub const MIN_TICK: i32 = -887272;
pub const MAX_TICK: i32 = 887272;

/// Calculate the word position for a given tick
/// Formula: word_pos = (tick / tickSpacing) >> 8
pub fn tick_to_word_pos(tick: i32, tick_spacing: i32) -> i16 {
    let compressed = tick / tick_spacing;
    (compressed >> 8) as i16
}

/// Calculate the bit position within a word for a given tick
/// Formula: bit_pos = (tick / tickSpacing) % 256
pub fn tick_to_bit_pos(tick: i32, tick_spacing: i32) -> u8 {
    let compressed = tick / tick_spacing;
    (compressed.rem_euclid(256)) as u8
}

/// Generate list of all word positions that could contain initialized ticks
/// This covers the full range from MIN_TICK to MAX_TICK
pub fn generate_word_positions(tick_spacing: i32) -> Vec<i16> {
    let min_word = tick_to_word_pos(MIN_TICK, tick_spacing);
    let max_word = tick_to_word_pos(MAX_TICK, tick_spacing);
    (min_word..=max_word).collect()
}

/// Extract initialized tick positions from a bitmap
/// Returns list of ticks that have their bit set in the bitmap
pub fn extract_ticks_from_bitmap(
    word_pos: i16,
    bitmap: u128,
    tick_spacing: i32,
) -> Vec<i32> {
    let mut ticks = Vec::new();

    // Check each bit in the lower 128 bits
    for bit_pos in 0..128u8 {
        if bitmap & (1u128 << bit_pos) != 0 {
            // Reconstruct tick from word_pos and bit_pos
            let compressed = ((word_pos as i32) << 8) | (bit_pos as i32);
            let tick = compressed * tick_spacing;

            // Validate tick is in valid range
            if tick >= MIN_TICK && tick <= MAX_TICK {
                ticks.push(tick);
            }
        }
    }

    ticks
}

/// Extract initialized tick positions from a full 256-bit bitmap
/// For bitmaps stored as U256
pub fn extract_ticks_from_bitmap_u256(
    word_pos: i16,
    bitmap_bytes: &[u8; 32],
    tick_spacing: i32,
) -> Vec<i32> {
    let mut ticks = Vec::new();

    // Process all 256 bits
    for byte_idx in 0..32 {
        let byte = bitmap_bytes[31 - byte_idx]; // Big-endian
        if byte == 0 {
            continue;
        }

        for bit_in_byte in 0..8u8 {
            if byte & (1 << bit_in_byte) != 0 {
                let bit_pos = (byte_idx as u16 * 8) + bit_in_byte as u16;

                // Reconstruct tick
                let compressed = ((word_pos as i32) << 8) | (bit_pos as i32);
                let tick = compressed * tick_spacing;

                // Validate tick range
                if tick >= MIN_TICK && tick <= MAX_TICK {
                    ticks.push(tick);
                }
            }
        }
    }

    ticks
}

/// Calculate a focused range of word positions around a current tick
/// Useful for querying a subset instead of all possible word positions
pub fn word_positions_around_tick(
    current_tick: i32,
    tick_spacing: i32,
    range_words: i16,
) -> Vec<i16> {
    let center_word = tick_to_word_pos(current_tick, tick_spacing);
    let min_word = (center_word - range_words).max(tick_to_word_pos(MIN_TICK, tick_spacing));
    let max_word = (center_word + range_words).min(tick_to_word_pos(MAX_TICK, tick_spacing));

    (min_word..=max_word).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tick_to_word_pos() {
        assert_eq!(tick_to_word_pos(0, 60), 0);

        // tick=887220, spacing=60 => compressed=14787 => word=14787>>8=57
        assert_eq!(tick_to_word_pos(887220, 60), 57);

        // Negative tick
        assert_eq!(tick_to_word_pos(-887220, 60), -58);
    }

    #[test]
    fn test_tick_to_bit_pos() {
        assert_eq!(tick_to_bit_pos(0, 60), 0);
        assert_eq!(tick_to_bit_pos(60, 60), 1);

        // tick=15360, spacing=60 => compressed=256 => bit=0 (wraps)
        assert_eq!(tick_to_bit_pos(15360, 60), 0);
    }

    #[test]
    fn test_generate_word_positions() {
        let positions = generate_word_positions(60);
        assert!(!positions.is_empty());
        assert!(positions[0] < 0); // Should start negative
        assert!(positions[positions.len() - 1] > 0); // Should end positive
    }

    #[test]
    fn test_extract_ticks() {
        // Bitmap with bits 0 and 5 set
        let bitmap = 0b100001u128;
        let ticks = extract_ticks_from_bitmap(0, bitmap, 60);

        assert_eq!(ticks.len(), 2);
        assert_eq!(ticks[0], 0); // bit 0 => compressed 0 => tick 0
        assert_eq!(ticks[1], 300); // bit 5 => compressed 5 => tick 300
    }

    #[test]
    fn test_word_positions_around_tick() {
        let positions = word_positions_around_tick(0, 60, 5);
        assert!(positions.contains(&0));
        assert!(positions.contains(&-5));
        assert!(positions.contains(&5));
        assert_eq!(positions.len(), 11); // -5 to +5 inclusive
    }
}
