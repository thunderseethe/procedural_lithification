// The entry file of your WebAssembly module.
import { just_pressed as input_just_pressed } from './interface/input';

export function just_pressed(inp: anyref): i32 {
  return input_just_pressed(inp, 34);
}