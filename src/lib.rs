#![warn(
    clippy::perf,
    clippy::pedantic,
    clippy::map_entry,
    clippy::boxed_local,
    clippy::format_in_format_args,
    clippy::to_string_in_format_args,
    clippy::large_const_arrays,
    clippy::large_enum_variant,
    clippy::manual_memcpy,
    clippy::missing_spin_loop,
    clippy::needless_collect,
    clippy::expect_fun_call,
    clippy::extend_with_drain,
    clippy::iter_nth,
    clippy::iter_overeager_cloned,
    clippy::iter_with_drain,
    clippy::manual_str_repeat,
    clippy::or_fun_call,
    clippy::single_char_pattern,
    clippy::unnecessary_to_owned,
    clippy::cmp_owned,
    clippy::redundant_clone,
    clippy::slow_vector_initialization,
    clippy::stable_sort_primitive,
    clippy::box_collection,
    clippy::redundant_allocation,
    clippy::useless_vec,
    clippy::vec_init_then_push,
    clippy::bytes_count_to_len,
    clippy::collapsible_str_replace,
    clippy::get_last_with_len,
    clippy::inefficient_to_string,
)]
#![deny(warnings)]

pub mod checks;
pub mod creature;
pub mod elements;
pub mod render;
pub mod rng;
pub mod ui;
pub mod world;

pub const W: usize = 1280;
pub const H: usize = 720;
