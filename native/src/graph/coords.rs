use std::ops::Index;

use core_simd::simd::prelude::*;

use super::{direction, i32x3, i8x3, u8x3, Coords3};
use crate::math::*;

pub struct GraphCoordSpace {
    // WARNING: if this is 128, there will be conversion problems when out of bounds above the
    // world.
    pub y_length_tiles: u8,
    pub xz_length_tiles: u8,

    pub world_bottom_section_y: i8,
    pub world_top_section_y: i8,
}

impl GraphCoordSpace {
    /// The lengths provided must be greater than or equal to 2, and less
    /// than 128. The lengths multiplied together must be less than or equal to
    /// 65536.
    pub fn new(
        y_length_tiles: u8,
        xz_length_tiles: u8,
        world_bottom_section_y: i8,
        world_top_section_y: i8,
    ) -> Self {
        Self {
            y_length_tiles,
            xz_length_tiles,
            world_bottom_section_y,
            world_top_section_y,
        }
    }

    // Index is packed in YZX ordering
    pub fn pack_index(&self, coords: LocalTileCoords) -> LocalTileIndex {
        // We don't want to wrap the coordinates on the Y axis, so we do a bounds check.
        #[cfg(debug_assertions)]
        assert!(
            self.tile_coords_in_bounds(coords),
            "Tile Y coordinate out of bounds - Y: {}, Graph Height: {}",
            coords[Y],
            self.y_length_tiles,
        );

        let x_wrapped = unsafe {
            (coords[X] as i16)
                .checked_rem_euclid(self.xz_length_tiles as i16)
                .unwrap_unchecked() as u16
        };
        let z_wrapped = unsafe {
            (coords[Z] as i16)
                .checked_rem_euclid(self.xz_length_tiles as i16)
                .unwrap_unchecked() as u16
        };

        LocalTileIndex(
            (((coords[Y] as u16 * self.xz_length_tiles as u16) + z_wrapped)
                * self.xz_length_tiles as u16)
                + x_wrapped,
        )
    }

    pub fn tile_coords_in_bounds(&self, coords: LocalTileCoords) -> bool {
        let y = coords[Y] as i16;
        (y >= 0) & (y < self.y_length_tiles as i16)
    }

    /// Calculates the tile coordinates in the graph and the section coordinates
    /// in that tile that the given global section coordinates are located.
    pub fn section_to_tile_coords(&self, section_coords: i32x3) -> (LocalTileCoords, u8x3) {
        let shifted_coords =
            section_coords - i32x3::from_xyz(0, self.world_bottom_section_y as i32, 0);

        // shift right by 3 is like a divide by 8, and each tile is 8 sections long on
        // each axis
        let scaled_coords = shifted_coords >> 3;
        let wrapped_xz = scaled_coords
            .rem_euclid(Simd::splat(self.xz_length_tiles as i32))
            .cast::<i8>();
        // exclude Y axis from wrapping
        let mut tile_coords = wrapped_xz;
        tile_coords[Y] = scaled_coords[Y] as i8;

        let section_coords_in_tile = shifted_coords.cast::<u8>() & Simd::splat(0b111);

        (LocalTileCoords(tile_coords), section_coords_in_tile)
    }

    /// Converts global block coordinates to local block coordinates
    pub fn block_to_local_coords(&self, block_coords: i32x3) -> i32x3 {
        let wrapped_xz = block_coords.rem_euclid(Simd::splat(
            self.xz_length_tiles as i32 * LocalTileCoords::LENGTH_IN_BLOCKS as i32,
        ));

        let world_bottom_block_y = (self.world_bottom_section_y as i32) << 4;
        let shifted_y = block_coords[Y] - world_bottom_block_y;

        let mut combined = wrapped_xz;
        combined[Y] = shifted_y;
        combined
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(align(8))] // speeds up packing and stepping slightly
pub struct LocalTileCoords(pub i8x3);

impl LocalTileCoords {
    pub const LENGTH_IN_BLOCKS: u8 = 128;
    pub const LENGTH_IN_SECTIONS: u8 = 8;

    // TODO: debug assert that Y didn't wrap when doing this
    pub fn step(self, direction: u8) -> Self {
        // position a 1-byte mask within a 6-byte SWAR vector, with each of the 6 bytes
        // representing a direction
        let dir_index = direction::to_index(direction);
        let shifted_byte = 0xFF_u64 << (dir_index * 8);

        // positive directions (indices 3, 4, and 5) need to be shifted into the lower
        // half. this lets us convert it to a 3-byte vector.
        // the mask is used to turn each present value in the mask into a positive 1.
        let pos_selected = (shifted_byte >> 24) as u32 & 0x01_01_01;

        // negative directions (indices 0, 1, and 2) are already in the bottom half, so
        // we mask out the top half. the mask here is also used to turn each present
        // value in the mask into a negative 1, or 0xFF in hex.
        let neg_selected = shifted_byte as u32 & 0xFF_FF_FF;

        // because we only allow 1 direction to be passed to this function, we know that
        // one of the two vectors will be empty. we can combine the positive and
        // negative vectors to get a vector that we know contains our increment value.
        let collapsed_selected = pos_selected | neg_selected;

        // each byte in the SWAR register is actually meant to represent an i8, so we
        // turn the bytes into a vector and cast it as such.
        let offset_vec = Simd::from_array(collapsed_selected.to_le_bytes())
            .resize(0)
            .cast::<i8>();

        Self(self.0 + offset_vec)
    }

