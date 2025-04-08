use crate::*;
use deluxe::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum QuakeClassType {
	Base,
	Point,
	Solid,
}

struct Tokens(TokenStream);
impl ParseMetaItem for Tokens {
	fn parse_meta_item(input: parse::ParseStream, _mode: ParseMode) -> syn::Result<Self> {
		input.parse::<TokenStream>().map(Self)
	}
}
/* impl FromMeta for Tokens {
	fn from_meta(item: &Meta) -> darling::Result<Self> {
		match item {
			Meta::Path(_) => Self::from_word(),
			Meta::List(list) => Ok(Self(list.tokens.clone())),
			Meta::NameValue(ref value) => Self::from_expr(&value.value),
		}
	}
} */

/* #[derive(Default, FromMeta)]
#[darling(default)]
struct ClassOpts {

} */

struct Size {
	from_x: f32,
	from_y: f32,
	from_z: f32,
	to_x: f32,
	to_y: f32,
	to_z: f32,
}
impl ParseMetaItem for Size {
	fn parse_meta_item(input: parse::ParseStream, mode: ParseMode) -> syn::Result<Self> {
		fn parse_number(input: parse::ParseStream, mode: ParseMode, msg: &str) -> f32 {
			if let Ok(i) = i32::parse_meta_item(input, mode) {
				return i as f32;
			}
			f32::parse_meta_item(input, mode).expect(msg)
		}

		let from_x = parse_number(input, mode, "from_x");
		let from_y = parse_number(input, mode, "from_y");
		let from_z = parse_number(input, mode, "from_z");
		input.parse::<Token![,]>().expect("Size: expected comma");
		let to_x = parse_number(input, mode, "to_x");
		let to_y = parse_number(input, mode, "to_y");
		let to_z = parse_number(input, mode, "to_z");

		Ok(Self {
			from_x,
			from_y,
			from_z,
			to_x,
			to_y,
			to_z,
		})
	}
}
impl std::fmt::Display for Size {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let Self {
			from_x,
			from_y,
			from_z,
			to_x,
			to_y,
			to_z,
		} = self;
		write!(f, "{from_x} {from_y} {from_z}, {to_x} {to_y} {to_z}")
	}
}

#[derive(Default, ExtractAttributes)]
#[deluxe(default, attributes(class))]
struct Opts {
	model: Option<Tokens>,
	color: Option<Tokens>,
	iconsprite: Option<Tokens>,
	size: Option<Size>,
	geometry: Option<Expr>,
	classname: Option<Tokens>,
	base: Vec<Type>,
	no_register: bool,
	doc: Option<String>,
}

fn extract_doc(meta: MetaNameValue, doc: &mut Option<String>) {
	let Expr::Lit(ExprLit { lit: Lit::Str(lit), .. }) = meta.value else { return };
	let value = lit.value().trim().replace('"', "''");

	let s = doc.get_or_insert_default();

	if !s.is_empty() {
		s.push(' ');
	}

	if value.is_empty() {
		s.push('\n');
	} else {
		s.push_str(&value);
	}
}

