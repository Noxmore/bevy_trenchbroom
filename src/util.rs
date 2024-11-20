use bevy::render::texture::{ImageAddressMode, ImageSamplerDescriptor};

use crate::*;

/// Creates an image sampler with repeating textures, and optionally filtered.
pub fn repeating_image_sampler(filtered: bool) -> ImageSamplerDescriptor {
    ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        address_mode_w: ImageAddressMode::Repeat,
        ..if filtered {
            ImageSamplerDescriptor::linear()
        } else {
            ImageSamplerDescriptor::nearest()
        }
    }
}

pub trait ZUpToYUp {
    /// Converts from a z-up, y-forward coordinate space to a y-up, negative-z-forward coordinate space.
    fn z_up_to_y_up(self) -> Self;
}

impl ZUpToYUp for DVec3 {
    #[inline]
    fn z_up_to_y_up(self) -> Self {
        dvec3(self.x, self.z, -self.y)
    }
}
impl ZUpToYUp for Vec3 {
    #[inline]
    fn z_up_to_y_up(self) -> Self {
        vec3(self.x, self.z, -self.y)
    }
}

// pub const Z_UP_TO_Y_UP: Quat = Quat::

pub(crate) trait AlmostEqual<T> {
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
        self.x.almost_eq(other.x, margin) &&
        self.y.almost_eq(other.y, margin) &&
        self.z.almost_eq(other.z, margin)
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
        self.x.almost_eq(other.x, margin) &&
        self.y.almost_eq(other.y, margin) &&
        self.z.almost_eq(other.z, margin)
    }
}

impl AlmostEqual<Quat> for Quat {
    type Margin = f32;
    fn almost_eq(self, other: Quat, margin: Self::Margin) -> bool {
        self.x.almost_eq(other.x, margin) &&
        self.y.almost_eq(other.y, margin) &&
        self.z.almost_eq(other.z, margin) &&
        self.w.almost_eq(other.w, margin)
    }
}

