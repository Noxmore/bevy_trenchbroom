mod fgd_type;
mod quake_class;

use heck::*;
use proc_macro2::*;
use quote::*;
use syn::*;

// TODO spawnflags support using something like bitflags?

#[proc_macro_derive(PointClass, attributes(model, color, iconsprite, size, classname, base, no_default))]
pub fn point_class_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
	quake_class::class_derive(parse_macro_input!(input as DeriveInput), quake_class::QuakeClassType::Point).into()
}

#[proc_macro_derive(SolidClass, attributes(geometry, classname, base, no_default))]
pub fn solid_class_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
	quake_class::class_derive(parse_macro_input!(input as DeriveInput), quake_class::QuakeClassType::Solid).into()
}

#[proc_macro_derive(BaseClass, attributes(model, color, iconsprite, size, classname, base, no_default))]
pub fn base_class_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
	quake_class::class_derive(parse_macro_input!(input as DeriveInput), quake_class::QuakeClassType::Base).into()
}

#[proc_macro_derive(FgdType)]
pub fn fgd_type_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
	fgd_type::fgd_type_derive(parse_macro_input!(input as DeriveInput)).into()
}

fn compare_path(path: &Path, s: &str) -> bool {
	path.segments
		== [PathSegment {
			ident: Ident::new(s, Span::mixed_site()),
			arguments: PathArguments::None,
		}]
		.into_iter()
		.collect()
}

fn option(value: Option<impl quote::ToTokens>) -> TokenStream {
	match value {
		Some(value) => quote! { Some(#value) },
		None => quote!(None),
	}
}

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
