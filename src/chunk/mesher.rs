use super::block::Block;
use crate::octree::new_octree::*;
use crate::octree::{octant::Octant, octant_face::OctantFace, Octree};
use crate::volume::Cuboid;
use alga::general::SubsetOf;
use amethyst::core::nalgebra as na;
use amethyst::{
    core::nalgebra::{convert, try_convert, Point3, Scalar, Unit, Vector2, Vector3},
    renderer::PosNormTex,
};
use num_traits::{AsPrimitive, One, Zero};
use std::fmt::Display;
use std::{cmp::Ordering, fmt};

#[derive(Eq, PartialEq, Debug, Copy, Clone, FromPrimitive, ToPrimitive)]
enum Axis {
    X = 0,
    Y = 1,
    Z = 2,
}
impl Axis {
    pub fn next(&self) -> Self {
        match self {
            Axis::X => Axis::Y,
            Axis::Y => Axis::Z,
            Axis::Z => Axis::X,
        }
    }

    pub fn index(&self) -> usize {
        match self {
            Axis::X => 0,
            Axis::Y => 1,
            Axis::Z => 2,
        }
    }

    pub fn unit<N: Scalar + Zero + One>(&self) -> Unit<Vector3<N>> {
        match self {
            Axis::X => Vector3::x_axis(),
            Axis::Y => Vector3::y_axis(),
            Axis::Z => Vector3::z_axis(),
        }
    }

    pub fn front_face(&self) -> OctantFace {
        match self {
            Axis::X => OctantFace::Right,
            Axis::Y => OctantFace::Up,
            Axis::Z => OctantFace::Back,
        }
    }

    pub fn back_face(&self) -> OctantFace {
        match self {
            Axis::X => OctantFace::Left,
            Axis::Y => OctantFace::Down,
            Axis::Z => OctantFace::Front,
        }
    }
}

fn option_xor<A>(opt_a: Option<A>, opt_b: Option<A>) -> Option<A> {
    match (opt_a, opt_b) {
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        _ => None,
    }
}

pub struct Mesher<'a, O>
where
    O: FieldType,
{
    octree: &'a O,
    offset: Vector3<FieldOf<O>>,
    size: usize,
}
impl<'a, O: FieldType> Mesher<'a, O> {
    pub fn to_index<N: SubsetOf<usize> + Display>(&self, x_: N, y_: N, z_: N) -> usize {
        let x: usize = x_.to_superset();
        let y: usize = y_.to_superset();
        let z: usize = z_.to_superset();
        x + y * self.size + z * self.size * self.size
    }
}
impl<'a, O> Mesher<'a, O>
where
    O: OctreeLike + HasPosition<Position = Point3<FieldOf<O>>>,
    &'a O: IntoIterator,
{
    pub fn new(octree: &'a O) -> Self {
        let p = octree.position();
        Mesher {
            octree,
            offset: Vector3::new(p.x, p.y, p.z),
            size: O::diameter(),
        }
    }

    pub fn generate_quads_array(&self) -> Vec<Quad> {
        let mut quads = Vec::new();
        let size_iter: i32 = self.size as i32;
        let mut mask: Vec<Option<(Block, bool)>> = vec![None; self.size * self.size];
        let mut chunk: Vec<Option<Block>> = vec![None; self.size * self.size * self.size];
        self.octree.into_iter().for_each(|octant| {
            let bottom_left: Point3<usize> = na::convert(octant.bottom_left_front - self.offset);
            let top_right: Point3<usize> = {
                octant.top_right() - na::convert::<Vector3<FieldOf<O>>, Vector3<usize>>(self.offset)
            };
            for p in Cuboid::new(bottom_left, top_right).into_iter() {
                chunk[self.to_index(p.x, p.y, p.z)] = Some(*octant.data);
            }
        });
        let mut x: Point3<i32> = Point3::origin();
        for d in vec![Axis::X, Axis::Y, Axis::Z] {
            let u = d.next();
            let v = u.next();
            for dimension_cursor in -1..size_iter {
                let mut n = 0;
                for j in 0..size_iter {
                    for i in 0..size_iter {
                        x[d.index()] = dimension_cursor;
                        x[v.index()] = j;
                        x[u.index()] = i;
                        let q = d.unit();

                        let front_face = as_option(0 <= dimension_cursor)
                            .and_then(|_| try_convert(x))
                            .and_then(|p: Point3<u8>| chunk[self.to_index(p.x, p.y, p.z)]);
                        let back_face = as_option(dimension_cursor < size_iter - 1)
                            .and_then(|_| try_convert(x + q.as_ref()))
                            .and_then(|p: Point3<u8>| chunk[self.to_index(p.x, p.y, p.z)]);
                        mask[n] = option_xor(
                            front_face.map(|block| (block, false)),
                            back_face.map(|block| (block, true)),
                        );
                        n += 1;
                    }
                }
                n = 0;
                for j in 0..size_iter {
                    let mut i = 0;
                    while i < size_iter && n < mask.len() {
                        if mask[n].is_none() {
                            i += 1;
                            n += 1;
                            continue;
                        }

                        let (w, h) = self.determine_quad_dimensions(
                            &mask[n..],
                            self.size - i as usize,
                            self.size - j as usize,
                        );

                        let (block, is_back_face) = mask[n].unwrap();
                        x[d.index()] = dimension_cursor + 1;
                        x[u.index()] = i as i32;
                        x[v.index()] = j as i32;
                        let du: Vector3<i32> = u.unit().into_inner() * w as i32;
                        let dv: Vector3<i32> = v.unit().into_inner() * h as i32;

                        quads.push(Quad::new(
                            x,
                            x + dv,
                            x + du,
                            x + du + dv,
                            block,
                            if is_back_face {
                                d.back_face()
                            } else {
                                d.front_face()
                            },
                        ));

                        for l in 0..h {
                            for k in n..(n + w) {
                                mask[k + l * self.size] = None;
                            }
                        }

                        i += w as i32;
                        n += w;
                    }
                }
            }
        }
        return quads;
    }

    fn determine_quad_dimensions<E: PartialEq>(
        &self,
        mask: &[E],
        max_width: usize,
        max_height: usize,
    ) -> (usize, usize) {
        let test = &mask[0];
        let w = mask
            .iter()
            .take(max_width)
            .take_while(|ele| test.eq(ele))
            .count();
        let h = mask
            .chunks(self.size)
            .take(max_height)
            .take_while(|row| row.iter().take(w).all(|ele| test.eq(ele)))
            .count();
        (w, h)
    }
}