    pub fn to_local_block_coords(self) -> i32x3 {
        self.0.cast::<i32>() << 7
    }
}

impl Coords3<i8> for LocalTileCoords {
    fn from_xyz(x: i8, y: i8, z: i8) -> Self {
        Self(Simd::from_xyz(x, y, z))
    }
}

impl Index<usize> for LocalTileCoords {
    type Output = i8;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

#[derive(Clone, Copy, PartialEq, Debug, Hash, Eq)]
pub struct LocalTileIndex(pub u16);

impl LocalTileIndex {
    pub fn to_usize(self) -> usize {
        self.0 as usize
    }
}

/// Relative to the camera position
#[derive(Clone, Copy)]
pub struct RelativeBoundingBox {
    pub(crate) min: f32x3,
    pub(crate) max: f32x3,
}

impl RelativeBoundingBox {
    // add 1 block to account for large block models
    pub const BOUNDING_BOX_EXTENSION_MIN: f32 = 1.0;
    // add 0.125 blocks to account for float imprecision
    pub const BOUNDING_BOX_EXTENSION: f32 = Self::BOUNDING_BOX_EXTENSION_MIN + 0.125;
    // the maximum area that we allow float imprecision to add is 0.25 blocks
    pub const BOUNDING_BOX_EXTENSION_MAX: f32 = Self::BOUNDING_BOX_EXTENSION_MIN + 0.25;

    pub fn new_extended(min: f32x3, max: f32x3) -> Self {
        Self {
            max: max + f32x3::splat(Self::BOUNDING_BOX_EXTENSION),
            min: min - f32x3::splat(Self::BOUNDING_BOX_EXTENSION),
        }
    }

    pub fn new(min: f32x3, max: f32x3) -> Self {
        Self { max, min }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::graph::direction::*;

    #[test]
    fn pack_index_test() {
        let storage_distance = 20;
        let y_length_sections = 24_u16;
        let xz_length_sections = (storage_distance as u16 * 2) + 1;

        let y_length_tiles = (y_length_sections.next_multiple_of(8) >> 3).max(2);
        let xz_length_tiles = (xz_length_sections.next_multiple_of(8) >> 3).max(2);

        let graph_total_tiles = y_length_tiles as u32 * (xz_length_tiles as u32).pow(2);

        let coord_space = GraphCoordSpace::new(y_length_tiles as u8, xz_length_tiles as u8, -4, 19);
        let mut index_coords_map = HashMap::<LocalTileIndex, LocalTileCoords>::new();

        for y in 0..y_length_tiles {
            for z in 0..xz_length_tiles {
                for x in 0..xz_length_tiles {
                    let coords = LocalTileCoords::from_xyz(x as i8, y as i8, z as i8);
                    let index = coord_space.pack_index(coords);

                    assert!(
                        (index.0 as u32) < graph_total_tiles,
                        "Index too large. Index: {:#018b}, Max: {:#018b}",
                        index.0,
                        graph_total_tiles
                    );

                    let entry = index_coords_map.get(&index);
                    if let Some(&existing_coords) = entry {
                        panic!(
                            "Duplicate Tile Index Found: {:?}\nCoords: {:?} and {:?}",
                            index.0, existing_coords.0, coords.0
                        );
                    } else {
                        index_coords_map.insert(index, coords);
                    }
                }
            }
        }

        // test a stray out of bounds index to see if it's handled
        {
            let coords = LocalTileCoords::from_xyz(-1, 0, -1);
            let index = coord_space.pack_index(coords);
            assert!(
                (index.0 as u32) < graph_total_tiles,
                "Index too large. Index: {:#018b}, Max: {:#018b}",
                index.0,
                graph_total_tiles
            );
        }

        // test wrapping on edges
    }

    // TODO: make this automatic
    #[test]
    fn step_test() {
        let coords = LocalTileCoords(Simd::from_xyz(10, 15, 31));

        let mut direction_set = ALL_DIRECTIONS;
        while direction_set != 0 {
            let direction = take_one(&mut direction_set);
            let stepped = coords.step(direction);
            println!("{} {stepped:?}", to_str(direction));
        }
    }
}
