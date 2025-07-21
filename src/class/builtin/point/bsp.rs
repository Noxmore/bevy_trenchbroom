use super::*;

/// Defines the single-player start point and direction.
///
/// (If this is not inside level geometry, `qbsp` reports warnings like
/// `WARNING: Reached occupant "info_player_start" at (1000 -296 -40), no filling performed.`
/// which fails the build just as if level geometry is not enclosed.)
#[point_class]
pub struct InfoPlayerStart;
