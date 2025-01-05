use class::{QuakeClassProperties, QuakeClassPropertyType};

use crate::*;

impl TrenchBroomConfig {
    /// Converts this config to a string for writing `fgd` (entity definition) files.
    pub fn to_fgd(&self) -> String {
        use fmt::Write;
        let mut s = String::new();

        for class in self.class_iter() {
            write!(s, "@{:?}Class ", class.info.ty);
    
            if !class.info.base.is_empty() {
                write!(s, "base({}) ", class.info.base.join(", "));
            }
    
            if let Some(value) = class.info.color {
                write!(s, "color({value})");
            }
            if let Some(value) = class.info.iconsprite {
                write!(s, "iconsprite({value})");
            }
            if let Some(value) = class.info.size {
                write!(s, "size({value})");
            }
            if let Some(value) = class.info.model {
                write!(s, "model({value})");
            }
    
            write!(s, "= {}", class.info.name);
            if let Some(description) = class.info.description {
                write!(s, " : \"{description}\"");
            }
            write!(s, "\n[\n");
    
            let mut properties = QuakeClassProperties::new();
            (class.properties_fn)(self, &mut properties);

            for (property_name, property) in properties.values.into_iter() {
                write!(s, "\t{property_name}({}): \"{}\" : {} : \"{}\"",
                    match &property.ty {
                        QuakeClassPropertyType::Value(ty) => ty,
                        QuakeClassPropertyType::Choices(_) => "choices",
                    },
                    property.title.unwrap_or(property_name.clone()),
                    property.default_value.unwrap_or_default(),
                    property.description.unwrap_or_default(),
                );

                if let QuakeClassPropertyType::Choices(choices) = property.ty {
                    write!(s, " = \n\t[\n");
                    for (key, title) in choices {
                        write!(s, "\t\t{key} : \"{title}\"\n");
                    }
                    write!(s, "\t]");
                }

                write!(s, "\n");
            }

            write!(s, "]\n\n");
        }
        
        s
    }
}

/// Contains Quake/TrenchBroom-specific parsing and stringification functions.
pub trait FgdType: Sized {
    /// If quotes should be put around this value when writing out an FGD file.
    const FGD_IS_QUOTED: bool = true;

    /// Parses a string into `Self` FGD-style. Used for parsing entity properties.
    fn fgd_parse(input: &str) -> anyhow::Result<Self>; // TODO do we want to keep anyhow?
    /// Converts this value into a string used for writing FGDs.
    fn fgd_to_string(&self) -> String;
    /// Calls `fgd_to_string`, but if `FGD_IS_QUOTED` is true, surrounds the output with quotes.
    fn fgd_to_string_quoted(&self) -> String {
        if Self::FGD_IS_QUOTED {
            format!("\"{}\"", self.fgd_to_string())
        } else {
            self.fgd_to_string()
        }
    }

    fn fgd_type() -> QuakeClassPropertyType;
}

impl FgdType for String {
    fn fgd_parse(input: &str) -> anyhow::Result<Self> {
        Ok(input.to_string())
    }
    fn fgd_to_string(&self) -> String {
        self.clone()
    }
    fn fgd_type() -> QuakeClassPropertyType {
        QuakeClassPropertyType::Value("string".s())
    }
}

impl FgdType for &str {
    fn fgd_parse(_input: &str) -> anyhow::Result<Self> {
        // Lifetimes don't allow me to just return Some(input) unfortunately.
        unimplemented!("use String::fgd_parse instead");
    }
    fn fgd_to_string(&self) -> String {
        self.to_string()
    }
    fn fgd_type() -> QuakeClassPropertyType {
        QuakeClassPropertyType::Value("string".s())
    }
}

macro_rules! simple_fgd_type_impl {
    ($ty:ty, $quoted:expr, $fgd_type:ident $fgd_type_value:expr) => {
        impl FgdType for $ty {
            const FGD_IS_QUOTED: bool = $quoted;

            fn fgd_parse(input: &str) -> anyhow::Result<Self> {
                Ok(input.parse()?)
            }
            fn fgd_to_string(&self) -> String {
                self.to_string()
            }
            fn fgd_type() -> QuakeClassPropertyType {
                QuakeClassPropertyType::$fgd_type($fgd_type_value.into())
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

simple_fgd_type_impl!(bool, true, Choices [("true".fgd_to_string_quoted(), "true".s()), ("false".fgd_to_string_quoted(), "false".s())]);

simple_fgd_type_impl!(f32, true, Value "float");
simple_fgd_type_impl!(f64, true, Value "float");

impl FgdType for Aabb {
    const FGD_IS_QUOTED: bool = false;

    fn fgd_parse(input: &str) -> anyhow::Result<Self> {
        let values = <[f32; 6]>::fgd_parse(input)?;
        Ok(Aabb::from_min_max(
            Vec3::from_slice(&values[0..=3]),
            Vec3::from_slice(&values[3..=6]),
        ))
    }
    fn fgd_to_string(&self) -> String {
        let min = self.min();
        let max = self.max();
        format!(
            "{} {} {}, {} {} {}",
            min.x, min.y, min.z, max.x, max.y, max.z
        )
    }
    fn fgd_type() -> QuakeClassPropertyType {
        QuakeClassPropertyType::Value("aabb".s())
    }
}

impl FgdType for Vec4 {
    fn fgd_parse(input: &str) -> anyhow::Result<Self> {
        <[f32; 4]>::fgd_parse(input).map(Vec4::from)
    }
    fn fgd_to_string(&self) -> String {
        format!("{} {} {} {}", self.x, self.y, self.z, self.w)
    }
    fn fgd_type() -> QuakeClassPropertyType {
        QuakeClassPropertyType::Value("vec4".s())
    }
}
impl FgdType for Vec3 {
    fn fgd_parse(input: &str) -> anyhow::Result<Self> {
        <[f32; 3]>::fgd_parse(input).map(Vec3::from)
    }
    fn fgd_to_string(&self) -> String {
        format!("{} {} {}", self.x, self.y, self.z)
    }
    fn fgd_type() -> QuakeClassPropertyType {
        QuakeClassPropertyType::Value("vector".s())
    }
}
impl FgdType for Vec2 {
    fn fgd_parse(input: &str) -> anyhow::Result<Self> {
        <[f32; 2]>::fgd_parse(input).map(Vec2::from)
    }
    fn fgd_to_string(&self) -> String {
        format!("{} {}", self.x, self.y)
    }
    fn fgd_type() -> QuakeClassPropertyType {
        QuakeClassPropertyType::Value("vec2".s())
    }
}

impl FgdType for Color {
    fn fgd_parse(input: &str) -> anyhow::Result<Self> {
        <[f32; 3]>::fgd_parse(input)
            .map(Color::srgb_from_array)
            .or(<[f32; 4]>::fgd_parse(input).map(|[r, g, b, a]| Color::srgba(r, g, b, a)))
    }
    fn fgd_to_string(&self) -> String {
        let col = self.to_srgba();
        format!("{} {} {} {}", col.red, col.green, col.blue, col.alpha)
    }
    fn fgd_type() -> QuakeClassPropertyType {
        QuakeClassPropertyType::Value("color1".s())
    }
}

// God i love rust's trait system
impl<T: FgdType + Default + Copy, const N: usize> FgdType for [T; N] {
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
    fn fgd_to_string(&self) -> String {
        self.iter().map(T::fgd_to_string).join(" ")
    }
    fn fgd_type() -> QuakeClassPropertyType {
        T::fgd_type()
    }
}