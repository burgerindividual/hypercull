use context::{CameraArea, CombinedTestResults, GraphSearchContext};
use coords::{GraphCoordSpace, LocalTileIndex};
use core_simd::simd::prelude::*;
use direction::*;
use tile::Tile;
use visibility::*;

use self::coords::LocalTileCoords;
use crate::bitset::{self, BitSet};
use crate::ffi::FFITile;
use crate::math::*;

pub mod context;
pub mod coords;
pub mod direction;
pub mod tile;
pub mod visibility;

macro_rules! iterate_dirs {
    ($graph:ident, $context:ident, $($dir:expr),+) => {{
        const DIRS_SLICE: &[u8] = &[$($dir),+];
        const INCOMING_DIRS: u8 = opposite(bitset::from_elements_u8(DIRS_SLICE));
        const TRAVERSAL_DIRS: u8 = all_except(INCOMING_DIRS);

        if Graph::should_process::<INCOMING_DIRS>($context.camera_area) {
            $graph.iterate_dirs(
                $context,
                $context.iter_start_tile_coords,
                DIRS_SLICE,
                Self::process_tile::<INCOMING_DIRS, TRAVERSAL_DIRS>,
            );
        }
    }};
}

pub struct Tiles(Box<[Tile]>);

impl Tiles {
    fn get_mut(&mut self, index: LocalTileIndex) -> &mut Tile {
        unsafe { self.0.get_unchecked_mut(index.to_usize()) }
    }

    fn get(&self, index: LocalTileIndex) -> &Tile {
        unsafe { self.0.get_unchecked(index.to_usize()) }
    }
}

pub struct Graph {
    tiles: Tiles,

    pub coord_space: GraphCoordSpace,
    do_height_checks: bool,
    top_tile_visibility_mask: u8x64,
    oob_above_incoming_sections: u8x64,

    pub visible_tiles: Vec<FFITile>,
}

impl Graph {
    pub fn new(render_distance: u8, world_bottom_section_y: i8, world_top_section_y: i8) -> Self {
        // Same as Minecraft's ClientChunkCache.calculateStorageRange
        let storage_distance = render_distance.max(2) + 3;
        let y_length_sections =
            (world_top_section_y as i16 - world_bottom_section_y as i16 + 1) as u16;
        let xz_length_sections = (storage_distance as u16 * 2) + 1;

        assert!(y_length_sections > 0 && xz_length_sections > 0,
            "Invalid graph size. RD: {render_distance}, Bottom Section: {world_bottom_section_y}, Top Section: {world_top_section_y}"
        );

        // the minimum size of the graph is 2x2x2 tiles, so we can guarantee that each
        // tile will only be processed once. if any axis were allowed to have a
        // size of 1, when the graph search wraps past the edge of the graph, we
        // would land on the same tile that was just processed.
        let y_length_tiles = (y_length_sections.next_multiple_of(8) >> 3).max(2);
        let xz_length_tiles = (xz_length_sections.next_multiple_of(8) >> 3).max(2);

        let graph_total_tiles = y_length_tiles as usize * (xz_length_tiles as usize).pow(2);

        // Make sure graph can be represented with i8 coordinates that won't wrap
        // when going out-of-bounds, and u16 indices.
        const MAX_AXIS_LENGTH: u16 = 64;
        const MAX_TOTAL_TILES: usize = u16::MAX as usize + 1;
        assert!(
            y_length_tiles <= MAX_AXIS_LENGTH && xz_length_tiles <= MAX_AXIS_LENGTH && graph_total_tiles <= MAX_TOTAL_TILES,
            "Graph size is too large. Y Length (tiles): {y_length_tiles}, XZ Length (tiles): {xz_length_tiles}"
        );

        let tiles = unsafe {
            let mut tiles_uninit = Box::<[Tile]>::new_uninit_slice(graph_total_tiles);

            for tile_uninit in tiles_uninit.iter_mut() {
                tile_uninit.write(Default::default());
            }

            Tiles(tiles_uninit.assume_init())
        };

        let section_height_in_top_tile =
            y_length_sections % LocalTileCoords::LENGTH_IN_SECTIONS as u16;
        let do_height_checks = section_height_in_top_tile != 0;
        let top_tile_visibility_mask = if do_height_checks {
            tile::height::gen_top_tile_visibility_mask(section_height_in_top_tile)
        } else {
            Simd::splat(!0)
        };

        Self {
            tiles,
            coord_space: GraphCoordSpace::new(
                y_length_tiles as u8,
                xz_length_tiles as u8,
                world_bottom_section_y,
                world_top_section_y,
            ),
            do_height_checks,
            top_tile_visibility_mask,
            visible_tiles: Vec::with_capacity(128),
            oob_above_incoming_sections: tile::height::gen_oob_above_incoming_sections(
                section_height_in_top_tile,
            ),
        }
    }

