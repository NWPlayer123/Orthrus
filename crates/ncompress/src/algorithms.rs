//! This file is for shared functions across multiple modules in this crate. The filename may
//! change, and it's only for internal use right now.

// This is taken more or less from https://github.com/decompals/crunch64/pull/18/files
const HASH_BITS: usize = 15;
const HASH_SIZE: usize = 1 << HASH_BITS;
const HASH_MASK: usize = HASH_SIZE - 1;

const WINDOW_SIZE: usize = 0x1000;
const WINDOW_MASK: usize = WINDOW_SIZE - 1;

const MIN_MATCH: usize = 3;
const NULL: u16 = 0xFFFF;

const H_SHIFT: usize = (HASH_BITS + MIN_MATCH - 1) / MIN_MATCH;

/// Updates a hash value with the given input byte
fn update_hash(hash: usize, byte: u8) -> usize {
    ((hash << H_SHIFT) ^ (byte as usize)) & HASH_MASK
}

fn longest_common_prefix(a: &[u8], b: &[u8], max_len: usize) -> usize {
    for i in 0..max_len {
        if a[i] != b[i] {
            return i;
        }
    }
    max_len
}

// Finds the longest match in a 0x1000-byte sliding window, searching
// front-to-back with a minimum match size of 3 bytes. The algorithm is similar
// to the one described in section 4 of RFC 1951
// (https://www.rfc-editor.org/rfc/rfc1951.html#section-4), using a chained hash
// table of 3-byte sequences to find matches. Each character in the window is
// identified by its position & 0xFFF (like in a circular buffer).
pub(crate) struct Window<'a> {
    // Compression input
    input: &'a [u8],
    // Current position in the input
    input_pos: usize,
    // Hash value at the window start
    hash_start: usize,
    // Hash value at the current input position
    hash_end: usize,
    // Maximum possible sequence able to be found
    max_match_length: usize,
    // Head of hash chain for each hash value, or NULL
    head: [u16; HASH_SIZE],
    // Tail of hash chain for each hash value, or NULL
    tail: [u16; HASH_SIZE],
    // Next index in the hash chain, or NULL
    next: [u16; WINDOW_SIZE],
}

impl Window<'_> {
    pub(crate) fn new(input: &[u8], max_match_length: usize) -> Window {
        let mut hash = 0;
        for &b in input.iter().take(MIN_MATCH - 1) {
            hash = update_hash(hash, b);
        }

        Window {
            input,
            input_pos: 0,
            hash_start: hash,
            hash_end: hash,
            max_match_length,
            head: [NULL; HASH_SIZE],
            tail: [NULL; HASH_SIZE],
            next: [NULL; WINDOW_SIZE],
        }
    }

    // Advances the window by one byte, updating the hash chains.
    pub(crate) fn advance(&mut self) {
        if self.input_pos >= self.input.len() {
            return;
        }

        // Remove the oldest byte from the hash chain
        if self.input_pos >= WINDOW_SIZE {
            self.hash_start = update_hash(
                self.hash_start,
                self.input[self.input_pos - WINDOW_SIZE + MIN_MATCH - 1],
            );

            let head = self.head[self.hash_start];
            let next = self.next[head as usize];

            self.head[self.hash_start] = next;
            if next == NULL {
                self.tail[self.hash_start] = NULL;
            }
        }

        // Add the current byte to the hash chain
        if self.input_pos + MIN_MATCH < self.input.len() {
            self.hash_end = update_hash(self.hash_end, self.input[self.input_pos + MIN_MATCH - 1]);
            let tail = self.tail[self.hash_end];
            let pos = (self.input_pos & WINDOW_MASK) as u16;

            self.next[pos as usize] = NULL;
            self.tail[self.hash_end] = pos;
            if tail == NULL {
                self.head[self.hash_end] = pos;
            } else {
                self.next[tail as usize] = pos;
            }
        }

        self.input_pos += 1;
    }

    // Move the window forward the input position, and seach the window back-to-front for a match
    // at most `max_match_length` bytes long, returning the offset and length of the longest match
    // found. Successive searches can only be performed at increasing input positions.
    pub(crate) fn search(&mut self, search_pos: usize) -> (u32, u32) {
        if search_pos < self.input_pos {
            panic!("window moved backwards");
        } else if search_pos >= self.input.len() {
            return (0, 0);
        }

        let max_match = core::cmp::min(self.input.len() - search_pos, self.max_match_length);
        if max_match < MIN_MATCH {
            return (0, 0);
        }

        while self.input_pos < search_pos {
            self.advance();
        }

        let hash = update_hash(self.hash_end, self.input[self.input_pos + MIN_MATCH - 1]);
        let mut pos = self.head[hash];
        let mut best_len = MIN_MATCH - 1;
        let mut best_offset = 0;

        while pos != NULL {
            // Figure out the current match offset from `pos` (which is equal to `match_offset &
            // WINDOW_MASK`) using the fact that `1 <= input_pos - match_offset <=
            // WINDOW_SIZE`
            let match_offset =
                search_pos - 1 - (search_pos.wrapping_sub(pos as usize + 1) & WINDOW_MASK);

            if self.input[search_pos] == self.input[match_offset]
                && self.input[search_pos + 1] == self.input[match_offset + 1]
                && self.input[search_pos + best_len] == self.input[match_offset + best_len]
            {
                // The hash function guarantees that if the first two bytes match, the third byte
                // will too
                let candidate_len = MIN_MATCH
                    + longest_common_prefix(
                        &self.input[search_pos + MIN_MATCH..],
                        &self.input[match_offset + MIN_MATCH..],
                        max_match - MIN_MATCH,
                    );
                if candidate_len > best_len {
                    best_len = candidate_len;
                    best_offset = match_offset;
                    if best_len == max_match {
                        break;
                    }
                }
            }

            pos = self.next[pos as usize];
        }
        (best_offset as u32, best_len as u32)
    }
}
