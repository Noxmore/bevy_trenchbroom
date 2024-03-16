use crate::*;

pub trait TrenchBroomToBevySpace {
    /// Converts from a z-up coordinate space to a y-up coordinate space.
    fn z_up_to_y_up(self) -> Self;
    /// Converts from a z-up coordinate space to a y-up coordinate space, and scales everything down by the current [TrenchBroomConfig]'s scale.
    fn trenchbroom_to_bevy_space(self) -> Self;
}

impl TrenchBroomToBevySpace for DVec3 {
    #[inline]
    fn z_up_to_y_up(self) -> Self {
        dvec3(self.x, self.z, -self.y)
    }
    #[inline]
    fn trenchbroom_to_bevy_space(self) -> Self {
        self.z_up_to_y_up() / *TRENCHBROOM_SCALE.read().unwrap() as f64
    }
}
impl TrenchBroomToBevySpace for Vec3 {
    #[inline]
    fn z_up_to_y_up(self) -> Self {
        vec3(self.x, self.z, -self.y)
    }
    #[inline]
    fn trenchbroom_to_bevy_space(self) -> Self {
        self.z_up_to_y_up() / *TRENCHBROOM_SCALE.read().unwrap()
    }
}

pub trait AlmostEqual<T> {
    type Margin;
    fn almost_eq(self, other: T, margin: Self::Margin) -> bool;
}

impl AlmostEqual<f32> for f32 {
    type Margin = f32;
    fn almost_eq(self, other: f32, margin: Self::Margin) -> bool {
        (other - self).abs() < margin
    }
}
impl AlmostEqual<Vec3> for Vec3 {
    type Margin = f32;
    fn almost_eq(self, other: Vec3, margin: Self::Margin) -> bool {
        self.x.almost_eq(other.x, margin)
            && self.y.almost_eq(other.y, margin)
            && self.z.almost_eq(other.z, margin)
    }
}

impl AlmostEqual<f64> for f64 {
    type Margin = f64;
    fn almost_eq(self, other: f64, margin: Self::Margin) -> bool {
        (other - self).abs() < margin
    }
}
impl AlmostEqual<DVec3> for DVec3 {
    type Margin = f64;
    fn almost_eq(self, other: DVec3, margin: Self::Margin) -> bool {
        self.x.almost_eq(other.x, margin)
            && self.y.almost_eq(other.y, margin)
            && self.z.almost_eq(other.z, margin)
    }
}

pub trait ConvertZeroToOne {
    /// If this equals to zero, return it where it is one, created for use with division.
    fn convert_zero_to_one(self) -> Self;
}

impl ConvertZeroToOne for f32 {
    fn convert_zero_to_one(self) -> Self {
        if self == 0. {
            1.
        } else {
            self
        }
    }
}

impl ConvertZeroToOne for Vec2 {
    fn convert_zero_to_one(self) -> Self {
        vec2(self.x.convert_zero_to_one(), self.y.convert_zero_to_one())
    }
}

/// Contains TrenchBroom-specific parsing and stringification functions.
pub trait TrenchBroomValue: Sized {
    /// If quotes should be put around this value when writing out an `fgd` file.
    const TB_IS_QUOTED: bool = true;

    /// Parses a string into `Self` TrenchBroom-style, used for parsing entity properties.
    fn tb_parse(input: &str) -> anyhow::Result<Self>;
    /// Converts this value into a string TrenchBroom-style, used for writing `fgd`s.
    fn tb_to_string(&self) -> String;
    /// Calls `tb_to_string`, but if `TB_IS_QUOTED` is true, surrounds the output with quotes.
    fn tb_to_string_quoted(&self) -> String {
        if Self::TB_IS_QUOTED {
            format!("\"{}\"", self.tb_to_string())
        } else {
            self.tb_to_string()
        }
    }

    fn fgd_type() -> EntDefPropertyType;
}

impl TrenchBroomValue for String {
    fn tb_parse(input: &str) -> anyhow::Result<Self> {
        Ok(input.to_string())
    }
    fn tb_to_string(&self) -> String {
        self.clone()
    }
    fn fgd_type() -> EntDefPropertyType {
        EntDefPropertyType::Value("string".into())
    }
}

impl TrenchBroomValue for &str {
    fn tb_parse(_input: &str) -> anyhow::Result<Self> {
        // Lifetimes don't allow me to just return Some(input) unfortunately.
        unimplemented!("use String::tb_parse instead");
    }
    fn tb_to_string(&self) -> String {
        self.to_string()
    }
    fn fgd_type() -> EntDefPropertyType {
        EntDefPropertyType::Value("string".into())
    }
}

macro_rules! simple_trenchbroom_value_impl {
    ($ty:ty, $quoted:expr, $fgd_type:ident $fgd_type_value:expr) => {
        impl TrenchBroomValue for $ty {
            const TB_IS_QUOTED: bool = $quoted;

            fn tb_parse(input: &str) -> anyhow::Result<Self> {
                Ok(input.parse()?)
            }
            fn tb_to_string(&self) -> String {
                self.to_string()
            }
            fn fgd_type() -> EntDefPropertyType {
                EntDefPropertyType::$fgd_type($fgd_type_value.into())
            }
        }
    };
}

