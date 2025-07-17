use super::*;

pub fn test_coords(
    coord_space: &GraphCoordSpace,
    coords: LocalTileCoords,
    results: &mut CombinedTestResults,
) {
    let tile_y = coords[Y];
    let world_max_y = (coord_space.y_length_tiles - 1) as i8;

    // out of bounds
    if tile_y > world_max_y {
        // early exit
        // TODO: should this ever happen?
        *results = CombinedTestResults::OUTSIDE;
        return;
    }

    // if height checks are on, we know that tiles at the maximum Y coord will be
    // partially outside of the world.
    results.set_partial::<{ CombinedTestResults::HEIGHT_BIT }>(tile_y == world_max_y);
}

pub fn gen_top_tile_visibility_mask(section_height_in_top_tile: u16) -> u8x64 {
    let height_mask_small = (1_u8 << section_height_in_top_tile) - 1;
    mask64x8::from_bitmask(height_mask_small as u64)
        .to_int()
        .to_le_bytes()
}

pub fn gen_oob_above_incoming_sections(section_height_in_top_tile: u16) -> u8x64 {
    let shift_amount = (section_height_in_top_tile + 7) & 0b111;
    let height_mask_small = !((1_u8 << shift_amount) - 1);
    mask64x8::from_bitmask(height_mask_small as u64)
        .to_int()
        .to_le_bytes()
}