#[derive(Eq, PartialEq)]
pub struct Quad {
    bottom_left: Point3<i32>,
    top_left: Point3<i32>,
    bottom_right: Point3<i32>,
    top_right: Point3<i32>,
    block: Block,
    pub face: OctantFace,
}
impl Quad {
    pub fn new(
        bottom_left: Point3<i32>,
        top_left: Point3<i32>,
        bottom_right: Point3<i32>,
        top_right: Point3<i32>,
        block: Block,
        face: OctantFace,
    ) -> Self {
        Quad {
            bottom_left,
            top_left,
            bottom_right,
            top_right,
            block,
            face,
        }
    }

    pub fn u(&self) -> i32 {
        use OctantFace::*;
        match self.face {
            Left | Right => self.bottom_left.y,
            Up | Down => self.bottom_left.z,
            Front | Back => self.bottom_left.x,
        }
    }

    pub fn v(&self) -> i32 {
        use OctantFace::*;
        match self.face {
            Left | Right => self.bottom_left.z,
            Up | Down => self.bottom_left.x,
            Front | Back => self.bottom_left.y,
        }
    }

    pub fn width(&self) -> i32 {
        use OctantFace::*;
        match self.face {
            Left | Right => self.top_right.y - self.bottom_left.y,
            Up | Down => self.top_right.z - self.bottom_left.z,
            Front | Back => self.top_right.x - self.bottom_left.x,
        }
    }

    pub fn height(&self) -> i32 {
        use OctantFace::*;
        match self.face {
            Left | Right => self.top_right.z - self.bottom_left.z,
            Up | Down => self.top_right.x - self.bottom_left.x,
            Front | Back => self.top_right.y - self.bottom_left.y,
        }
    }

    pub fn normal_vector(&self) -> Vector3<f32> {
        use OctantFace::*;
        match self.face {
            Back => Vector3::new(0.0, 0.0, 1.0),
            Up => Vector3::new(0.0, 1.0, 0.0),
            Front => Vector3::new(0.0, 0.0, -1.0),
            Down => Vector3::new(0.0, -1.0, 0.0),
            Right => Vector3::new(1.0, 0.0, 0.0),
            Left => Vector3::new(-1.0, 0.0, 0.0),
        }
    }