simple_trenchbroom_value_impl!(u8, false, Value "integer");
simple_trenchbroom_value_impl!(u16, false, Value "integer");
simple_trenchbroom_value_impl!(u32, false, Value "integer");
simple_trenchbroom_value_impl!(u64, false, Value "integer");
simple_trenchbroom_value_impl!(usize, false, Value "integer");
simple_trenchbroom_value_impl!(i8, false, Value "integer");
simple_trenchbroom_value_impl!(i16, false, Value "integer");
simple_trenchbroom_value_impl!(i32, false, Value "integer");
simple_trenchbroom_value_impl!(i64, false, Value "integer");
simple_trenchbroom_value_impl!(isize, false, Value "integer");

simple_trenchbroom_value_impl!(bool, true, Choices [("true".into(), "true".into()), ("false".into(), "false".into())]);

simple_trenchbroom_value_impl!(f32, true, Value "float");
simple_trenchbroom_value_impl!(f64, true, Value "float");

impl TrenchBroomValue for Aabb {
    const TB_IS_QUOTED: bool = false;

    fn tb_parse(input: &str) -> anyhow::Result<Self> {
        let values = <[f32; 6]>::tb_parse(input)?;
        Ok(Aabb::from_min_max(
            Vec3::from_slice(&values[0..=3]),
            Vec3::from_slice(&values[3..=6]),
        ))
    }
    fn tb_to_string(&self) -> String {
        let min = self.min();
        let max = self.max();
        format!(
            "{} {} {}, {} {} {}",
            min.x, min.y, min.z, max.x, max.y, max.z
        )
    }
    fn fgd_type() -> EntDefPropertyType {
        EntDefPropertyType::Value("aabb".into())
    }
}

impl TrenchBroomValue for Vec4 {
    fn tb_parse(input: &str) -> anyhow::Result<Self> {
        <[f32; 4]>::tb_parse(input).map(Vec4::from)
    }
    fn tb_to_string(&self) -> String {
        format!("{} {} {} {}", self.x, self.y, self.z, self.w)
    }
    fn fgd_type() -> EntDefPropertyType {
        EntDefPropertyType::Value("vec4".into())
    }
}
impl TrenchBroomValue for Vec3 {
    fn tb_parse(input: &str) -> anyhow::Result<Self> {
        <[f32; 3]>::tb_parse(input).map(Vec3::from)
    }
    fn tb_to_string(&self) -> String {
        format!("{} {} {}", self.x, self.y, self.z)
    }
    fn fgd_type() -> EntDefPropertyType {
        EntDefPropertyType::Value("vector".into())
    }
}
impl TrenchBroomValue for Vec2 {
    fn tb_parse(input: &str) -> anyhow::Result<Self> {
        <[f32; 2]>::tb_parse(input).map(Vec2::from)
    }
    fn tb_to_string(&self) -> String {
        format!("{} {}", self.x, self.y)
    }
    fn fgd_type() -> EntDefPropertyType {
        EntDefPropertyType::Value("vec2".into())
    }
}

impl TrenchBroomValue for Color {
    fn tb_parse(input: &str) -> anyhow::Result<Self> {
        <[f32; 3]>::tb_parse(input)
            .map(Color::rgb_from_array)
            .or(<[f32; 4]>::tb_parse(input).map(Color::rgba_from_array))
    }
    fn tb_to_string(&self) -> String {
        format!("{} {} {} {}", self.r(), self.g(), self.b(), self.a())
    }
    fn fgd_type() -> EntDefPropertyType {
        EntDefPropertyType::Value("color1".into())
    }
}

// God i love rust's trait system
impl<T: TrenchBroomValue + Default + Copy, const COUNT: usize> TrenchBroomValue for [T; COUNT] {
    // const TB_IS_QUOTED: bool = T::TB_IS_QUOTED;

    fn tb_parse(input: &str) -> anyhow::Result<Self> {
        // This might be a problem for TrenchBroomValues that use spaces in their parsing. Oh well!
        let mut out = [T::default(); COUNT];

        for (i, input) in input.split_ascii_whitespace().enumerate() {
            if i >= out.len() {
                return Err(anyhow::anyhow!("Too many elements! Expected: {COUNT}"));
            }
            out[i] = T::tb_parse(input)?;
        }

        Ok(out)
    }
    fn tb_to_string(&self) -> String {
        self.iter().map(T::tb_to_string).join(" ")
    }
    fn fgd_type() -> EntDefPropertyType {
        T::fgd_type()
    }
}

/// Band-aid fix for a [TrenchBroom bug](https://github.com/TrenchBroom/TrenchBroom/issues/4447) where GLTF models are rotated be 90 degrees on the Y axis.
///
/// Put this on an entity when inserting to counteract the rotation.
#[derive(Component)]
pub struct TrenchBroomGltfRotationFix;

/// See docs on [TrenchBroomGltfRotationFix]
pub(crate) fn trenchbroom_gltf_rotation_fix(commands: &mut Commands, entity: Entity) {
    commands.entity(entity).add(|mut ent: EntityWorldMut| {
        if ent.contains::<TrenchBroomGltfRotationFix>() {
            if let Some(mut transform) = ent.get_mut::<Transform>() {
                transform.rotate_local_y(std::f32::consts::PI / 2.);
            }
        }
    });
}