    pub fn cull(&mut self, context: &GraphSearchContext) {
        self.clear();

        self.iterate_tiles(context);
    }

    pub fn clear(&mut self) {
        self.visible_tiles.clear();

        #[cfg(debug_assertions)]
        for tile in &mut self.tiles.0 {
            tile.processed = false;
        }
    }

    fn iterate_tiles(&mut self, context: &GraphSearchContext) {
        // Center
        if Self::should_process::<0>(context.camera_area) {
            self.process_tile::<0, ALL_DIRECTIONS>(
                context,
                self.coord_space.pack_index(context.iter_start_tile_coords),
                context.iter_start_tile_coords,
            );
        }

        // Axes
        iterate_dirs!(self, context, POS_X);
        iterate_dirs!(self, context, POS_Z);
        iterate_dirs!(self, context, POS_Y);
        iterate_dirs!(self, context, NEG_X);
        iterate_dirs!(self, context, NEG_Z);
        iterate_dirs!(self, context, NEG_Y);

        // Planes
        iterate_dirs!(self, context, NEG_Y, POS_X);
        iterate_dirs!(self, context, NEG_Z, POS_X);
        iterate_dirs!(self, context, POS_Z, POS_X);
        iterate_dirs!(self, context, POS_Y, POS_X);
        iterate_dirs!(self, context, NEG_Y, POS_Z);
        iterate_dirs!(self, context, POS_Y, POS_Z);
        iterate_dirs!(self, context, POS_Y, NEG_X);
        iterate_dirs!(self, context, POS_Z, NEG_X);
        iterate_dirs!(self, context, NEG_Z, NEG_X);
        iterate_dirs!(self, context, NEG_Y, NEG_X);
        iterate_dirs!(self, context, POS_Y, NEG_Z);
        iterate_dirs!(self, context, NEG_Y, NEG_Z);

        // Octants
        iterate_dirs!(self, context, NEG_Y, NEG_Z, POS_X);
        iterate_dirs!(self, context, NEG_Y, POS_Z, POS_X);
        iterate_dirs!(self, context, POS_Y, NEG_Z, POS_X);
        iterate_dirs!(self, context, POS_Y, POS_Z, POS_X);
        iterate_dirs!(self, context, POS_Y, POS_Z, NEG_X);
        iterate_dirs!(self, context, POS_Y, NEG_Z, NEG_X);
        iterate_dirs!(self, context, NEG_Y, POS_Z, NEG_X);
        iterate_dirs!(self, context, NEG_Y, NEG_Z, NEG_X);
    }

    /// `dirs` must not be empty when calling this
    #[inline(never)]
    fn iterate_dirs(
        &mut self,
        context: &GraphSearchContext,
        start_coords: LocalTileCoords,
        dirs: &[u8],
        process_tile_fn: fn(&mut Self, &GraphSearchContext, LocalTileIndex, LocalTileCoords),
    ) {
        let last_direction = dirs.len() == 1;
        let direction = dirs[0];
        let steps = context.direction_step_counts[to_index(direction)];
        let mut coords = start_coords;

        for _ in 0..steps {
            coords = coords.step(direction);

            // if the direction set is empty, we should stop recursing, and start processing
            // tiles
            if last_direction {
                let index = self.coord_space.pack_index(coords);

                process_tile_fn(self, context, index, coords);
            } else {
                self.iterate_dirs(context, coords, &dirs[1..], process_tile_fn);
            }
        }
    }

    fn should_process<const INCOMING_DIRS: u8>(camera_area: CameraArea) -> bool {
        match camera_area {
            CameraArea::Inside => true,
            CameraArea::Above => bitset::contains_u8(INCOMING_DIRS, direction::POS_Y),
            CameraArea::Below => bitset::contains_u8(INCOMING_DIRS, direction::NEG_Y),
        }
    }

