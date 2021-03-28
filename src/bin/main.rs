use std::{cell::RefCell, usize};
use std::{
    any::{type_name, TypeId},
    sync::Arc,
    rc::Rc,
};

use bevy::input::Input;
use wasi_cap_std_sync::WasiCtxBuilder;
use wasmtime::*;
use wasmtime_wasi::snapshots::preview_1::Wasi;
use std::mem::size_of;

const U32_LEN: usize = std::mem::size_of::<u32>();

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
        let ctx = Rc::new(RefCell::new(WasiCtxBuilder::new()
            .inherit_stdio()
            .build().expect("couldn't construct WasiCtx")));
        let wasi = Wasi::new(&store, ctx);
        let mut linker = Linker::new(&store);
        wasi.add_to_linker(&mut linker).expect("Failed to add wasi to linker");
        RefCell::new(linker)
    });
}


fn main() -> anyhow::Result<()> {
    let module = ENGINE.with(|engine| {
        Module::from_file(engine.as_ref(), "./mods/as_sys/build/optimized.wasm")
    })?;

    println!("Vec3.size({})", size_of::<Vec3>());

    use glam::f32::{Vec3, Quat};
    let instance_res: anyhow::Result<Instance> = LINKER.with(|linker| {
        let vec3_size = Global::new(linker.borrow().store(),
            GlobalType::new(ValType::I32, Mutability::Const),
            Val::I32(size_of::<Vec3>() as i32))?;

        linker.borrow_mut().func("console", "log", 
            |ctx: Caller<'_>, ptr: i32| -> () {
                let mem = ctx.get_export("memory")
                    .and_then(|ext| ext.into_memory())
                    .expect("expected export \"memory\"");

                let s = read_utf16_string(&mem, ptr as usize).unwrap();
                println!("{}", s);
            })?;

        linker.borrow_mut().define(
            "interface",
            "VEC3_SIZE",
            vec3_size)?;


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

        linker.borrow_mut().func("interface", "_unit_z", |ctx: Caller<'_>, ptr: i32| -> () {
            let unit_z = Vec3::Z;

            let mem = ctx.get_export("memory")
                .and_then(|ext| ext.into_memory())
                .expect("expected export \"memory\"");
            mem.write(ptr as usize, bytemuck::bytes_of(&unit_z)).expect("enough bytes were allocated for Vec3")
        })?;

        linker.borrow_mut().func("interface", "_normalize", |ctx: Caller<'_>, in_ptr: i32| -> () {
            let mem = ctx.get_export("memory")
                .and_then(|ext| ext.into_memory())
                .expect("expected export \"memory\"");

            let in_ptr = in_ptr as usize;
            // SAFE: this function will only be called while wasm mem is live so we can take reference to it without worry
            let vec3: &Vec3 = unsafe {
                let mem_s = mem.data_unchecked();
                bytemuck::from_bytes(&mem_s[in_ptr..(in_ptr+size_of::<Vec3>())])
            };
            let out = vec3.normalize();
            mem.write(in_ptr as usize, bytemuck::bytes_of(&out)).expect("normalize(): expected enough mem to write Vec3 at ptr");
        })?;

        linker.borrow_mut().func("interface", "_mul_vec3", |ctx: Caller<'_>, quat_ptr: i32, vec_ptr: i32, res:i32| -> () {
            let mem = ctx.get_export("memory")
                .and_then(|ext| ext.into_memory())
                .expect("expected export \"memory\"");

            let quat_ptr = quat_ptr as usize;
            let quat: Quat = unsafe {
                let mem_s = mem.data_unchecked();
                let mut buf: [u8; size_of::<Quat>()] = [0; size_of::<Quat>()];
                buf.copy_from_slice(&mem_s[quat_ptr..(quat_ptr+size_of::<glam::Quat>())]);
                std::mem::transmute(buf)
            };

            // SAFE: this function will only be called while wasm mem is live so we can take reference to it without worry
            let vec_ptr = vec_ptr as usize;
            let vec3: &Vec3 = unsafe {
                let mem_s = mem.data_unchecked(); 
                bytemuck::from_bytes(&mem_s[vec_ptr..(vec_ptr+size_of::<Vec3>())])
            };

            let out = quat.mul_vec3(vec3.clone());

            mem.write(res as usize, bytemuck::bytes_of(&out)).expect("mul_vec3(): expected enough mem to write Vec3 at ptr");
        })?;

        let instance = linker.borrow().instantiate(&module)?;
        Ok(instance)
    });

    let instance = instance_res?;

    let mem = instance
        .get_memory("memory")
        .expect("expected export \"memory\"");

    let alloc: TypedFunc<i32, i32> = instance.get_typed_func("alloc")?;
    let ptr = alloc.call(size_of::<Quat>() as i32)?;

    //let quat = Quat::IDENTITY;
    let quat = Quat::from_axis_angle(Vec3::new(1.0, 0.0, 1.0), 1.0);
    mem.write(ptr as usize, bytemuck::bytes_of(&quat))?;

    let q_ptr = alloc.call(size_of::<i32>() as i32)?;
    mem.write(q_ptr as usize, bytemuck::bytes_of(&ptr))?;

    let forward_vector = instance.get_func("forward_vector").expect("expected export \"forward_vector\"");
    let obj_ptr = forward_vector.typed::<i32, i32>()?.call(q_ptr)? as usize;   
    let v_ptr = read_u32(&mem, obj_ptr)? as usize;

    let mut buf: [u8; size_of::<Vec3>()] = [0; size_of::<Vec3>()];
    mem.read(v_ptr, &mut buf[..])?;
    println!("{:?}", buf);
    println!("{:?}", bytemuck::from_bytes::<Vec3>(&buf));
    //let ffi = unsafe {
    //    let mem_s = mem.data_unchecked();
    //    let name_ptr = read_u32(mem_s, obj_ptr);
    //    let val_ptr = read_u32(mem_s, obj_ptr + USIZE_LEN);
    //    let name: String = read_utf16_string(&mem, name_ptr as usize);
    //    let as_obj = AsObj::from_wasm_mem(&mem, val_ptr as usize);

    //    FfiObj {
    //        type_name: TypeName(name),
    //        type_id: generate_component_id(),
    //        obj: as_obj
    //    }
    //};

    //let reflect_component = ReflectComponent::from_type();

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

