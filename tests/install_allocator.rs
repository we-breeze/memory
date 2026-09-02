memory::install_global_allocator!();

#[test]
fn exported_macro_installs_the_allocator_in_a_consumer() {
    let values = Vec::<u64>::with_capacity(16);
    assert_eq!(values.capacity(), 16);
    assert_eq!(memory::heap().is_some(), cfg!(feature = "heap-stats"));
}
