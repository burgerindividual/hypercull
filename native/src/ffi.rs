#![allow(non_snake_case)]

use std::boxed::Box;
use std::ffi::c_void;

use context::GraphSearchContext;
use core_simd::simd::prelude::*;
use core_simd::simd::ToBytes;

use crate::graph::*;
use crate::math::*;
use crate::panic;
use crate::panic::PanicHandlerFn;

type JNIEnv = c_void;
type JClass = c_void;

#[repr(C)]
pub struct FFISlice<T> {
    pub data_ptr: *const T,
    pub count: usize,
}

impl<T> From<&[T]> for FFISlice<T> {
    fn from(value: &[T]) -> Self {
        Self {
            data_ptr: value.as_ptr(),
            count: value.len(),
        }
    }
}

#[repr(C)]
pub struct FFICamera {
    pub frustum_planes: [[f32; 4]; 6],
    pub pos: [f64; 3],
}

#[repr(C)]
pub struct FFITile {
    pub origin_section_coords: [i32; 3],
    pub visible_sections: [u64; 8],
}

impl FFITile {
    pub fn new(origin_section_coords: i32x3, visible_sections: u8x64) -> Self {
        Self {
            origin_section_coords: origin_section_coords.to_array(),
            visible_sections: u64x8::from_le_bytes(visible_sections).to_array(),
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn Java_com_github_burgerindividual_hypercull_client_ffi_HyperCullNativeLib_setPanicHandler(
    _: *mut JNIEnv,
    _: *mut JClass,
    panic_handler_fn_ptr: PanicHandlerFn,
) {
    if cfg!(feature = "panic_handler") {
        panic::set_panic_handler(panic_handler_fn_ptr);
    }
}

#[no_mangle]
pub extern "C" fn Java_com_github_burgerindividual_hypercull_client_ffi_HyperCullNativeLib_graphCreate(
    _: *mut JNIEnv,
    _: *mut JClass,
    render_distance: u8,
    world_bottom_section_y: i8,
    world_top_section_y: i8,
) -> *mut Graph {
    let graph = Box::new(Graph::new(
        render_distance,
        world_bottom_section_y,
        world_top_section_y,
    ));

    Box::leak(graph)
}

#[no_mangle]
pub unsafe extern "C" fn Java_com_github_burgerindividual_hypercull_client_ffi_HyperCullNativeLib_graphSetSection(
    _: *mut JNIEnv,
    _: *mut JClass,
    graph_ptr: *mut Graph,
    x: i32,
    y: i32,
    z: i32,
    visibility_bitmask: u64,
) {
    let graph = graph_ptr
        .as_mut()
        .expect("expected pointer to graph to be valid");

    graph.set_section(i32x3::from_xyz(x, y, z), visibility_bitmask);
}

#[no_mangle]
pub unsafe extern "C" fn Java_com_github_burgerindividual_hypercull_client_ffi_HyperCullNativeLib_graphSearch(
    _: *mut JNIEnv,
    _: *mut JClass,
    return_value_ptr: *mut FFISlice<FFITile>,
    graph_ptr: *mut Graph,
    camera_ptr: *const FFICamera,
    search_distance: f32,
    use_occlusion_culling: bool,
) {
    #[cfg(debug_assertions)]
    println!("start search --------------------------");

    let graph = graph_ptr
        .as_mut()
        .expect("expected pointer to graph to be valid");

    let camera = camera_ptr
        .as_ref()
        .expect("expected pointer to camera to be valid");

    let simd_camera_pos = Simd::from_array(camera.pos);
    let simd_frustum_planes = camera.frustum_planes.map(Simd::from_array);

    let context = GraphSearchContext::new(
        &graph.coord_space,
        simd_frustum_planes,
        simd_camera_pos,
        search_distance,
        use_occlusion_culling,
    );

    graph.cull(&context);

    #[cfg(debug_assertions)]
    {
        use std::collections::HashSet;

        let mut coords_set = HashSet::<[i32; 3]>::with_capacity(100);
        for tile in &graph.visible_tiles {
            if coords_set.contains(&tile.origin_section_coords) {
                panic!("Duplicate coords found in visible_tiles");
            } else {
                coords_set.insert(tile.origin_section_coords);
            }
        }
    }

    *return_value_ptr = graph.visible_tiles.as_slice().into();
}

#[no_mangle]
pub unsafe extern "C" fn Java_com_github_burgerindividual_hypercull_client_ffi_HyperCullNativeLib_graphDelete(
    _: *mut JNIEnv,
    _: *mut JClass,
    graph_ptr: *mut Graph,
) {
    let graph = graph_ptr
        .as_mut()
        .expect("expected pointer to graph to be valid");

    let graph_box = Box::from_raw(graph);
    drop(graph_box);
}