fn read_string(mem: &Memory, ptr: usize) -> Result<String, wasmtime::MemoryAccessError> {
    let str_size = read_u32(mem, ptr-4)? as usize;
    let mut buf = Vec::with_capacity(str_size);
    buf.reserve_exact(str_size);
    // String is utf8 encoded on wasm side so we can unwrap here
    mem.read(ptr, &mut buf[..]).map(|_|
        unsafe { String::from_utf8_unchecked(buf) })
}

fn read_utf16_string(mem: &Memory, ptr: usize) -> Result<String, wasmtime::MemoryAccessError> {
    let str_size = read_u32(&mem, ptr-4)? as usize;
    unsafe {
        let str_ptr = mem.data_ptr() as *const u16;
        Ok(String::from_utf16(std::slice::from_raw_parts(str_ptr, str_size / 2))
            .expect("Expected javascript string to be utf16 encoded"))
    }
}

fn read_u32(mem: &Memory, ptr: usize) -> Result<u32, wasmtime::MemoryAccessError> {
    let mut bytes: [u8; U32_LEN] = [0; U32_LEN];
    mem.read(ptr, &mut bytes).map(|_|
        u32::from_le_bytes(bytes))
}

trait FromWasmMem 
where
    Self: Sized,
{
    fn from_wasm_mem(memory: &Memory, prt: usize) -> Result<Self, wasmtime::MemoryAccessError>;
}

#[derive(Debug, Clone)]
struct AsObj {
    mm_info: u32,
    gc_info: u32,
    gc_info2: u32,
    rt_id: u32,
    payload: Vec<u8>,
}
impl FromWasmMem for AsObj {
    fn from_wasm_mem(mem: &Memory, ptr: usize) -> Result<Self, wasmtime::MemoryAccessError> {
        // Read AS header from behind initial pointer before reading payload
        let mm_info = read_u32(mem, ptr - 20)?;
        let gc_info = read_u32(mem, ptr - 16)?;
        let gc_info2 = read_u32(mem, ptr - 12)?;
        let rt_id = read_u32(mem, ptr - 8)?;
        let rt_size = read_u32(mem, ptr - 4)? as usize;

        // Read rt_size bytes from the ptr given to us.
        // This is the actual data of the object and is opaque to us.
        let payload = unsafe {
            let mem = mem.data_unchecked();
            (&mem[ptr..(ptr + rt_size)]).to_owned()
        };

        Ok(Self { mm_info, gc_info, gc_info2, rt_id, payload, })
    }
}

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
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
impl AsRef<str> for TypeName {
    fn as_ref(&self) -> &str {
        &self.0 
    }
}

/*
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
*/