pub(super) fn class_derive(mut input: DeriveInput, ty: QuakeClassType) -> TokenStream {
	let mut opts = match Opts::extract_attributes(&mut input) {
		Ok(x) => x,
		Err(err) => panic!("Parsing attributes: {err}"),
	};
	let DeriveInput { ident, attrs, data, .. } = input;
	let ty_ident = format_ident!("{ty:?}");

	for attr in attrs {
		if let Meta::NameValue(meta) = attr.meta {
			if opts.doc.is_none() && ty != QuakeClassType::Base && compare_path(&meta.path, "doc") {
				extract_doc(meta, &mut opts.doc);
			}
		}
	}

	let mut properties: Vec<TokenStream> = Vec::new();

	let spawn_constructor: TokenStream;
	// If fields are present, we'll construct a default instance to help if any fields are missing.
	let mut spawn_constructor_default_value: Option<TokenStream> = None;

	match data {
		Data::Struct(data) => {
			let mut field_constructors = Vec::with_capacity(data.fields.len());
			let fields_type = FieldsType::from_fields(&data.fields);

			if !data.fields.is_empty() {
				spawn_constructor_default_value = Some(quote! {
					let default = <#ident as Default>::default();
				});
			}

			for (field_idx, field) in data.fields.into_iter().enumerate() {
				let ty = field.ty;
				let field_ident = field.ident.clone();
				let field_name = field.ident.map(|ident| ident.to_string()).unwrap_or_else(|| field_idx.to_string());
				let field_ident_or_number = Ident::new(&field_name, Span::mixed_site());

				let mut doc = None;
				let mut defaulted = true;

				for attr in field.attrs {
					match attr.meta {
						Meta::NameValue(meta) => {
							if compare_path(&meta.path, "doc") {
								extract_doc(meta, &mut doc);
							}
						}
						Meta::Path(path) => {
							if compare_path(&path, "no_default") {
								defaulted = false;
							}
						}
						_ => {}
					}
				}

				let description = option(doc);

				let default_value_fn = if defaulted {
					quote! { Some(|| ::bevy_trenchbroom::fgd::FgdType::fgd_to_string(&<Self as Default>::default().#field_ident_or_number)) }
				} else {
					quote! { None }
				};

				properties.push(quote! {
					::bevy_trenchbroom::class::QuakeClassProperty {
						ty: <#ty as ::bevy_trenchbroom::fgd::FgdType>::PROPERTY_TYPE,
						name: #field_name,
						title: None,
						description: #description,
						default_value: #default_value_fn,
					},
				});

				let setter = field_ident.as_ref().map(|ident| quote! { #ident: });

				let not_found_handler = if defaulted {
					quote! { .with_default(default.#field_ident_or_number)? }
				} else {
					quote! { ? }
				};

				field_constructors.push(quote! {
					#setter src_entity.get(#field_name)#not_found_handler,
				});
			}

			spawn_constructor = match fields_type {
				FieldsType::Named => quote! { Self { #(#field_constructors)* } },
				FieldsType::Unnamed => quote! { Self(#(#field_constructors)*) },
				FieldsType::Unit => quote! { Self },
			};
		}
		_ => panic!("Only structs supported"),
	}

	if opts.geometry.is_none() && ty == QuakeClassType::Solid {
		panic!("Solid classes must have a `#[geometry(...)]` attribute.");
	}

	let inventory_submit = (cfg!(feature = "auto_register") && !opts.no_register).then(|| {
		quote! {
			::bevy_trenchbroom::inventory::submit! { <#ident as ::bevy_trenchbroom::class::QuakeClass>::ERASED_CLASS }
		}
	});

	let name = opts
		.classname
		.and_then(|tokens| tokens.0.into_iter().next())
		.map(|tree| match tree {
			TokenTree::Literal(lit) => lit,
			TokenTree::Ident(ident) => match ident.to_string().as_str() {
				"snake_case" => Literal::string(&ident.to_string().to_snake_case()),
				"UPPER_SNAKE_CASE" => Literal::string(&ident.to_string().to_shouty_snake_case()),
				"lowercase" => Literal::string(&ident.to_string().to_lowercase()),
				"UPPERCASE" => Literal::string(&ident.to_string().to_uppercase()),
				"camelCase" => Literal::string(&ident.to_string().to_lower_camel_case()),
				"PascalCase" => Literal::string(&ident.to_string().to_pascal_case()),
				_ => panic!("Invalid casing! Valid casings are snake_case, UPPER_SNAKE_CASE, lowercase, UPPERCASE, camelCase, and PascalCase."),
			},
			_ => panic!("Invalid arguments! Must either be a casing like snake_case, or a name like \"worldspawn\"!"),
		})
		.unwrap_or_else(|| Literal::string(&ident.to_string().to_snake_case()));
	let description = option(opts.doc);

	let bases = opts.base.into_iter();

	let model = option(opts.model.map(|Tokens(model)| quote! { stringify!(#model) }));
	let color = option(opts.color.map(|Tokens(color)| quote! { stringify!(#color) }));
	let iconsprite = option(opts.iconsprite.map(|Tokens(iconsprite)| quote! { stringify!(#iconsprite) }));
	let size = option(opts.size.map(|size| size.to_string().to_token_stream()));

	let geometry_provider = opts.geometry.map(|expr| {
		quote! { (|| #expr) }
	});

	quote! {
		impl ::bevy_trenchbroom::class::QuakeClass for #ident {
			const CLASS_INFO: ::bevy_trenchbroom::class::QuakeClassInfo = ::bevy_trenchbroom::class::QuakeClassInfo {
				ty: ::bevy_trenchbroom::class::QuakeClassType::#ty_ident #geometry_provider,
				name: #name,
				description: #description,
				base: &[#(<#bases as ::bevy_trenchbroom::class::QuakeClass>::ERASED_CLASS),*],

				model: #model,
				color: #color,
				iconsprite: #iconsprite,
				size: #size,

				properties: &[#(#properties)*],
			};

			#[allow(unused)]
			fn class_spawn(config: &::bevy_trenchbroom::config::TrenchBroomConfig, src_entity: &::bevy_trenchbroom::qmap::QuakeMapEntity, entity: &mut ::bevy::ecs::world::EntityWorldMut) -> ::bevy_trenchbroom::anyhow::Result<()> {
				use ::bevy_trenchbroom::qmap::QuakeEntityErrorResultExt;
				#spawn_constructor_default_value
				entity.insert(#spawn_constructor);
				Ok(())
			}
		}

		#inventory_submit
	}
}
