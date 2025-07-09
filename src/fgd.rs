use bevy_reflect::{DynamicEnum, DynamicVariant, Enum, GetTypeRegistration, TypeRegistry};
use class::{ChoicesKey, QuakeClassPropertyType};
use enumflags2::{BitFlag, BitFlags};

use crate::*;

pub struct FgdPlugin;
impl Plugin for FgdPlugin {
	fn build(&self, app: &mut App) {
		#[rustfmt::skip]
		app
			.register_type::<IntBool>()
			.register_type::<Srgb>()
		;
	}
}

/// Writes classes in a type registry to a string for writing `fgd` (entity definition) files.
pub fn write_fgd(type_registry: &TypeRegistry) -> String {
	let classes = type_registry
		.iter_with_data::<ReflectQuakeClass>()
		.filter(|(_, class)| class.enabled)
		.map(|(_, class)| class.erased_class)
		.sorted_by(|a, b| a.info.name.cmp(b.info.name))
		.collect_vec();

	use fmt::Write;
	let mut s = String::new();
	macro_rules! write {($($arg:tt)*) => {
		s.write_fmt(format_args!($($arg)*)).ok()
	};}

	'class_loop: for class in &classes {
		// If this is a base class, and nothing depends on it, we shouldn't write it.
		// This checks names instead of references because i'm still not 100% sure const static refs are stable.
		if class.info.ty.is_base()
			&& classes
				.iter()
				.all(|checking_class| !checking_class.info.base.iter().any(|base| base.id() == class.id()))
		{
			continue;
		}

		// Validate that all inherited classes are registered.
		for base in class.info.base {
			if !type_registry.contains(base.id()) {
				error!("`{}`'s base class `{}` isn't registered, skipping", class.info.name, base.info.name);
				continue 'class_loop;
			}
		}

		write!("@{}Class ", class.info.ty);

		if !class.info.base.is_empty() {
			write!("base({}) ", class.info.base.iter().map(|base| base.info.name).join(", "));
		}

		if let Some(value) = class.info.color {
			write!("color({value}) ");
		}
		if let Some(value) = class.info.iconsprite {
			write!("iconsprite({value}) ");
		}
		if let Some(value) = class.info.size {
			write!("size({value}) ");
		}
		if let Some(value) = class.info.model {
			write!("model({value}) ");
		}

		write!("= {}", class.info.name);
		if let Some(description) = class.info.description {
			write!(" : \"{description}\"");
		}
		write!("\n[\n");

		for property in class.info.properties {
			if let QuakeClassPropertyType::Flags(new_flags_iter) = property.ty {
				let flags_iter = new_flags_iter();

				write!("\t{}(flags) =\n\t[\n", property.name);
				let default = property.default_value.and_then(|f| u32::fgd_parse(&f()).ok()).unwrap_or(0);

				for (i, (value, title)) in flags_iter.enumerate() {
					write!("\t\t{value} : \"{title}\" : {}\n", (default >> i) & 1);
				}

				write!("\t]\n");
				continue;
			}

			write!(
				"\t{}({}) : \"{}\" : {} : \"{}\"",
				property.name,
				match property.ty {
					QuakeClassPropertyType::Value(ty) => ty,
					QuakeClassPropertyType::Choices(_) => "choices",
					QuakeClassPropertyType::Flags(_) => unreachable!(),
				},
				property.title.unwrap_or(property.name),
				property.default_value.unwrap_or(String::new)(),
				property.description.unwrap_or_default(),
			);

			if let QuakeClassPropertyType::Choices(choices) = property.ty {
				write!(" = \n\t[\n");
				for (key, title) in choices {
					write!("\t\t{key} : \"{title}\"\n");
				}
				write!("\t]");
			}

			write!("\n");
		}

		write!("]\n\n");
	}

	s
}

/// Contains Quake/TrenchBroom-specific parsing and stringification functions.
pub trait FgdType: Sized {
	/// If quotes should be put around this value when writing out an FGD file.
	const FGD_IS_QUOTED: bool = true;
	const PROPERTY_TYPE: QuakeClassPropertyType;

	/// Parses a string into `Self` FGD-style. Used for parsing entity properties.
	fn fgd_parse(input: &str) -> anyhow::Result<Self>;
	/// Converts this value into a string used for writing FGDs.
	fn fgd_to_string_unquoted(&self) -> String;
	/// Calls `fgd_to_string_unquoted`, but if `FGD_IS_QUOTED` is true, surrounds the output with quotes.
	fn fgd_to_string(&self) -> String {
		if Self::FGD_IS_QUOTED {
			format!("\"{}\"", self.fgd_to_string_unquoted())
		} else {
			self.fgd_to_string_unquoted()
		}
	}
}