/*
use std::any::Any;
use std::hash::{Hash, Hasher};


#[derive(Clone, Debug, PartialEq)]
struct FfiObj<T> {
    type_name: TypeName,
    type_id: ComponentId,
    obj: T,
}

impl<T: FromWasmMem> FfiObj<T> {
    fn from_wasm_mem(memory: &Memory, ptr: usize) -> Self {
        let data = unsafe { memory.data_unchecked() };
        let name_ptr = read_u32(data, ptr) as usize;
        let val_ptr = read_u32(data, ptr + USIZE_LEN) as usize;
        let type_name = TypeName(read_utf16_string(memory, name_ptr));
        let type_id = generate_component_id();
        FfiObj {
            type_name,
            type_id,
            obj: T::from_wasm_mem(memory, val_ptr)
        }
    }
}

impl<T> Reflect for FfiObj<T> 
where
    T: Clone + Hash + PartialEq + Send + Sync + 'static
{
    fn type_name(&self) -> &str {
        self.type_name.as_ref()
    }

    fn any(&self) -> &dyn Any {
        self as &dyn Any
    }

    fn any_mut(&mut self) -> &mut dyn Any {
        self as &mut dyn Any
    }

    fn apply(&mut self, value: &dyn Reflect) {
        todo!()
    }

    fn set(&mut self, value: Box<dyn Reflect>) -> Result<(), Box<dyn Reflect>> {
        if let Some(ffi_ref) = value.any().downcast_ref::<FfiObj<T>>() {
            FfiObj::clone_from(self, ffi_ref);
            Ok(())
        } else {
            Err(value)
        }
    }

    fn reflect_ref(&self) -> ReflectRef {
        //TODO: handle this more appropiately
        ReflectRef::Value(self)
    }

    fn reflect_mut(&mut self) -> ReflectMut {
        //TODO: handle this more appropiately
        ReflectMut::Value(self)
    }

    fn clone_value(&self) -> Box<dyn Reflect> {
        Box::new(self.clone()) as Box<dyn Reflect>
    }

    fn reflect_hash(&self) -> Option<u64> {
        use std::collections::hash_map::DefaultHasher;

        let mut hasher = DefaultHasher::new();
        self.type_name.hash(&mut hasher);
        self.type_id.hash(&mut hasher);
        self.obj.hash(&mut hasher);
        Some(hasher.finish())
    }

    fn reflect_partial_eq(&self, value: &dyn Reflect) -> Option<bool> {
        value.any().downcast_ref::<FfiObj<T>>()
            .map(|r| self.eq(r))
    }

    fn serializable(&self) -> Option<bevy::reflect::serde::Serializable> {
        // Handle this later when we need to serialized it later
        None
    }
}

use bevy::ecs::{System, SystemId};

fn generate_component_id() -> ComponentId {
    let uid = uuid::Uuid::new_v4();
    let (_, _, _, bytes) = uid.to_fields_le();
    ComponentId::new(u64::from_le_bytes(bytes.to_owned()) as usize)
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


//impl System for WasmSystem {
//    type In = ();
//
//    type Out = ();
//
//    fn name(&self) -> std::borrow::Cow<'static, str> {
//        self.module
//            .name()
//            .map(|name| name.to_string())
//            .map(Cow::Owned)
//            .unwrap_or_else(|| Cow::Owned("unnamed_wasm_system".to_string()))
//    }
//
//    fn id(&self) -> SystemId {
//        self.id
//    }
//
//    fn initialize(&mut self, world: &mut World) {
//        let instance = LINKER.with(|linker| {
//            linker.borrow().instantiate(&self.module).expect("Failed to instantiate module")
//        });
//        let initialize = instance.get_func("initialize").expect("Module must export \"initialize\"");
//        let ptr = initialize.typed::<(), i32>()
//            .expect("type to be () -> i32")
//            .call(()).expect("Don't trap please");
//        let memory = instance.get_memory("memory").expect("Expected export \"memory\"");
//        let ffi_obj: FfiObj<AsObj> = FfiObj::from_wasm_mem(&memory, ptr as usize);
//        
//        
//        ()
//    }
//
//    unsafe fn run_unsafe(&mut self, _input: Self::In, _world: &World) -> Self::Out {
//        todo!()
//    }
//
//    fn component_access(&self) -> &bevy::ecs::query::Access<bevy::ecs::component::ComponentId> {
//        todo!()
//    }
//
//    fn archetype_component_access(
//        &self,
//    ) -> &bevy::ecs::query::Access<bevy::ecs::archetype::ArchetypeComponentId> {
//        todo!()
//    }
//
//    fn apply_buffers(&mut self, world: &mut World) {
//        todo!()
//    }
//}
*/