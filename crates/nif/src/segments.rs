//! Segments of a `BSSegmentedTriShape`: runs of its triangles the game can
//! show or hide on their own. The distant-object blocks
//! (`meshes\landscape\lod\<world>\blocks\<world>.level4.x<X>.y<Y>.nif`, one
//! merged mesh per 4 × 4 cells) are such shapes, so the parts standing in
//! cells that are loaded in full can be hidden.
//!
//! The shape is a `NiTriShape` followed by the segment count (u32) and per
//! segment a flags byte, where its triangles start in the index list (u32,
//! each segment starting where the one before ended) and how many
//! triangles it has (u32): 9 bytes each, at the end of the block. Fallout
//! 3's blocks write an empty segment (a cell with nothing far-seen in it)
//! with start 0 rather than where the one before ended:
//! `wasteland.level4.x-4.y-8.nif` (Megaton's) has 16 segments, the third
//! and fourteenth empty. Read as they are, a whole block failed to split
//! and its part over the loaded cells was never hidden: Megaton's walls
//! drew their far-off versions over the real ones.

use crate::file::Nif;
use crate::reader::Reader;

/// One segment: its flags and its triangles (first, count).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Segment {
    pub flags: u8,
    pub first_triangle: u32,
    pub triangles: u32,
}

impl Nif {
    /// A `BSSegmentedTriShape` block's segments (empty for other blocks or
    /// when they don't add up).
    pub fn segments(&self, block: usize) -> Vec<Segment> {
        if block >= self.blocks().len() || self.block_type(block) != "BSSegmentedTriShape" {
            return Vec::new();
        }
        let bytes = self.block_bytes(block);
        // Find the count from the end: the only `n` whose 9-byte entries
        // fill the rest of the block after a count of `n`, and follow on.
        for n in 0..=bytes.len() / 9 {
            let Some(at) = bytes.len().checked_sub(9 * n + 4) else {
                break;
            };
            let count =
                u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]]);
            if count as usize != n {
                continue;
            }
            let mut r = Reader::new(&bytes[at + 4..], 0);
            let mut out = Vec::with_capacity(n);
            let mut next = 0u32;
            let mut ok = true;
            for _ in 0..n {
                let (Ok(flags), Ok(index), Ok(triangles)) =
                    (r.u8("flags"), r.u32("index"), r.u32("count"))
                else {
                    ok = false;
                    break;
                };
                // An empty one may start anywhere (Fallout 3 writes 0).
                if triangles == 0 {
                    out.push(Segment {
                        flags,
                        first_triangle: next / 3,
                        triangles,
                    });
                    continue;
                }
                if index != next {
                    ok = false;
                    break;
                }
                next = index + triangles * 3;
                out.push(Segment {
                    flags,
                    first_triangle: index / 3,
                    triangles,
                });
            }
            if ok && n > 0 {
                return out;
            }
        }
        Vec::new()
    }
}
