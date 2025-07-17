use core_simd::simd::prelude::*;
use std_float::StdFloat;

use super::coords::RelativeBoundingBox;
use super::tile::frustum::Frustum;
use crate::graph::*;

// TODO: move camera into its own struct
pub struct GraphSearchContext {
    pub frustum: Frustum,

    pub global_section_offset: i32x3,

    pub fog_distance: f32,

    // the camera coords (in blocks) relative to the local origin, which is the (0, 0, 0) point of
    // the graph. the representation here is slightly different than the representation in
    // CameraTransform.java, as camera_pos_frac can never be negative here.
    pub camera_pos_int: i32x3,
    pub camera_pos_frac: f32x3,
    pub camera_area: CameraArea,

    pub camera_section_in_tile: u8x3,

    pub iter_start_tile_coords: LocalTileCoords,
    pub direction_step_counts: Simd<u8, DIRECTION_COUNT>,

    pub use_occlusion_culling: bool,

    pub outward_direction_masks: [u8x64; DIRECTION_COUNT],
}

impl GraphSearchContext {
    pub fn new(
        coord_space: &GraphCoordSpace,
        frustum_planes: [f32x4; 6],
        global_camera_pos: f64x3,
        search_distance: f32,
        use_occlusion_culling: bool,
    ) -> Self {
        assert!(
            search_distance >= 0.0,
            "Search distance must not be negative - Search Distance: {search_distance}"
        );

        let max_search_distance =
            coord_space.xz_length_tiles as f32 * LocalTileCoords::LENGTH_IN_BLOCKS as f32;
        assert!(
            search_distance <= max_search_distance,
            "Search distance exceeds maximum for graph - Search Distance: {search_distance}, Maximum: {max_search_distance}"
        );

        let frustum = Frustum::new(frustum_planes);

        let global_camera_pos_floor = global_camera_pos.floor();
        // see the comment in CameraTransform.java for why we reduce the precision
        const PRECISION_MODIFIER: f32x3 = Simd::splat(128.0);
        let camera_pos_frac = ((global_camera_pos - global_camera_pos_floor).cast::<f32>()
            + PRECISION_MODIFIER)
            - PRECISION_MODIFIER;

        // Safety: We check if the conversion was lossless directly after the operation.
        // This should catch any particularly stupid camera positions.
        let global_camera_pos_int = unsafe { global_camera_pos_floor.to_int_unchecked::<i32>() };
        assert_eq!(
            global_camera_pos_int.cast::<f64>(),
            global_camera_pos_floor,
            "Camera position out of bounds: {global_camera_pos:?}",
        );

        let local_camera_pos_int = coord_space.block_to_local_coords(global_camera_pos_int);
        let global_section_offset = (global_camera_pos_int - local_camera_pos_int) >> 4;
        let local_camera_pos = local_camera_pos_int.cast::<f64>() + camera_pos_frac.cast::<f64>();

        let mut iter_start_tile_coords = (local_camera_pos_int >> 7).cast::<i8>();

        let global_top_block_y = ((coord_space.world_top_section_y as i32 + 1) << 4) - 1;
        let global_bottom_block_y = (coord_space.world_bottom_section_y as i32) << 4;

        let camera_area = if global_camera_pos_int[Y] > global_top_block_y {
            iter_start_tile_coords[Y] = coord_space.y_length_tiles as i8;
            CameraArea::Above
        } else if global_camera_pos_int[Y] < global_bottom_block_y {
            iter_start_tile_coords[Y] = -1;
            CameraArea::Below
        } else {
            CameraArea::Inside
        };

        let local_top_block_y = (global_top_block_y - global_bottom_block_y) as u16;

        let positive_step_counts = {
            let mut iter_end_block = (local_camera_pos + Simd::splat(search_distance as f64))
                .floor()
                .cast::<u16>();
            iter_end_block[Y] = iter_end_block[Y].clamp(0, local_top_block_y);
            let iter_end_tile = iter_end_block >> 7;
            (iter_end_tile.cast::<i16>() - iter_start_tile_coords.cast::<i16>())
                .max(Simd::splat(0))
                .cast::<u8>()
        };
        let negative_step_counts = {
            let mut iter_end_block = (local_camera_pos - Simd::splat(search_distance as f64))
                .floor()
                .cast::<i16>();
            iter_end_block[Y] = iter_end_block[Y].clamp(0, local_top_block_y as i16);
            let iter_end_tile = iter_end_block >> 7;
            (iter_start_tile_coords.cast::<i16>() - iter_end_tile)
                .max(Simd::splat(0))
                .cast::<u8>()
        };

        let direction_step_counts = simd_swizzle!(
            negative_step_counts.cast::<u8>(),
            positive_step_counts.cast::<u8>(),
            [0, 1, 2, 3, 4, 5,],
        );

        let camera_section_in_tile = (local_camera_pos_int >> 4).cast::<u8>() & Simd::splat(0b111);

        Self {
            frustum,
            global_section_offset,
            fog_distance: search_distance,
            camera_pos_int: local_camera_pos_int,
            camera_pos_frac,
            camera_area,
            camera_section_in_tile,
            iter_start_tile_coords: LocalTileCoords(iter_start_tile_coords),
            direction_step_counts,
            use_occlusion_culling,
            outward_direction_masks: tile::traversal::gen_outward_direction_masks(
                camera_section_in_tile,
            ),
        }
    }