impl FgdType for String {
	const PROPERTY_TYPE: QuakeClassPropertyType = QuakeClassPropertyType::Value("string");

	fn fgd_parse(input: &str) -> anyhow::Result<Self> {
		Ok(input.to_string())
	}
	fn fgd_to_string_unquoted(&self) -> String {
		self.clone()
	}
}

impl FgdType for &str {
	const PROPERTY_TYPE: QuakeClassPropertyType = String::PROPERTY_TYPE;

	fn fgd_parse(_input: &str) -> anyhow::Result<Self> {
		// Lifetimes don't allow me to just return Some(input) unfortunately.
		unimplemented!("use String::fgd_parse instead");
	}
	fn fgd_to_string_unquoted(&self) -> String {
		self.to_string()
	}
}

macro_rules! simple_fgd_type_impl {
	($ty:ty, $quoted:expr, $fgd_type:ident $fgd_type_value:expr) => {
		impl FgdType for $ty {
			const FGD_IS_QUOTED: bool = $quoted;
			const PROPERTY_TYPE: QuakeClassPropertyType = QuakeClassPropertyType::$fgd_type($fgd_type_value);

			fn fgd_parse(input: &str) -> anyhow::Result<Self> {
				Ok(input.trim().parse()?)
			}
			fn fgd_to_string_unquoted(&self) -> String {
				self.to_string()
			}
		}
	};
}

simple_fgd_type_impl!(u8, false, Value "integer");
simple_fgd_type_impl!(u16, false, Value "integer");
simple_fgd_type_impl!(u32, false, Value "integer");
simple_fgd_type_impl!(u64, false, Value "integer");
simple_fgd_type_impl!(usize, false, Value "integer");
simple_fgd_type_impl!(i8, false, Value "integer");
simple_fgd_type_impl!(i16, false, Value "integer");
simple_fgd_type_impl!(i32, false, Value "integer");
simple_fgd_type_impl!(i64, false, Value "integer");
simple_fgd_type_impl!(isize, false, Value "integer");

#[rustfmt::skip]
simple_fgd_type_impl!(bool, true, Choices & [(ChoicesKey::String("true"), "true"), (ChoicesKey::String("false"), "false")]);

macro_rules! simple_float_fgd_type_impl {
	($ty:ty, $quoted:expr, $fgd_type:ident $fgd_type_value:expr) => {
		impl FgdType for $ty {
			const FGD_IS_QUOTED: bool = $quoted;
			const PROPERTY_TYPE: QuakeClassPropertyType = QuakeClassPropertyType::$fgd_type($fgd_type_value);

			fn fgd_parse(input: &str) -> anyhow::Result<Self> {
				// Some BSP properties include European localized strings (e.g. "1,50").
				// Rather than bring in a crate for the special case, convert here.
				if input.contains(',') {
					Ok(input
						.chars()
						.filter(|&c| c != '.' && c != ' ')
						.map(|c| if c == ',' { '.' } else { c })
						.collect::<String>()
						.parse()?)
				} else {
					Ok(input.trim().parse()?)
				}
			}
			fn fgd_to_string_unquoted(&self) -> String {
				self.to_string()
			}
		}
	};
}

simple_float_fgd_type_impl!(f32, true, Value "float");
simple_float_fgd_type_impl!(f64, true, Value "float");

/// [`FgdType`] Wrapper for a `bool` that expects integers rather than boolean strings. Non-zero is `true`, zero is `false`.
#[derive(Reflect, Debug, Clone, Copy, Default, PartialEq, Eq, Deref, DerefMut, Serialize, Deserialize)]
pub struct IntBool(pub bool);
impl FgdType for IntBool {
	const FGD_IS_QUOTED: bool = false;
	const PROPERTY_TYPE: QuakeClassPropertyType = QuakeClassPropertyType::Value("integer");

	fn fgd_parse(input: &str) -> anyhow::Result<Self> {
		i64::fgd_parse(input).map(|v| Self(v > 0))
	}

	fn fgd_to_string_unquoted(&self) -> String {
		if self.0 { "1".s() } else { "0".s() }
	}
}

/// Mainly for BSP compiler properties, 1 translates to [`Enable`](IntBoolOverride::Enable), 0 to [`Inherit`](IntBoolOverride::Inherit), and -1 to [`Disable`](IntBoolOverride::Disable).
#[derive(Reflect, Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum IntBoolOverride {
	Enable,
	#[default]
	Inherit,
	Disable,
}
impl FgdType for IntBoolOverride {
	const FGD_IS_QUOTED: bool = false;
	const PROPERTY_TYPE: QuakeClassPropertyType = QuakeClassPropertyType::Value("integer");

