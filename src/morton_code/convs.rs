use super::{IntoBytes, LUTType, MortonCode, MortonStorage};
use amethyst::core::nalgebra::{Point3, Scalar};
use num_traits::{AsPrimitive, Zero};
use std::ops::BitOr;

impl<N> From<Point3<N>> for MortonCode<N>
where
    N: IntoBytes + MortonStorage + Scalar + Zero + BitOr<Output = N>,
    LUTType: AsPrimitive<N> + AsPrimitive<N::Storage>,
{
    fn from(p: Point3<N>) -> Self {
        MortonCode::new(p.x, p.y, p.z)
    }
}
impl<N> From<&Point3<N>> for MortonCode<N>
where
    N: IntoBytes + MortonStorage + Scalar + Zero + BitOr<Output = N>,
    LUTType: AsPrimitive<N> + AsPrimitive<N::Storage>,
{
    fn from(p: &Point3<N>) -> Self {
        MortonCode::new(p.x, p.y, p.z)
    }
}

impl<N> Into<Point3<N>> for MortonCode<N>
where
    N: IntoBytes + MortonStorage + Scalar + Zero + BitOr<Output = N>,
    LUTType: AsPrimitive<N> + AsPrimitive<N::Storage>,
{
    fn into(self) -> Point3<N> {
        let (x, y, z) = self.decode();
        Point3::new(x, y, z)
    }
}
