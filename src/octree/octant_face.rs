#[derive(Eq, PartialEq, Debug, Copy, Clone, FromPrimitive, ToPrimitive)]
pub enum OctantFace {
    Back = 0,
    Up = 1,
    Front = 2,
    Down = 3,
    Right = 4,
    Left = 5,
}
