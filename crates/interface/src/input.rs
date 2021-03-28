use bevy_input::prelude::*;

#[no_mangle]
pub fn just_pressed(input: &Input<KeyCode>, key: KeyCode) -> bool {
    input.just_pressed(key)
}

#[no_mangle]
pub fn just_released(input: &Input<KeyCode>, key: KeyCode) -> bool {
    input.just_released(key)
}

#[no_mangle]
pub fn pressed(input: &Input<KeyCode>, key: KeyCode) -> bool {
    input.pressed(key)
}

#[no_mangle]
pub fn keycode_a() -> KeyCode {
    KeyCode::A
}
#[no_mangle]
pub fn keycode_s() -> KeyCode {
    KeyCode::S
}
#[no_mangle]
pub fn keycode_d() -> KeyCode {
    KeyCode::D
}
#[no_mangle]
pub fn keycode_lshift() -> KeyCode {
    KeyCode::LShift
}
#[no_mangle]
pub fn keycode_space() -> KeyCode {
    KeyCode::Space
}