    pub fn test_tile(
        &self,
        coord_space: &GraphCoordSpace,
        coords: LocalTileCoords,
        relative_pos: f32x3,
        do_height_checks: bool,
    ) -> CombinedTestResults {
        let mut results = CombinedTestResults::ALL_INSIDE;

        let bb = RelativeBoundingBox::new_extended(
            relative_pos,
            relative_pos + Simd::splat(LocalTileCoords::LENGTH_IN_BLOCKS as f32),
        );

        self.frustum.test_box(bb, &mut results);

        if results == CombinedTestResults::OUTSIDE {
            // early exit
            return results;
        }
        tile::fog::test_box(bb, self.fog_distance, &mut results);

        if results == CombinedTestResults::OUTSIDE {
            // early exit
            return results;
        }

        if do_height_checks {
            tile::height::test_coords(coord_space, coords, &mut results);
        }

        results
    }

    pub fn relative_tile_pos(&self, coords: LocalTileCoords) -> f32x3 {
        let pos_int = coords.to_local_block_coords() - self.camera_pos_int;
        pos_int.cast::<f32>() - self.camera_pos_frac
    }

    #[inline(never)]
    pub fn voxelize_fog_cylinder(&self, relative_tile_pos: f32x3, visible_sections: &mut u8x64) {
        *visible_sections &= tile::fog::voxelize_cylinder(relative_tile_pos, self.fog_distance);
    }
}

// If the value of this is not OUTSIDE, the following applies:
// Each test is represented by a single bit in this bit set. For each test:
// 1-bit = Partially inside, partially outside
// 0-bit = Inside
#[derive(PartialEq, Copy, Clone)]
pub struct CombinedTestResults(u16);

impl CombinedTestResults {
    pub const ALL_INSIDE: Self = Self(0b000);
    pub const OUTSIDE: Self = Self(!0);

    const FRUSTUM_PLANE_BITS: u16 = 0b00111111;
    pub const FOG_BIT: u16 = 0b01000000;
    pub const HEIGHT_BIT: u16 = 0b10000000;

    pub fn is_partial<const BIT: u16>(self) -> bool {
        bitset::contains_u16(self.0, BIT)
    }

    pub fn set_partial<const BIT: u16>(&mut self, value: bool) {
        self.0 |= (value as u16) << BIT.trailing_zeros();
    }

    pub fn set_intersecting_planes(&mut self, value: u8) {
        self.0 |= value as u16;
    }

    pub fn get_intersecting_planes(self) -> u8 {
        (self.0 & Self::FRUSTUM_PLANE_BITS) as u8
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum CameraArea {
    Inside,
    Above,
    Below,
}
