mod fgd_type;
mod quake_class;

use heck::*;
use proc_macro2::*;
use quote::*;
use syn::*;

/// Point classes don't have any geometry built in -- simply a point in space.
///
/// If the `auto_register` feature is enabled, this will automatically register the type with `bevy_trenchbroom`.
///
/// # Type attributes
/// - `#[model(<path expression>)]` Displays the entity as the specified model in-editor.
/// - `#[model({ "path": <path expr>, "skin": <skin expr>, "frame": <frame expr>, "scale": <scale expr> })]` Same as above attribute, but with greater control over how the model is shown. Note that any of these properties can be left out.
/// - `#[color(<red> <green> <blue>)]` Changes the wireframe color of the entity. Each number has a range from 0 to 255.
/// - `#[iconsprite(...)]` Alias for `model`. When this or `model` is set to an image, it displays the entity as said image, presented as a billboard (always facing the camera).
/// - `#[size(<-x> <-y> <-z>, <+x> <+y> <+z>)]` The bounding box of the entity in-editor.
/// - `#[classname(<case type>)]` Case type can be something like `PascalCase` or `snake_case`. Default if not specified is `snake_case`.
/// - `#[classname(<string>)]` When outputted to fgd, use the specified string instead of a classname with case converted via the previous attribute.
/// - `#[require(<type ...>)]` From `Component` macro, this also sets this entity's base classes in the `FGD.
/// - `#[base(<type ...>)]` Overrides `require` attribute. Use if you want separate base classes and required components.
/// - `#[no_register]` Doesn't automatically register even if the `auto_register` feature is enabled.
///
/// # Field attributes
/// - `#[no_default]` Use on fields you want to output an error if not defined, rather than just being replaced by the field's default value.
#[proc_macro_derive(PointClass, attributes(model, color, iconsprite, size, classname, base, no_register, no_default))]
pub fn point_class_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
	quake_class::class_derive(parse_macro_input!(input as DeriveInput), quake_class::QuakeClassType::Point).into()
}

/// Solid classes contain brush geometry.
///
/// If the `auto_register` feature is enabled, this will automatically register the type with `bevy_trenchbroom`.
///
/// # Type attributes
/// - `#[geometry(<rust expression>)]` Required. An expression that produces a `GeometryProvider` to control how the geometry appears in the world.
/// - `#[classname(<case type>)]` Case type can be something like `PascalCase` or `snake_case`. Default if not specified is `snake_case`.
/// - `#[classname(<string>)]` When outputted to fgd, use the specified string instead of a classname with case converted via the previous attribute.
/// - `#[require(<type ...>)]` From `Component` macro, this also sets this entity's base classes in the `FGD.
/// - `#[base(<type ...>)]` Overrides `require` attribute. Use if you want separate base classes and required components.
/// - `#[no_register]` Doesn't automatically register even if the `auto_register` feature is enabled.
///
/// # Field attributes
/// - `#[no_default]` Use on fields you want to output an error if not defined, rather than just being replaced by the field's default value.
#[proc_macro_derive(SolidClass, attributes(geometry, classname, base, no_register, no_default))]
pub fn solid_class_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
	quake_class::class_derive(parse_macro_input!(input as DeriveInput), quake_class::QuakeClassType::Solid).into()
}

/// Base classes don't appear in-editor, rather they give properties and attributes to their sub-classes (components that require them).
///
/// If the `auto_register` feature is enabled, this will automatically register the type with `bevy_trenchbroom`.
///
/// It has the same attributes as [`PointClass`].
#[proc_macro_derive(BaseClass, attributes(model, color, iconsprite, size, classname, base, no_register, no_default))]
pub fn base_class_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
	quake_class::class_derive(parse_macro_input!(input as DeriveInput), quake_class::QuakeClassType::Base).into()
}

/// Any field in quake class components must implement `FgdType`. Specifically, this macro implements it for unit enums, to create `choices` properties.
///
/// By default, it uses the name of the variant as the key. To use the discriminant of the variant, use the `#[number_key]` attribute on the struct.
#[proc_macro_derive(FgdType, attributes(number_key))]
pub fn fgd_type_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
	fgd_type::fgd_type_derive(parse_macro_input!(input as DeriveInput)).into()
}

/// Returns `true` if the path contains a single segment, that segment being `s`.
fn compare_path(path: &Path, s: &str) -> bool {
	path.segments.len() == 1 && path.segments[0].ident == s
}

/// Returns a token stream where if `value` is [`Some`], returns `Some(<value>)`, else returns a token stream containing `None`.
fn option(value: Option<impl quote::ToTokens>) -> TokenStream {
	match value {
		Some(value) => quote! { Some(#value) },
		None => quote!(None),
	}
}

/// Unit enum version of [`Fields`].
enum FieldsType {
	Named,
	Unnamed,
	Unit,
}
impl FieldsType {
	pub fn from_fields(fields: &Fields) -> Self {
		match fields {
			Fields::Named(_) => Self::Named,
			Fields::Unnamed(_) => Self::Unnamed,
			Fields::Unit => Self::Unit,
		}
	}
}
