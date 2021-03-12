use std::sync::Arc;
use wasmtime::*;
use bevy::{DefaultPlugins, prelude::*};
use bevy::input::Input;


fn main() -> anyhow::Result<()> {
    let mut config = Config::default();
    config
        .wasm_bulk_memory(true)
        .wasm_reference_types(true)
        .wasm_module_linking(true)
        .wasm_multi_memory(true);
    let engine = Arc::new(Engine::new(&config));

    let store = Store::new(engine.as_ref());

    let mut inp: Input<i32> = Input::default();
    inp.press(34);
    let inp_ref = ExternRef::new(inp);
    
    let module =
        Module::from_file(store.engine(), "./mods/as_sys/build/optimized.wat")?;

    let mut linker = Linker::new(&store);

    linker.func(
        "input", "just_pressed", |inp: Option<ExternRef>, arg: i32| -> i32 {
            let extern_ref = inp.expect("ExternRef should be present");
            let inp: &Input<i32> = extern_ref.data().downcast_ref().expect("ExternRef should be Input<i32>");
            inp.just_pressed(arg) as i32
        })?;

    let instance = linker.instantiate(&module)?;

    let just_pressed = instance.get_func("just_pressed").expect("whoops").get1::<Option<ExternRef>, i32>()?;

    println!("{:?}", just_pressed(Some(inp_ref))?);

    App::build()
        .add_plugins(DefaultPlugins)
        .run();

    Ok(())
}

struct WasmSystem {}

