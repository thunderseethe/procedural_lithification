// The entry file of your WebAssembly module.
import "wasi";
import { just_pressed as input_just_pressed } from './interface';

export function just_pressed(inp: externref): i32 {
  return input_just_pressed(inp, 34);
}

//export interface FlyCamera {
//  /// The speed the FlyCamera moves at. Defaults to `1.0`
//  speed: f32,
//  /// The maximum speed the FlyCamera can move at. Defaults to `0.5`
//  max_speed: f32,
//	/// The sensitivity of the FlyCamera's motion based on mouse movement. Defaults to `3.0`
//	sensitivity: f32,
//	/// The amount of deceleration to apply to the camera's motion. Defaults to `1.0`
//	friction: f32,
//	/// The current pitch of the FlyCamera in degrees. This value is always up-to-date, enforced by [FlyCameraPlugin](struct.FlyCameraPlugin.html)
//	pitch: f32,
//	/// The current pitch of the FlyCamera in degrees. This value is always up-to-date, enforced by [FlyCameraPlugin](struct.FlyCameraPlugin.html)
//	yaw: f32,
//	/// The current velocity of the FlyCamera. This value is always up-to-date, enforced by [FlyCameraPlugin](struct.FlyCameraPlugin.html)
//	velocity: Float32Array,
//	/// Key used to move forward. Defaults to `W`
//	key_forward: i32,
//	/// Key used to move backward. Defaults to `S
//	key_backward: i32,
//	/// Key used to move left. Defaults to `A`
//	key_left: i32,
//	/// Key used to move right. Defaults to `D`
//	key_right: i32,
//	/// Key used to move up. Defaults to `Space`
//	key_up: i32,
//	/// Key used to move forward. Defaults to `LShift`
//	key_down: i32,
//	/// If `false`, disable keyboard control of the camera. Defaults to `true`
//	enabled: bool,
//}

@unmanaged 
class FlyCamera {
  constructor(
    public x: i32,
    public y: i32,
    public z: i32,
  ) {}
}

function alloc_tuple<T>(name: String, val: T): usize {
  //let name_utf8 = String.UTF8.encode(name, false);
  let name_ptr: usize = changetype<usize>(name);
  let val_ptr: usize = changetype<usize>(val);
  let tuple_ptr = memory.data(sizeof<usize>() * 2);
  store<usize>(tuple_ptr, name_ptr);
  store<usize>(tuple_ptr + sizeof<usize>(), val_ptr);
  return tuple_ptr;
}

export function initialize(): usize {
  let fly_cam = new FlyCamera(1, 2, 4);
  return alloc_tuple("mods::as_sys::FlyCamera", fly_cam);
}
  //let velocity = new Float32Array(3);
  //velocity[0] = 0;
  //velocity[1] = 0;
  //velocity[2] = 0;
  //return {
  //  speed: 1.5,
  //  max_speed: 0.5,
  //  sensitivity: 3.0,
  //  friction: 1.0,
  //  pitch: 0.0,
  //  yaw: 0.0,
  //  velocity,
  //  key_forward: 32 /* W */,
  //  key_backward: 28 /* S */,
  //  key_left: 10 /* A */,
  //  key_right: 13 /* D */,
  //  key_up: 76 /* Space */,
  //  key_down: 114 /* LShift */,
  //  enabled: true,
  //};
//}
