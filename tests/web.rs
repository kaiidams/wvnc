#![cfg(target_arch = "wasm32")]

#[test]
fn wasm32_test() {
    assert_eq!(1 + 1, 2);
}
