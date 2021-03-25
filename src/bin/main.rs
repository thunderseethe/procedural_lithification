use std::cell::RefCell;
use std::{
    any::{type_name, TypeId},
    borrow::Cow,
    collections::HashMap,
    convert::TryInto,
    sync::Arc,
};

use bevy::input::Input;
use bevy::prelude::*;
use wasi_cap_std_sync::WasiCtxBuilder;
use wasmtime::*;
use wasmtime_wasi::Wasi;

const U32_LEN: usize = std::mem::size_of::<u32>();

#[derive(PartialEq, Eq, Hash)]
struct TypeName(String);
impl TypeName {
    pub fn of<T>() -> Self {
        Self(type_name::<T>().to_string())
    }

    // For types from wasm that do not have an innate type name
    pub(crate) fn dynamic_name<S: ToString>(string: S) -> Self {
        Self(string.to_string())
    }
}

struct TypeRegistry {
    registry: HashMap<TypeName, TypeId>
}
impl TypeRegistry {
    fn register<T: 'static>(&mut self) {
        let name = TypeName::of::<T>();

        self.registry.entry(name).or_insert(TypeId::of::<T>());
    }

    fn dynamic_register(&mut self, name: TypeName, id: TypeId) {
        self.registry.entry(name).or_insert(id);
    }

    fn get(&self, name: &TypeName) -> Option<&TypeId> {
        self.registry.get(name)
    }

    fn as_map(&mut self) -> &mut HashMap<TypeName, TypeId> {
        &mut self.registry
    }
}


thread_local! {
    pub static CONFIG: Config = {
        let mut config = Config::default();
        config
            .wasm_bulk_memory(true)
            .wasm_reference_types(true)
            .wasm_module_linking(true)
            .wasm_multi_memory(true);
        config
    };
    pub static ENGINE: Arc<Engine> = CONFIG.with(|config| {
        Arc::new(Engine::new(config).expect("couldn't constrct Engine"))
    });
    pub static LINKER: RefCell<Linker> = ENGINE.with(|engine| {
        let store = Store::new(engine.as_ref());
        let wasi = Wasi::new(&store, WasiCtxBuilder::new().inherit_stdio().build().expect("couldn't construct WasiCtx"));
        let mut linker = Linker::new(&store);
        wasi.add_to_linker(&mut linker).expect("Failed to add wasi to linker");
        RefCell::new(linker)
    });
}

use std::mem::size_of;
const USIZE_LEN: usize = size_of::<u32>();

fn main() -> anyhow::Result<()> {
    let module = ENGINE.with(|engine| {
        Module::from_file(engine.as_ref(), "./mods/as_sys/build/optimized.wasm")
    })?;

    let instance_res: anyhow::Result<Instance> = LINKER.with(|linker| {
        linker.borrow_mut().func(
            "interface",
            "just_pressed",
            |inp: Option<ExternRef>, arg: i32| -> i32 {
                let extern_ref = inp.expect("ExternRef should be present");
                let inp: &Input<i32> = extern_ref
                    .data()
                    .downcast_ref()
                    .expect("ExternRef should be Input<i32>");
                inp.just_pressed(arg) as i32
            },
        )?;
        let instance = linker.borrow().instantiate(&module)?;
        Ok(instance)
    });

    let instance = instance_res?;

    let setup = instance.get_func("initialize").expect("whoops");

    let obj_ptr = setup.typed::<(), i32>()?.call(())? as usize;

    let mem = instance
        .get_memory("memory")
        .expect("expected export \"memory\"");
    unsafe {
        let mem_s = mem.data_unchecked();
        let name_ptr = read_u32(mem_s, obj_ptr);
        let val_ptr = read_u32(mem_s, obj_ptr + USIZE_LEN);
        let name: String = read_wasm_string(&mem, name_ptr as usize);
        let as_obj = AsObj::from_wasm_mem(&mem, val_ptr as usize);
        println!("obj_ptr: {}, name: {}, val: {:?}", obj_ptr, name, as_obj)
    }
    //let as_obj = AsObj::from_wasm_mem(mem, obj_ptr);
    //println!("{:?}", as_obj);

    // TypeNames are used as public tracks
    //let mut type_ids: HashMap<TypeName, TypeId> = HashMap::new();
    //type_ids.insert(
    //    TypeName::of::<Time>(),
    //    TypeId::of::<Time>(),
    //);
    //println!("{}", type_name::<Time>());

    Ok(())
}

