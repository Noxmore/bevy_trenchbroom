use super::*;

/// As far as i know, this entity is used by the compiler as a hint of the inside vs outside of the map. It also complains if you don't add it.
#[point_class]
pub struct InfoPlayerStart;
