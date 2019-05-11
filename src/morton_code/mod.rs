use amethyst::core::nalgebra::{Point3, Scalar};
use num_traits::AsPrimitive;
use std::{fmt, iter::Iterator};

mod lut;
pub use lut::LUTType;
use lut::*;

pub mod convs;

#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Copy, Debug, Hash)]
pub struct MortonCode<N: MortonStorage> {
    data: N::Storage,
    marker: std::marker::PhantomData<N>,
}

impl<N: MortonStorage> fmt::Display for MortonCode<N>
where
    N::Storage: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.data)
    }
}
const fn decode_triplets<T>() -> usize {
    let bits = std::mem::size_of::<T>() * 8;
    return (bits + 3) / 3;
}

impl<N> MortonCode<N>
where
    N: IntoBytes + MortonStorage + Scalar + Zero + BitOr<Output = N>,
    LUTType: AsPrimitive<N> + AsPrimitive<N::Storage>,
{
    pub fn new(x: N, y: N, z: N) -> Self {
        MortonCode {
            data: MortonCode::encode(x, y, z),
            marker: std::marker::PhantomData,
        }
    }
    pub fn from_raw(raw: N::Storage) -> Self {
        MortonCode {
            data: raw,
            marker: std::marker::PhantomData,
        }
    }

    fn encode(x: N, y: N, z: N) -> N::Storage {
        let xs = x.into_bytes().into_iter();
        let ys = y.into_bytes().into_iter();
        let zs = z.into_bytes().into_iter();
        let mut answer: N::Storage = N::Storage::zero();
        for ((xindx, yindx), zindx) in xs.zip(ys).zip(zs).rev() {
            let x = MORTON_ENCODE_X[xindx as usize];
            let y = MORTON_ENCODE_Y[yindx as usize];
            let z = MORTON_ENCODE_Z[zindx as usize];
            answer = (answer << 24) | x.as_() | y.as_() | z.as_();
        }
        return answer;
    }

    pub fn decode(&self) -> (N, N, N) {
        let morton = self.data;
        let (mut x, mut y, mut z): (N, N, N) = (N::zero(), N::zero(), N::zero());
        for i in 0..decode_triplets::<N>() {
            let base_shift = i * 9;
            x = x
                | (MORTON_DECODE_X[((morton >> base_shift) & N::NINE_BIT_MASK).as_()] << (3 * i))
                    .as_();
            y = y
                | (MORTON_DECODE_Y[((morton >> base_shift) & N::NINE_BIT_MASK).as_()] << (3 * i))
                    .as_();
            z = z
                | (MORTON_DECODE_Z[((morton >> base_shift) & N::NINE_BIT_MASK).as_()] << (3 * i))
                    .as_();
        }
        (x, y, z)
    }

    pub fn as_point(&self) -> Point3<N> {
        let (x, y, z) = self.decode();
        Point3::new(x, y, z)
    }
}
use num_traits::{WrappingShl, WrappingShr, Zero};
use std::ops::{BitAnd, BitOr};
pub trait MortonStorage {
    /// Type that can contain an encoded MortonCode value from 3 elements of Self.
    /// For example this will be u32 for u8, since 3 * 8 = 24 bits and the smallest type that can hold that is u32.
    type Storage: Zero
        + WrappingShl
        + WrappingShr
        + BitOr<Output = Self::Storage>
        + BitAnd<Output = Self::Storage>
        + AsPrimitive<usize>;

    const NINE_BIT_MASK: Self::Storage;
}

macro_rules! impl_morton_storage_for {
    ($( [ $($num:ty),+ ] -> $storage:ty );*) => ($( $(
        impl MortonStorage for $num {
            type Storage = $storage;

            const NINE_BIT_MASK: $storage = 0x1FF;
        }
        )* )*
    )
}

impl_morton_storage_for! {
    [i32, u32] -> u128;
    [i16, u16] -> u64;
    [i8, u8] -> u32
}

pub trait IntoBytes {
    fn into_bytes(self) -> Vec<u8>;
}

macro_rules! impl_as_bytes_for {
    ($($num:ty),*) => {$(
        impl IntoBytes for $num {
            fn into_bytes(self) -> Vec<u8> {
                self.to_ne_bytes().to_vec()
            }
        }
    )*};
}

impl_as_bytes_for!(i8, i16, i32, i64, i128, u8, u16, u32, u64, u128);

#[cfg(test)]
mod test {
    use super::*;
    fn split_by_n(input: u128, bits_remaining: usize) -> u128 {
        if bits_remaining == 0 {
            input
        } else {
            (split_by_n(input >> 1, bits_remaining - 1) << 3) | (input & 1)
        }
    }

    fn control_encode_impl(fields: &[i32]) -> u128 {
        if fields.len() == 1 {
            split_by_n(fields[0] as u128, 32)
        } else {
            let (first, tail) = fields
                .split_first()
                .expect("Expected non empty slice for control_encode_impl");
            (control_encode_impl(tail) << 1) | split_by_n(*first as u128, 32)
        }
    }

    fn join_by_n(input: u128, bits_remaining: usize) -> u128 {
        if bits_remaining == 0 {
            0
        } else {
            (join_by_n(input >> 3, bits_remaining - 1) << 1) | (input & 1)
        }
    }

    fn control_decode_impl(encoded: u128) -> (i32, i32, i32) {
        let z = join_by_n(encoded, 32) as i32;
        let y = join_by_n(encoded >> 1, 32) as i32;
        let x = join_by_n(encoded >> 2, 32) as i32;
        (x, y, z)
    }

    #[test]
    fn test_morton_i32_encoding_against_control() {
        let field_bits = 32 / 3;
        let monstrosity = (0..=(field_bits - 4)).flat_map(|offset| {
            (0..16).flat_map(move |i| {
                (0..16).flat_map(move |j| (0..16).map(move |k| (offset, i, j, k)))
            })
        });
        for (offset, i, j, k) in monstrosity {
            let x: i32 = i << offset;
            let y: i32 = j << offset;
            let z: i32 = k << offset;

            let expected = control_encode_impl(&[z, y, x]);
            let result = MortonCode::new(x, y, z).data;

            assert_eq!(
                result, expected,
                "Wrong result for ({}, {}, {}) expected: {:?} but got: {:?}",
                x, y, z, expected, result
            );
        }
    }

    #[test]
    fn test_morton_i32_decoding_against_control() {
        let encoded_bits = (64 / 3) * 3;
        let product = (0..=(encoded_bits - 12))
            .flat_map(|offset| (0..4096).map(move |morton| (offset, morton)));
        for (offset, morton) in product {
            let encoded = morton << offset;
            let expected = control_decode_impl(encoded);
            let result = MortonCode::from_raw(encoded).decode();
            assert_eq!(
                expected, result,
                "Incorrect decoding for Morton value {}",
                encoded
            );
        }
    }
}