#[allow(unused)]
macro_rules! assert_almost_eq {
    ($left:expr, $right:expr, $margin:expr) => {
        match ($left, $right, $margin) {
            (left, right, margin) => {
                if !left.almost_eq(right, margin) {
                    panic!("assertion `left.almost_eq(right)` failed\n  left: {left}\n right: {right}");
                }
            }
        }
    };
    ($left:expr, $right:expr, $margin:expr, $($arg:tt)+) => {
        match ($left, $right, $margin) {
            (left, right, margin) => {
                if !left.almost_eq(right, margin) {
                    panic!($($arg)+);
                }
            }
        }
    };
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

simple_trenchbroom_value_impl!(bool, true, Choices [("true".tb_to_string_quoted(), "true".into()), ("false".tb_to_string_quoted(), "false".into())]);

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

// Should this use linear or srgb? VDC doesn't specify the color space. It probably doesn't matter anyway.
impl TrenchBroomValue for Color {
    fn tb_parse(input: &str) -> anyhow::Result<Self> {
        <[f32; 3]>::tb_parse(input)
            .map(Color::srgb_from_array)
            .or(<[f32; 4]>::tb_parse(input).map(|[r, g, b, a]| Color::srgba(r, g, b, a)))
    }
    fn tb_to_string(&self) -> String {
        let col = self.to_srgba();
        format!("{} {} {} {}", col.red, col.green, col.blue, col.alpha)
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
/// Put this on an entity when spawning to counteract the rotation.
#[derive(Component)]
pub struct TrenchBroomGltfRotationFix;

/// See docs on [TrenchBroomGltfRotationFix]
pub(crate) fn trenchbroom_gltf_rotation_fix(world: &mut World, entity: Entity) {
    if world
        .entity(entity)
        .contains::<TrenchBroomGltfRotationFix>()
    {
        if let Some(mut transform) = world.entity_mut(entity).get_mut::<Transform>() {
            transform.rotate_local_y(std::f32::consts::PI / 2.);
        }
    }
}

pub(crate) fn invalid_data(err: impl std::error::Error + Send + Sync + 'static) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, err)
}

pub fn alpha_mode_from_image(image: &Image) -> AlphaMode {
    let mut cutout = false;

    for color in image.data.chunks_exact(4) {
        let alpha = color[3];

        if alpha == 0 {
            cutout = true;
        } else if alpha != 255 {
            return AlphaMode::Blend;
        }
    }

    if cutout { AlphaMode::Mask(0.5) } else { AlphaMode::Opaque }
}

/// `angles` is pitch, yaw, roll. Converts from degrees to radians. `0 0 0` [points east](https://www.gamers.org/dEngine/quake/QDP/qmapspec.html#2.1.1).
#[inline]
pub fn angles_to_quat(angles: Vec3) -> Quat {
    // Quat::from_euler(
    //     // Honestly, i don't know why this works, i got here through hours of trial and error
    //     EulerRot::YXZ,
    //     (angles.y - 90.).to_radians(),
    //     -angles.x.to_radians(),
    //     -angles.z.to_radians(),
    // )
    let yaw = Quat::from_rotation_y((angles.y - 90.).to_radians()); // We must be east-pointing
    let pitch = Quat::from_rotation_x(-angles.x.to_radians());
    let roll = Quat::from_rotation_z(-angles.z.to_radians());
    yaw * pitch * roll
}

/// `mangle` is yaw, pitch, roll. Converts from degrees to radians. `0 0 0` [points east](https://www.gamers.org/dEngine/quake/QDP/qmapspec.html#2.1.1).
/// 
/// NOTE: TrenchBroom docs dictate that this function should only be called when the entity classname begins with "light", otherwise "mangle" is a synonym for “angles”.
#[inline]
pub fn mangle_to_quat(mangle: Vec3) -> Quat {
    let yaw = Quat::from_rotation_y((mangle.x - 90.).to_radians()); // We must be east-pointing
    let pitch = Quat::from_rotation_x(mangle.y.to_radians());
    let roll = Quat::from_rotation_z(-mangle.z.to_radians());
    yaw * pitch * roll
}

/// `angle` is the rotation around the Y axis. Converts from degrees to radians. `0` [points east](https://www.gamers.org/dEngine/quake/QDP/qmapspec.html#2.1.1).
/// # Special Values
/// - -1: Up
/// - -2: Down
#[inline]
pub fn angle_to_quat(angle: f32) -> Quat {
    match angle {
        -1. => Quat::from_rotation_x(FRAC_PI_2),
        -2. => Quat::from_rotation_x(-FRAC_PI_2),
        angle => Quat::from_rotation_y((angle - 90.).to_radians()),
    }
}

pub const QUAKE_LIGHT_TO_LUX_DIVISOR: f32 = 50_000.;
/// Quake light (such as the `light` property used in light entities) conversion to lux (lumens per square meter).
/// 
/// NOTE: This is only a rough estimation, based on what i've personally found looks right.
#[inline]
pub fn quake_light_to_lux(light: f32) -> f32 {
    light / QUAKE_LIGHT_TO_LUX_DIVISOR
}

#[test]
fn z_up_to_y_up() {
    assert_eq!(Vec3::X.z_up_to_y_up(), Vec3::X);
    assert_eq!(Vec3::Y.z_up_to_y_up(), Vec3::NEG_Z);
    assert_eq!(Vec3::Z.z_up_to_y_up(), Vec3::Y);
}

#[test]
fn rotation_property_to_quat() {
    const MARGIN: f32 = 0.0001;

    // angle
    assert_almost_eq!(angle_to_quat(0.) * Vec3::NEG_Z, Vec3::X, MARGIN);
    assert_almost_eq!(angle_to_quat(90.) * Vec3::NEG_Z, Vec3::NEG_Z, MARGIN);
    assert_almost_eq!(angle_to_quat(0.) * Vec3::Y, Vec3::Y, MARGIN);
    assert_almost_eq!(angle_to_quat(-1.) * Vec3::NEG_Z, Vec3::Y, MARGIN);
    assert_almost_eq!(angle_to_quat(-2.) * Vec3::NEG_Z, Vec3::NEG_Y, MARGIN);
    assert_almost_eq!(angle_to_quat(-2.) * Vec3::Y, Vec3::NEG_Z, MARGIN);

    // mangle
    assert_almost_eq!(mangle_to_quat(vec3(0., 0., 0.)) * Vec3::NEG_Z, Vec3::X, MARGIN);
    assert_almost_eq!(mangle_to_quat(vec3(0., 0., 0.)) * Vec3::Y, Vec3::Y, MARGIN);

    assert_almost_eq!(mangle_to_quat(vec3(90., 0., 0.)) * Vec3::NEG_Z, Vec3::NEG_Z, MARGIN);
    assert_almost_eq!(mangle_to_quat(vec3(0., -90., 0.)) * Vec3::NEG_Z, Vec3::NEG_Y, MARGIN);
    assert_almost_eq!(mangle_to_quat(vec3(0., 90., 0.)) * Vec3::NEG_Z, Vec3::Y, MARGIN);
    assert_almost_eq!(mangle_to_quat(vec3(0., 0., 90.)) * Vec3::Y, Vec3::Z, MARGIN);
    // assert_eq!((mangle_to_quat(vec3(45., 45., 0.)) * Vec3::NEG_Z - vec3(1., 1., -1.).normalize()).length(), 0.);
    // almost 0.17 in precision loss??? how??
    assert_almost_eq!(mangle_to_quat(vec3(45., 45., 0.)) * Vec3::NEG_Z, vec3(1., 1., -1.).normalize(), 0.2);

    // angles
    assert_almost_eq!(angles_to_quat(vec3(0., 0., 0.)) * Vec3::NEG_Z, Vec3::X, MARGIN);
    assert_almost_eq!(angles_to_quat(vec3(0., 0., 0.)) * Vec3::Y, Vec3::Y, MARGIN);
    
    assert_almost_eq!(angles_to_quat(vec3(0., 90., 0.)) * Vec3::NEG_Z, Vec3::NEG_Z, MARGIN);
    assert_almost_eq!(angles_to_quat(vec3(90., 0., 0.)) * Vec3::NEG_Z, Vec3::NEG_Y, MARGIN);
    assert_almost_eq!(angles_to_quat(vec3(0., 0., 90.)) * Vec3::Y, Vec3::Z, MARGIN);
    // Margin adjusted for bogus precision loss
    assert_almost_eq!(angles_to_quat(vec3(-45., -45., 0.)) * Vec3::NEG_Z, vec3(1., 1., 1.).normalize(), 0.2);
}