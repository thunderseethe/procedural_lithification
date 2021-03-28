use wasmtime_wiggle::*;

from_witx!({
    witx: ["./crates/interface/res/math.witx"],
    errors: { errno => InterfaceError }
});


wasmtime_integration!({
    target: crate,
    witx: ["./crates/interface/res/math.witx"],
    ctx: GlamCtx,
    modules: {
        wasm_glam => {
            name: WasmGlam,
            docs: "An instantiated instance of Glam imports",
        },
    }
});

pub struct GlamCtx {}

#[derive(Debug)]
pub enum InterfaceError {}

impl GuestErrorType for types::Errno {
    fn success() -> Self {
        types::Errno::Ok
    }
}

impl types::UserErrorConversion for GlamCtx {
    fn errno_from_interface_error(&self, _: InterfaceError)-> Result<types::Errno, wiggle::Trap>  {
        Ok(types::Errno::Ok)
    }
}

impl wasm_glam::WasmGlam for GlamCtx {
    fn unit_z(&self) -> Result<types::Vec3, InterfaceError>  {
        let v = glam::Vec3::Z;
        Ok(types::Vec3 {
            x: v.x,
            y: v.y,
            z: v.z,
        })
    }

    fn normalize(&self, in_: &types::Vec3) -> Result<types::Vec3, InterfaceError>  {
        let v: glam::Vec3 = in_.into();
        Ok(v.normalize().into())
    }
    
    fn mul_vec3(&self, q: &types::Quat, v: &types::Vec3) -> Result<types::Vec3, InterfaceError>  {
        let q: glam::Quat = q.into();
        Ok(q.mul_vec3(v.into()).into())
    }
}

impl Into<glam::Vec3> for &types::Vec3 {
    fn into(self) -> glam::Vec3 {
       glam::Vec3::new(self.x, self.y, self.z) 
    }
}
impl Into<glam::Vec3> for types::Vec3 {
    fn into(self) -> glam::Vec3 {
        glam::Vec3::new(self.x, self.y, self.z)
    }
}
impl From<glam::Vec3> for types::Vec3 {
    fn from(v: glam::Vec3) -> Self {
       types::Vec3 {
           x: v.x,
           y: v.y,
           z: v.z
       } 
    }
}

impl Into<glam::Quat> for &types::Quat {
    fn into(self) -> glam::Quat {
        glam::Quat::from_xyzw(self.x, self.y, self.z, self.w)
    }
}
impl Into<glam::Quat> for types::Quat {
    fn into(self) -> glam::Quat {
        glam::Quat::from_xyzw(self.x, self.y, self.z, self.w)
    }
}