    // the inlining of this function was a bit too aggressive
    #[inline(never)]
    fn process_tile<const INCOMING_DIRS: u8, const TRAVERSAL_DIRS: u8>(
        &mut self,
        context: &GraphSearchContext,
        index: LocalTileIndex,
        coords: LocalTileCoords,
    ) {
        #[cfg(debug_assertions)]
        println!("Current Tile - Coords: {:?} Index: {:?}", coords.0, index.0);

        // try to quickly determine whether we need to actually traverse the tile using
        // the frustum, fog, etc
        let relative_tile_pos = context.relative_tile_pos(coords);
        let test_result = context.test_tile(
            &self.coord_space,
            coords,
            relative_tile_pos,
            self.do_height_checks,
        );

        // tile needs to be re-borrowed multiple times in this method due to borrow
        // checker rules. these should get optimized out.
        let mut tile = self.tiles.get_mut(index);

        #[cfg(debug_assertions)]
        {
            assert!(!tile.processed);
            tile.processed = true;
        }

        if test_result == CombinedTestResults::OUTSIDE {
            // early exit
            tile.set_empty();
            return;
        }
        // All sections are visible initially, and each culling method masks it
        let mut visible_sections = tile::SECTIONS_FILLED;

        let intersecting_planes = test_result.get_intersecting_planes();
        if intersecting_planes != 0 {
            context.frustum.voxelize_planes(
                intersecting_planes,
                relative_tile_pos,
                &mut visible_sections,
            );
        }

        if test_result.is_partial::<{ CombinedTestResults::FOG_BIT }>() {
            context.voxelize_fog_cylinder(relative_tile_pos, &mut visible_sections);
        }

        if test_result.is_partial::<{ CombinedTestResults::HEIGHT_BIT }>() {
            visible_sections &= self.top_tile_visibility_mask;
        }

        if context.use_occlusion_culling {
            let mut traverse_start_sections = tile::SECTIONS_EMPTY;
            let mut incoming_dir_section_sets = [tile::SECTIONS_EMPTY; DIRECTION_COUNT];
            tile.outgoing_dir_section_sets = [tile::SECTIONS_EMPTY; DIRECTION_COUNT];

            // the center tile has no incoming directions, so there will be no data from
            // neighboring tiles. instead, we have to place the first set section manually.
            if INCOMING_DIRS == 0 {
                let section_index = tile::section_index(context.camera_section_in_tile);

                tile::set_bit(&mut traverse_start_sections, section_index);
                tile.setup_center_tile(section_index);
            } else {
                // tile goes out of scope here so we can observe neighboring tiles
                self.get_incoming_edges::<INCOMING_DIRS>(
                    coords,
                    context.camera_area,
                    visible_sections,
                    &mut traverse_start_sections,
                    &mut incoming_dir_section_sets,
                );
                // we then re-borrow the tile here so we can use it again
                tile = self.tiles.get_mut(index);

                // FAST PATH: if we start the traversal with all 0s, we'll end with all 0s.
                if traverse_start_sections == tile::SECTIONS_EMPTY {
                    // early exit
                    tile.set_empty();
                    return;
                }
            }

            // if we've hit this point, we know that there's atleast 1 section that has been
            // traversed in this tile. because of this, we know atleast part of
            // it is visible.

            let angle_visibility_masks = tile::angle::gen_visibility_masks(relative_tile_pos);

            #[cfg(debug_assertions)]
            let old_visible_sections = visible_sections;

            tile.traverse::<TRAVERSAL_DIRS>(
                traverse_start_sections,
                incoming_dir_section_sets,
                &context.outward_direction_masks,
                &angle_visibility_masks,
                &mut visible_sections,
            );

            #[cfg(debug_assertions)]
            {
                assert_eq!(
                    visible_sections & old_visible_sections,
                    visible_sections,
                    "traversal added incorrect visible sections"
                );
                for sections in tile.outgoing_dir_section_sets {
                    // TODO: should this be compared to old visible sections>
                    assert_eq!(
                        sections & visible_sections,
                        sections,
                        "traversal added incorrect outgoing dir sections"
                    );
                }
            }
        }

        if visible_sections != tile::SECTIONS_EMPTY {
            let local_section_coords = coords.0.cast::<i32>() << 3;
            let global_section_coords = context.global_section_offset + local_section_coords;

            self.visible_tiles
                .push(FFITile::new(global_section_coords, visible_sections));
        }
    }