    pub fn mesh_coords(self) -> Vec<PosNormTex> {
        //let p = base_point.borrow();
        //let base: Vector3<i32> = Vector3::new(p.x, p.y, p.z);

        let v: [Vector3<f32>; 4] = [
            convert(into_vec3(self.bottom_left)),
            convert(into_vec3(self.bottom_right)),
            convert(into_vec3(self.top_left)),
            convert(into_vec3(self.top_right)),
        ];

        let (width, height): (f32, f32) = (self.width().as_(), self.height().as_());
        let t = [
            Vector2::new(0.0, 0.0),
            Vector2::new(width, 0.0),
            Vector2::new(0.0, height),
            Vector2::new(width, height),
        ];
        let n = self.normal_vector();

        use OctantFace::*;
        let order = match self.face {
            Back => vec![0, 1, 2, 2, 1, 3],
            Up => vec![1, 3, 0, 0, 3, 2],
            Front => vec![2, 3, 0, 0, 3, 1],
            Down => vec![0, 2, 1, 1, 2, 3],
            Right => vec![2, 0, 3, 3, 0, 1],
            Left => vec![0, 2, 1, 1, 2, 3],
        };
        order
            .into_iter()
            .map(|i| pos_norm_tex(v[i], n, t[i]))
            .collect()
    }
}

fn pos_norm_tex(
    position: Vector3<f32>,
    normal: Vector3<f32>,
    tex_coord: Vector2<f32>,
) -> PosNormTex {
    PosNormTex {
        position,
        normal,
        tex_coord,
    }
}

impl fmt::Display for Quad {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Quad(\n\t{}, {},\n\t{}, {},\n\t{}, {:?})",
            self.top_left,
            self.top_right,
            self.bottom_left,
            self.bottom_right,
            self.block,
            self.face
        )
    }
}
impl fmt::Debug for Quad {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Quad({}, {}, {}, {}, {}, {:?})",
            self.bottom_left,
            self.top_left,
            self.bottom_right,
            self.top_right,
            self.block,
            self.face
        )
    }
}
impl PartialOrd for Quad {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.face != other.face || self.block != other.block {
            None
        } else {
            Some(self.cmp(other))
        }
    }
}
impl Ord for Quad {
    fn cmp(&self, other: &Self) -> Ordering {
        let (x, y, w, h) = (self.u(), self.v(), self.width(), self.height());
        let (_x, _y, _w, _h) = (other.u(), other.v(), other.width(), other.height());

        y.cmp(&_y)
            .then(x.cmp(&_x))
            .then(_w.cmp(&w))
            .then(_h.cmp(&h))
    }
}

fn into_vec3<N: Scalar>(point: Point3<N>) -> Vector3<N> {
    Vector3::new(point.x, point.y, point.z)
}
fn as_option(pred: bool) -> Option<()> {
    if pred {
        Some(())
    } else {
        None
    }
}

#[cfg(test)]
mod test {
    use super::{Block, Mesher, OctantFace, Point3, Quad};
    use crate::octree::Octree;
    use galvanic_assert::matchers::collection::*;

    //#[test]
    //fn full_octree_meshed_with_6_quads() {
    //    let height = 5;
    //    let octree: Octree<Block> = Octree::new(Point3::origin(), Some(1), height);
    //    let mesher = Mesher::new(&octree);
    //    let size = i32::pow(2, height);

    //    let quads = mesher.generate_quads_array();
    //    assert_that!(
    //        &quads,
    //        contains_in_any_order(vec![
    //            Quad::new(
    //                Point3::new(0, 0, 0),
    //                Point3::new(0, 0, size),
    //                Point3::new(0, size, 0),
    //                Point3::new(0, size, size),
    //                1,
    //                OctantFace::Left
    //            ),
    //            Quad::new(
    //                Point3::new(size, 0, 0),
    //                Point3::new(size, 0, size),
    //                Point3::new(size, size, 0),
    //                Point3::new(size, size, size),
    //                1,
    //                OctantFace::Right
    //            ),
    //            Quad::new(
    //                Point3::new(0, 0, 0),
    //                Point3::new(size, 0, 0),
    //                Point3::new(0, 0, size),
    //                Point3::new(size, 0, size),
    //                1,
    //                OctantFace::Down
    //            ),
    //            Quad::new(
    //                Point3::new(0, size, 0),
    //                Point3::new(size, size, 0),
    //                Point3::new(0, size, size),
    //                Point3::new(size, size, size),
    //                1,
    //                OctantFace::Up
    //            ),
    //            Quad::new(
    //                Point3::new(0, 0, 0),
    //                Point3::new(0, size, 0),
    //                Point3::new(size, 0, 0),
    //                Point3::new(size, size, 0),
    //                1,
    //                OctantFace::Front
    //            ),
    //            Quad::new(
    //                Point3::new(0, 0, size),
    //                Point3::new(0, size, size),
    //                Point3::new(size, 0, size),
    //                Point3::new(size, size, size),
    //                1,
    //                OctantFace::Back
    //            ),
    //        ])
    //    );
    //}

}