fn read_wasm_string(mem: &Memory, ptr: usize) -> String {
    unsafe { 
        let str_size = read_u32(mem.data_unchecked(), ptr-4) as usize;
        let mem_ptr = mem.data_ptr().add(ptr) as *const u16;
        String::from_utf16(std::slice::from_raw_parts(mem_ptr, str_size / 2))
            .expect("Expected javascript to be UTF16 encoded")
    }
}

fn read_u32(mem: &[u8], ptr: usize) -> u32 {
    let mut bytes: [u8; U32_LEN] = [0; U32_LEN];
    bytes.copy_from_slice(&mem[ptr..(ptr + U32_LEN)]);
    u32::from_le_bytes(bytes)
}

#[derive(Debug)]
struct AsObj {
    mm_info: u32,
    gc_info: u32,
    gc_info2: u32,
    rt_id: u32,
    payload: Vec<u8>,
}
impl AsObj {
    fn from_wasm_mem(memory: &Memory, ptr: usize) -> Self {
        let mem = unsafe { memory.data_unchecked() };
        let rt_size = read_u32(mem, ptr - 4) as usize;

        AsObj {
            mm_info: read_u32(mem, ptr - 20),
            gc_info: read_u32(mem, ptr - 16),
            gc_info2: read_u32(mem, ptr - 12),
            rt_id: read_u32(mem, ptr - 8),
            payload: (&mem[ptr..(ptr + rt_size)]).to_owned(),
        }
    }
}

use bevy::ecs::system::{System, SystemId};

fn generate_type_id() -> TypeId {
    let uid = uuid::Uuid::new_v4();
    let (_, _, _, bytes) = uid.to_fields_le();
    unsafe { std::mem::transmute(u64::from_le_bytes(bytes.to_owned())) }
}

use std::thread_local;

struct WasmSystem {
    id: SystemId,
    module: Module,
}
impl WasmSystem {
    fn new(module: Module) -> Self {
        Self {
            id: SystemId::new(),
            module,
        }
    }
}
impl System for WasmSystem {
    type In = ();

    type Out = ();

    fn name(&self) -> std::borrow::Cow<'static, str> {
        self.module
            .name()
            .map(|name| name.to_string())
            .map(Cow::Owned)
            .unwrap_or_else(|| Cow::Owned("unnamed_wasm_system".to_string()))
    }

    fn id(&self) -> SystemId {
        self.id
    }

    fn initialize(&mut self, world: &mut World) {
        let instance = LINKER.with(|linker| {
            linker.borrow().instantiate(&self.module).expect("Failed to instantiate module")
        });
        let initialize = instance.get_func("initialize").expect("Module must export \"initialize\"");
        let ptr = initialize.typed::<(), i32>()
            .expect("type to be () -> i32")
            .call(()).expect("Don't trap please");
        let memory = instance.get_memory("memory").expect("Expected export \"memory\"");
        let as_obj = AsObj::from_wasm_mem(&memory, ptr as usize);
        let mut type_registry = world.get_resource_mut::<TypeRegistry>().expect("expected TypeRegistry to be present");
        let as_obj_name = TypeName::dynamic_name(as_obj.rt_id);
        type_registry.as_map().entry(as_obj_name)
            .or_insert_with(|| generate_type_id());
        
        

        ()
    }

    unsafe fn run_unsafe(&mut self, _input: Self::In, _world: &World) -> Self::Out {
        todo!()
    }

    fn new_archetype(&mut self, archetype: &bevy::ecs::archetype::Archetype) {
        todo!()
    }

    fn component_access(&self) -> &bevy::ecs::query::Access<bevy::ecs::component::ComponentId> {
        todo!()
    }

    fn archetype_component_access(
        &self,
    ) -> &bevy::ecs::query::Access<bevy::ecs::archetype::ArchetypeComponentId> {
        todo!()
    }

    fn apply_buffers(&mut self, world: &mut World) {
        todo!()
    }

    fn check_change_tick(&mut self, change_tick: u32) {
        todo!()
    }

    fn is_send(&self) -> bool {
        todo!()
    }
}