    // TODO: consider not using const generics for this
    fn get_incoming_edges<const INCOMING_DIRS: u8>(
        &self,
        coords: LocalTileCoords,
        camera_area: CameraArea,
        visibility_mask: u8x64,
        traverse_start_sections: &mut u8x64,
        incoming_dir_section_sets: &mut [u8x64; DIRECTION_COUNT],
    ) {
        if bitset::contains_u8(INCOMING_DIRS, NEG_X) {
            let incoming_edge =
                self.get_incoming_edge::<NEG_X>(coords, camera_area) & visibility_mask;
            *traverse_start_sections |= incoming_edge;
            incoming_dir_section_sets[to_index(NEG_X)] = incoming_edge;
        }

        if bitset::contains_u8(INCOMING_DIRS, NEG_Y) {
            let incoming_edge =
                self.get_incoming_edge::<NEG_Y>(coords, camera_area) & visibility_mask;
            *traverse_start_sections |= incoming_edge;
            incoming_dir_section_sets[to_index(NEG_Y)] = incoming_edge;
        }

        if bitset::contains_u8(INCOMING_DIRS, NEG_Z) {
            let incoming_edge =
                self.get_incoming_edge::<NEG_Z>(coords, camera_area) & visibility_mask;
            *traverse_start_sections |= incoming_edge;
            incoming_dir_section_sets[to_index(NEG_Z)] = incoming_edge;
        }

        if bitset::contains_u8(INCOMING_DIRS, POS_X) {
            let incoming_edge =
                self.get_incoming_edge::<POS_X>(coords, camera_area) & visibility_mask;
            *traverse_start_sections |= incoming_edge;
            incoming_dir_section_sets[to_index(POS_X)] = incoming_edge;
        }

        if bitset::contains_u8(INCOMING_DIRS, POS_Y) {
            let incoming_edge =
                self.get_incoming_edge::<POS_Y>(coords, camera_area) & visibility_mask;
            *traverse_start_sections |= incoming_edge;
            incoming_dir_section_sets[to_index(POS_Y)] = incoming_edge;
        }

        if bitset::contains_u8(INCOMING_DIRS, POS_Z) {
            let incoming_edge =
                self.get_incoming_edge::<POS_Z>(coords, camera_area) & visibility_mask;
            *traverse_start_sections |= incoming_edge;
            incoming_dir_section_sets[to_index(POS_Z)] = incoming_edge;
        }
    }

    fn get_incoming_edge<const DIRECTION: u8>(
        &self,
        coords: LocalTileCoords,
        camera_area: CameraArea,
    ) -> u8x64 {
        // deal with fetching edge from out-of-bounds
        let top_tile_y = (self.coord_space.y_length_tiles - 1) as i8;
        if DIRECTION == POS_Y && coords[Y] == top_tile_y {
            if camera_area == CameraArea::Above {
                return self.oob_above_incoming_sections;
            } else {
                return tile::SECTIONS_EMPTY;
            }
        } else if DIRECTION == NEG_Y && coords[Y] == 0 {
            if camera_area == CameraArea::Below {
                return tile::OUT_OF_BOUNDS_BELOW_INCOMING_SECTIONS;
            } else {
                return tile::SECTIONS_EMPTY;
            }
        }

        let neighbor_coords = coords.step(DIRECTION);
        let neighbor_index = self.coord_space.pack_index(neighbor_coords);
        let neighbor_tile = self.tiles.get(neighbor_index);

        let neighbor_outgoing_sections =
            neighbor_tile.outgoing_dir_section_sets[to_index(opposite(DIRECTION))];

        match DIRECTION {
            NEG_X => tile::traversal::edge_pos_to_neg_x(neighbor_outgoing_sections),
            NEG_Y => tile::traversal::edge_pos_to_neg_y(neighbor_outgoing_sections),
            NEG_Z => tile::traversal::edge_pos_to_neg_z(neighbor_outgoing_sections),
            POS_X => tile::traversal::edge_neg_to_pos_x(neighbor_outgoing_sections),
            POS_Y => tile::traversal::edge_neg_to_pos_y(neighbor_outgoing_sections),
            POS_Z => tile::traversal::edge_neg_to_pos_z(neighbor_outgoing_sections),
            _ => unreachable!(),
        }
    }

    pub fn set_section(&mut self, section_coords: i32x3, visibility_data: u64) {
        let (tile_coords, section_coords_in_tile) =
            self.coord_space.section_to_tile_coords(section_coords);

        assert!(
            self.coord_space.tile_coords_in_bounds(tile_coords),
            "Tile Y coordinate out of bounds - Y: {}, Graph Height: {}",
            tile_coords[Y],
            self.coord_space.y_length_tiles,
        );

        let tile_index = self.coord_space.pack_index(tile_coords);
        let section_index = tile::section_index(section_coords_in_tile);

        #[cfg(debug_assertions)]
        println!(
            "Set Section - Section Coords: {:?}, Tile Coords: {:?}, Tile Index: {:?}, Section Index: {:?}, Vis: {}",
            section_coords, tile_coords.0, tile_index.0, section_index, visibility_data
        );

        let tile = self.tiles.get_mut(tile_index);

        for (array_idx, &bit_idx) in ARRAY_TO_BIT_IDX.iter().enumerate() {
            tile::modify_bit(
                &mut tile.connection_section_sets[array_idx],
                section_index,
                visibility_data.get_bit(bit_idx),
            );
        }
    }
}
