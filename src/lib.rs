use wiggle::{GuestErrorType, from_witx};

from_witx!({
    witx_literal: "
        (typename $vec3
            (record (field $x f32) (field $y f32) (field $z f32)))
        (typename $quat
            (record (field $x f32) (field $y f32) (field $z f32) (field $w f32)))
        (typename $errno
            (enum (@witx tag u32)
                $ok))
        (module $vec3_mod
            (@interface func (export \"unit_z\")
                (result $r (expected $vec3 (error $errno)))
            )
            (@interface func (export \"normalize\")
                (param $in $vec3)
                (result $r (expected $vec3 (error $errno)))))
        (module $quat_mod
            (@interface func (export \"mul_vec3\") 
                (param $q $quat)
                (param $v $vec3)
                (result $r (expected $vec3 (error $errno)))))",
    errors: { errno => YourRichError }
});

struct YourCtxType {}

#[derive(Debug)]
pub enum YourRichError {}

impl GuestErrorType for types::Errno {
    fn success() -> Self {
        types::Errno::Ok
    }
}

impl types::UserErrorConversion for YourCtxType {
    fn errno_from_your_rich_error(&self, _: YourRichError)-> Result<types::Errno, wiggle::Trap>  {
        Ok(types::Errno::Ok)
    }
}


impl vec3_mod::Vec3Mod for YourCtxType {
    fn unit_z(&self) -> Result<types::Vec3, YourRichError>  {
        let v = glam::Vec3::Z;
        Ok(types::Vec3 {
            x: v.x,
            y: v.y,
            z: v.z,
        })
    }

    fn normalize(&self, in_: &types::Vec3) -> Result<types::Vec3, YourRichError>  {
        let v: glam::Vec3 = in_.into();
        Ok(v.normalize().into())
    }
}

impl quat_mod::QuatMod for YourCtxType {
    fn mul_vec3(&self, q: &types::Quat, v: &types::Vec3) -> Result<types::Vec3, YourRichError>  {
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

/*
            (@interface func (export \"normalize\")
                (param $a_vec $vec3)
                (result $r (expected $vec3))
            )
 */