	fn fgd_parse(input: &str) -> anyhow::Result<Self> {
		i64::fgd_parse(input).map(|v| match v {
			..=-1 => Self::Disable,
			0 => Self::Inherit,
			1.. => Self::Enable,
		})
	}
	fn fgd_to_string_unquoted(&self) -> String {
		match self {
			Self::Enable => "1".s(),
			Self::Inherit => "0".s(),
			Self::Disable => "-1".s(),
		}
	}
}

impl FgdType for Aabb {
	const FGD_IS_QUOTED: bool = false;
	const PROPERTY_TYPE: QuakeClassPropertyType = QuakeClassPropertyType::Value("aabb");

	fn fgd_parse(input: &str) -> anyhow::Result<Self> {
		let values = <[f32; 6]>::fgd_parse(input)?;
		Ok(Aabb::from_min_max(Vec3::from_slice(&values[0..3]), Vec3::from_slice(&values[3..6])))
	}
	fn fgd_to_string_unquoted(&self) -> String {
		let min = self.min();
		let max = self.max();
		format!("{} {} {}, {} {} {}", min.x, min.y, min.z, max.x, max.y, max.z)
	}
}

impl FgdType for Vec4 {
	const PROPERTY_TYPE: QuakeClassPropertyType = QuakeClassPropertyType::Value("vec4");

	fn fgd_parse(input: &str) -> anyhow::Result<Self> {
		<[f32; 4]>::fgd_parse(input).map(Vec4::from)
	}
	fn fgd_to_string_unquoted(&self) -> String {
		format!("{} {} {} {}", self.x, self.y, self.z, self.w)
	}
}
impl FgdType for Vec3 {
	const PROPERTY_TYPE: QuakeClassPropertyType = QuakeClassPropertyType::Value("vector");

	fn fgd_parse(input: &str) -> anyhow::Result<Self> {
		<[f32; 3]>::fgd_parse(input).map(Vec3::from)
	}
	fn fgd_to_string_unquoted(&self) -> String {
		format!("{} {} {}", self.x, self.y, self.z)
	}
}
impl FgdType for Vec2 {
	const PROPERTY_TYPE: QuakeClassPropertyType = QuakeClassPropertyType::Value("vec2");

	fn fgd_parse(input: &str) -> anyhow::Result<Self> {
		<[f32; 2]>::fgd_parse(input).map(Vec2::from)
	}
	fn fgd_to_string_unquoted(&self) -> String {
		format!("{} {}", self.x, self.y)
	}
}

impl FgdType for Color {
	const PROPERTY_TYPE: QuakeClassPropertyType = QuakeClassPropertyType::Value("color1");

	fn fgd_parse(input: &str) -> anyhow::Result<Self> {
		Srgba::fgd_parse(input).map(Self::Srgba)
	}
	fn fgd_to_string_unquoted(&self) -> String {
		self.to_srgba().fgd_to_string_unquoted()
	}
}

fn truncate_byte_color_range<const N: usize>(color: [f32; N]) -> [f32; N] {
	if color.into_iter().any(|channel| channel > 1.) {
		color.map(|channel| channel / 255.)
	} else {
		color
	}
}

impl FgdType for Srgba {
	const PROPERTY_TYPE: QuakeClassPropertyType = QuakeClassPropertyType::Value("color1");

	fn fgd_parse(input: &str) -> anyhow::Result<Self> {
		Srgb::fgd_parse(input)
			.map(Into::into)
			.or(<[f32; 4]>::fgd_parse(input).map(truncate_byte_color_range).map(Self::from_f32_array))
	}
	fn fgd_to_string_unquoted(&self) -> String {
		format!("{} {} {} {}", self.red, self.green, self.blue, self.alpha)
	}
}

/// Like [`Srgba`], but without the alpha channel. Implements [`FgdType`], for use in quake classes.
#[derive(Reflect, Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct Srgb {
	pub red: f32,
	pub green: f32,
	pub blue: f32,
}
impl Srgb {
	pub const WHITE: Self = Self::new(1., 1., 1.);
	pub const BLACK: Self = Self::new(0., 0., 0.);
	pub const WHITE_255: Self = Self::new(255., 255., 255.);

	pub const fn new(red: f32, green: f32, blue: f32) -> Self {
		Self { red, green, blue }
	}
}
impl FgdType for Srgb {
	const PROPERTY_TYPE: QuakeClassPropertyType = QuakeClassPropertyType::Value("color1");

	fn fgd_parse(input: &str) -> anyhow::Result<Self> {
		<[f32; 3]>::fgd_parse(input)
			.map(truncate_byte_color_range)
			.map(|[red, green, blue]| Self { red, green, blue })
	}
	fn fgd_to_string_unquoted(&self) -> String {
		format!("{} {} {}", self.red, self.green, self.blue)
	}
}
impl From<Srgb> for Srgba {
	fn from(value: Srgb) -> Self {
		Self::new(value.red, value.green, value.blue, 1.)
	}
}
impl From<Srgb> for Color {
	fn from(value: Srgb) -> Self {
		Self::Srgba(value.into())
	}
}

