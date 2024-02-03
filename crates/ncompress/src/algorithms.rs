//! This file is for shared functions across multiple modules in this crate. The filename may
//! change, and it's only for internal use right now.

/// Maximum distance to look back in the buffer for a match (0xFFF for lower 3 nibbles + 1)
const MAX_LOOKBACK: usize = 0x1000;
/// Maximum number of bytes that can be copied from the lookback (0x12 threshold for a third byte +
/// 0xFF from that byte)
const MAX_COPY_SIZE: usize = 0x111;

/// Finds the biggest match in the lookback window for the bytes at the current input position.
pub(crate) fn find_match(input: &[u8], input_pos: usize) -> (usize, usize) {
    //Setup the initial location and size for the lookback window
    let mut window = core::cmp::max(input_pos.saturating_sub(MAX_LOOKBACK), 0);
    let mut window_size = 3;

    //This is the maximum we're able to copy in a single operation
    let max_match_size = core::cmp::min(input.len().saturating_sub(input_pos), MAX_COPY_SIZE);

    //If we can't copy more than two bytes (the size of the copy data) then don't bother looking
    if max_match_size < 3 {
        return (0, 0);
    }

    let mut window_offset = 0;
    let mut found_match_offset = 0;

    //Look for a match while we're within the range of the lookback buffer
    while window < input_pos && {
        window_offset = search_window(
            &input[input_pos..input_pos + window_size],
            &input[window..input_pos + window_size],
        );
        window_offset < input_pos - window
    } {
        //Expand the needle as long as it still matches the spot we found in the haystack
        while window_size < max_match_size
            && input[window + window_offset + window_size] == input[input_pos + window_size]
        {
            window_size += 1;
        }

        //If we've hit the max match size, we can't find a bigger match so just return it
        if window_size == max_match_size {
            return (window + window_offset, max_match_size);
        }

        found_match_offset = window + window_offset;
        window += window_offset + 1;
        window_size += 1;
    }

    //Return the biggest match we found, potentially none
    (
        found_match_offset,
        if window_size > 3 { window_size - 1 } else { 0 },
    )
}

/// Searches for the needle in the haystack using a modified version of Horspool's algorithm, and
/// returns the index of the first match
#[inline]
fn search_window(needle: &[u8], haystack: &[u8]) -> usize {
    //Check if we can even find the needle
    if needle.len() > haystack.len() {
        return haystack.len();
    }

    //Calculate the skip table for searching for end characters
    let skip_table = compute_skip_table(needle);

    let mut haystack_index = needle.len() - 1;
    'outer: loop {
        //Loop while we look for the last character of the needle, skipping through the haystack
        while haystack[haystack_index] != needle[needle.len() - 1] {
            haystack_index += skip_table[haystack[haystack_index] as usize] as usize;
        }
        haystack_index -= 1;

        //Found a possible match with the end character, now check if the rest of the needle
        // matches
        for needle_index in (0..needle.len() - 1).rev() {
            //If it doesn't, skip ahead and go back to searching for another end character
            if haystack[haystack_index] != needle[needle_index] {
                let mut skip: usize = skip_table[haystack[haystack_index] as usize] as usize;

                if needle.len() - needle_index > skip {
                    skip = needle.len() - needle_index;
                }
                haystack_index += skip;
                continue 'outer;
            }
            haystack_index = haystack_index.wrapping_sub(1);
        }
        return haystack_index.wrapping_add(1);
    }
}

/// Creates the skip table for Horspool's algorithm which contains how much farther to look forward
/// in the haystack in order for there to possibly be a match for the needle.
#[inline(always)]
fn compute_skip_table(needle: &[u8]) -> [u16; 256] {
    let mut table = [needle.len() as u16; 256];

    for i in 0..needle.len() {
        table[needle[i] as usize] = (needle.len() - i - 1) as u16;
    }

    table
}