#[cfg(feature = "bsp")]
impl FgdType for LightmapStyle {
	const FGD_IS_QUOTED: bool = u8::FGD_IS_QUOTED;
	const PROPERTY_TYPE: QuakeClassPropertyType = u8::PROPERTY_TYPE;

	fn fgd_parse(input: &str) -> anyhow::Result<Self> {
		u8::fgd_parse(input).map(Self)
	}

	fn fgd_to_string_unquoted(&self) -> String {
		self.0.fgd_to_string_unquoted()
	}
}

// We don't support the more common `bitflags` crate because it doesn't seem to support `derive(Reflect)`,
// and as far as i know i can't get documentation from each flag.

/// Drop-in replacement for [`BitFlags`], but supports reflection and implements [`FgdType`].
#[derive(Reflect, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FgdFlags<T: BitFlag> {
	pub value: T::Numeric,
}
impl<T: BitFlag> From<BitFlags<T>> for FgdFlags<T> {
	fn from(value: BitFlags<T>) -> Self {
		Self { value: value.bits() }
	}
}
impl<T: BitFlag> From<FgdFlags<T>> for BitFlags<T> {
	fn from(value: FgdFlags<T>) -> Self {
		Self::from_bits_truncate(value.value)
	}
}
impl<T: BitFlag> Default for FgdFlags<T> {
	fn default() -> Self {
		BitFlags::default().into()
	}
}
impl<T: BitFlag + fmt::Debug> fmt::Debug for FgdFlags<T> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		BitFlags::from(*self).fmt(f)
	}
}
impl<T: BitFlag + fmt::Debug> fmt::Display for FgdFlags<T> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		BitFlags::from(*self).fmt(f)
	}
}

impl<
	N: FgdType + enumflags2::_internal::BitFlagNum + Into<u32>,
	T: BitFlag + enumflags2::_internal::RawBitFlags<Numeric = N> + Enum + GetTypeRegistration + FromReflect,
> FgdType for FgdFlags<T>
{
	const FGD_IS_QUOTED: bool = false;

	const PROPERTY_TYPE: QuakeClassPropertyType = QuakeClassPropertyType::Flags(|| {
		let enum_info = T::get_type_registration().type_info().as_enum().unwrap();

		Box::new((0..enum_info.variant_len()).flat_map(|variant_idx| {
			let variant = enum_info.variant_at(variant_idx)?;
			let Ok(variant) = variant.as_unit_variant() else { return None };
			let title = variant.docs().map(str::trim).unwrap_or(variant.name());
			let value = T::from_reflect(&DynamicEnum::new(variant.name(), DynamicVariant::Unit))?;

			Some((BitFlags::from_flag(value).bits().into(), title))
		}))
	});

	fn fgd_parse(input: &str) -> anyhow::Result<Self> {
		N::fgd_parse(input).map(T::from_bits_truncate).map(Into::into)
	}

	fn fgd_to_string_unquoted(&self) -> String {
		self.value.fgd_to_string_unquoted()
	}
}

impl<T: FgdType + Default + Copy, const N: usize> FgdType for [T; N] {
	const PROPERTY_TYPE: QuakeClassPropertyType = T::PROPERTY_TYPE;

	fn fgd_parse(input: &str) -> anyhow::Result<Self> {
		// This might be a problem for FgdTypes that use spaces in their parsing. Oh well!
		let mut out = [T::default(); N];

		for (i, input) in input.split_ascii_whitespace().enumerate() {
			if i >= out.len() {
				return Err(anyhow::anyhow!("Too many elements! Expected: {N}"));
			}
			out[i] = T::fgd_parse(input)?;
		}

		Ok(out)
	}
	fn fgd_to_string_unquoted(&self) -> String {
		self.iter().map(T::fgd_to_string_unquoted).join(" ")
	}
}

impl<T: FgdType> FgdType for Option<T> {
	const FGD_IS_QUOTED: bool = T::FGD_IS_QUOTED;
	const PROPERTY_TYPE: QuakeClassPropertyType = T::PROPERTY_TYPE;

	fn fgd_parse(input: &str) -> anyhow::Result<Self> {
		if input.trim().is_empty() {
			return Ok(None);
		}
		T::fgd_parse(input).map(Some)
	}

	fn fgd_to_string_unquoted(&self) -> String {
		match self {
			Some(v) => v.fgd_to_string_unquoted(),
			None => String::new(),
		}
	}
}
