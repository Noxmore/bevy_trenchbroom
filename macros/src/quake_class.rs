use deluxe::*;
use syn::punctuated::Punctuated;

use crate::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum QuakeClassType {
	Base,
	Point,
	Solid,
}

/// Custom [`ParseMetaItem`] that just reads a [`TokenStream`]. There might already by an implementation in deluxe, but i searched a while and didn't find anything.
struct Tokens(TokenStream);
impl ParseMetaItem for Tokens {
	fn parse_meta_item(input: parse::ParseStream, _mode: ParseMode) -> syn::Result<Self> {
		input.parse::<TokenStream>().map(Self)
	}
}

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
		} = *self;
		write!(f, "{from_x} {from_y} {from_z}, {to_x} {to_y} {to_z}")
	}
}

struct BaseType {
	pub attrs: Vec<Attribute>,
	pub ty: Type,
}
impl ParseMetaItem for BaseType {
	fn parse_meta_item(input: parse::ParseStream, _mode: ParseMode) -> deluxe::Result<Self> {
		Ok(Self {
			attrs: Attribute::parse_outer(input)?,
			ty: input.parse()?,
		})
	}
}

#[derive(Default, ParseMetaItem)]
#[deluxe(default)]
struct Opts {
	model: Option<Tokens>,
	color: Option<Tokens>,
	iconsprite: Option<Tokens>,
	size: Option<Size>,
	hooks: Option<Expr>,
	classname: Option<Tokens>,
	group: Option<String>,
	base: Vec<BaseType>,
	doc: Option<String>,
	decal: bool,
}

#[derive(Default, ParseMetaItem, ParseAttributes)]
#[deluxe(default, attributes(class))]
struct FieldOpts {
	must_set: bool,
	ignore: bool,
	rename: Option<String>,
	default: Option<Literal>,
	title: Option<String>,
}

fn extract_doc(meta: &MetaNameValue, doc: &mut Option<String>) {
	let Expr::Lit(ExprLit { lit: Lit::Str(lit), .. }) = &meta.value else { return };
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

fn simple_path(s: &str) -> Path {
	Path::from(Ident::new(s, Span::mixed_site()))
}
fn simple_type_path(s: &str) -> TypePath {
	TypePath {
		qself: None,
		path: simple_path(s),
	}
}

pub(super) fn class_attribute(attr: TokenStream, input: TokenStream, ty: QuakeClassType) -> TokenStream {
	let mut opts = match deluxe::parse2::<Opts>(attr) {
		Ok(x) => x,
		Err(err) => panic!("Parsing attributes: {err}   | source code: {:?}", err.span().source_text()),
	};

	let mut item = syn::parse2::<ItemStruct>(input).expect("Must be a struct as input!");

	insert_required_attributes(&mut item);

	// Collect field attributes beforehand
	let mut field_attributes = Vec::with_capacity(item.fields.len());
	for field in item.fields.iter_mut() {
		let field_ident = field.ident.as_ref().expect("Field doesn't have identifier, tuple structs not allowed");

		let field_opts: FieldOpts = match deluxe::parse_attributes(field) {
			Ok(x) => x,
			Err(err) => panic!(
				"Parsing attributes for {field_ident}: {err}   | source code: {:?}",
				err.span().source_text()
			),
		};

		field_attributes.push(field_opts);

		field.attrs.retain(|attr| !compare_path(attr.meta.path(), "class"));
	}

	let ItemStruct { ident, attrs, fields, .. } = &item;
	let ty_ident = format_ident!("{ty:?}");

	for attr in attrs {
		if let Meta::NameValue(meta) = &attr.meta {
			if opts.doc.is_none() && ty != QuakeClassType::Base && compare_path(&meta.path, "doc") {
				extract_doc(meta, &mut opts.doc);
			}
		}
	}

	let mut properties: Vec<TokenStream> = Vec::new();

	let spawn_constructor: TokenStream;
	// If fields are present, we'll construct a default instance to help if any fields are missing.
	let mut spawn_constructor_default_value: Option<TokenStream> = None;

	let mut field_constructors = Vec::with_capacity(fields.len());
	let fields_type = FieldsType::from_fields(fields);

	if !fields.is_empty() {
		spawn_constructor_default_value = Some(quote! {
			let default = <#ident as Default>::default();
		});
	}

	for (field_idx, field) in fields.into_iter().enumerate() {
		let ty = &field.ty;
		let field_ident = field.ident.as_ref().unwrap();
		let field_ident_string = field_ident.to_string();

		let mut doc = None;

		let field_opts = &field_attributes[field_idx];
		if field_opts.ignore {
			field_constructors.push(quote! {
				#field_ident: default.#field_ident,
			});
			continue;
		}
		let property_name = field_opts.rename.as_ref().unwrap_or(&field_ident_string);

		for attr in &field.attrs {
			if let Meta::NameValue(meta) = &attr.meta {
				if compare_path(&meta.path, "doc") {
					extract_doc(meta, &mut doc);
				}
			}
		}

		let title = option(field_opts.title.clone());
		let description = option(doc);

		let default_value_fn = if let Some(default) = &field_opts.default {
			quote! { Some(|| stringify!(#default).to_owned()) }
		} else if field_opts.must_set {
			quote! { None }
		} else {
			quote! { Some(|| ::bevy_trenchbroom::fgd::FgdType::fgd_to_string(&<Self as Default>::default().#field_ident)) }
		};

		properties.push(quote! {
			::bevy_trenchbroom::class::QuakeClassProperty {
				ty: <#ty as ::bevy_trenchbroom::fgd::FgdType>::PROPERTY_TYPE,
				name: #property_name,
				title: #title,
				description: #description,
				default_value: #default_value_fn,
			},
		});

		let not_found_handler = if field_opts.must_set {
			quote! { ? }
		} else {
			quote! { .with_default(default.#field_ident)? }
		};

		field_constructors.push(quote! {
			#field_ident: view.src_entity.get(#property_name)#not_found_handler,
		});
	}

	spawn_constructor = match fields_type {
		FieldsType::Named => quote! { Self { #(#field_constructors)* } },
		FieldsType::Unnamed => quote! { Self(#(#field_constructors)*) },
		FieldsType::Unit => quote! { Self },
	};

	let mut name = opts
		.classname
		.and_then(|Tokens(tokens)| tokens.into_iter().next())
		.map(|tree| match tree {
			TokenTree::Literal(lit) => lit.to_string().trim_matches('"').to_owned(),
			TokenTree::Ident(casing) => match casing.to_string().as_str() {
				"snake_case" => ident.to_string().to_snake_case(),
				"UPPER_SNAKE_CASE" => ident.to_string().to_shouty_snake_case(),
				"lowercase" => ident.to_string().to_lowercase(),
				"UPPERCASE" => ident.to_string().to_uppercase(),
				"camelCase" => ident.to_string().to_lower_camel_case(),
				"PascalCase" => ident.to_string().to_pascal_case(),
				_ => panic!("Invalid casing! Valid casings are snake_case, UPPER_SNAKE_CASE, lowercase, UPPERCASE, camelCase, and PascalCase."),
			},
			_ => panic!("Invalid arguments! Must either be a casing like snake_case, or a name like \"worldspawn\"!"),
		})
		.unwrap_or_else(|| ident.to_string().to_snake_case());
	if let Some(group) = &opts.group {
		name = format!("{group}_{name}");
	}
	let description = option(opts.doc);

	// Attribute::to
	let bases = opts
		.base
		.into_iter()
		.map(|BaseType { attrs, ty }| quote! { #(#attrs)* <#ty as ::bevy_trenchbroom::class::QuakeClass>::ERASED_CLASS });

	let model = option(opts.model.map(|Tokens(model)| quote! { stringify!(#model) }));
	let color = option(opts.color.map(|Tokens(color)| quote! { stringify!(#color) }));
	let iconsprite = option(opts.iconsprite.map(|Tokens(iconsprite)| quote! { stringify!(#iconsprite) }));
	let size = option(opts.size.map(|size| size.to_string()));
	let decal = opts.decal;

	let spawn_hooks = match opts.hooks {
		None => match ty {
			QuakeClassType::Base => quote! { (view.tb_config.default_base_spawn_hooks)() },
			QuakeClassType::Point => quote! { (view.tb_config.default_point_spawn_hooks)() },
			QuakeClassType::Solid => quote! { (view.tb_config.default_solid_spawn_hooks)() },
		},
		Some(hooks) => hooks.to_token_stream(),
	};

	quote! {
		#item

		#[automatically_derived]
		impl ::bevy_trenchbroom::class::QuakeClass for #ident {
			const CLASS_INFO: ::bevy_trenchbroom::class::QuakeClassInfo = ::bevy_trenchbroom::class::QuakeClassInfo {
				ty: ::bevy_trenchbroom::class::QuakeClassType::#ty_ident,
				name: #name,
				description: #description,
				base: &[#(#bases),*],

				model: #model,
				color: #color,
				iconsprite: #iconsprite,
				size: #size,
				decal: #decal,

				properties: &[#(#properties)*],
			};

			fn class_spawn(view: &mut ::bevy_trenchbroom::class::QuakeClassSpawnView) -> ::bevy_trenchbroom::anyhow::Result<()> {
				use ::bevy_trenchbroom::qmap::QuakeEntityErrorResultExt;
				#spawn_constructor_default_value
				view.world.entity_mut(view.entity).insert(#spawn_constructor);
				let hooks: ::bevy_trenchbroom::class::spawn_hooks::SpawnHooks = #spawn_hooks;
				hooks.apply(view)?;
				Ok(())
			}
		}
	}
}

fn insert_required_attributes(item: &mut ItemStruct) {
	// Insert required attributes
	let mut has_derive_component = false;
	let mut has_derive_reflect = false;
	let mut has_reflect_quake_class = false;
	let mut has_reflect_component = false;
	for attr in &item.attrs {
		let Meta::List(meta) = &attr.meta else { continue };

		let is_derive = compare_path(&meta.path, "derive");
		let is_reflect = compare_path(&meta.path, "reflect");

		if !is_derive && !is_reflect {
			continue;
		}

		let tokens = &meta.tokens;
		let types: Punctuated<TypePath, Token![,]> = syn::parse_quote!(#tokens);

		// NOTE: These do not account for fully qualified paths, i don't know how we could account for that. It's probably not a big deal though.
		if is_derive {
			for ty in &types {
				if compare_path(&ty.path, "Component") {
					has_derive_component = true;
				} else if compare_path(&ty.path, "Reflect") {
					has_derive_reflect = true;
				}
			}
		} else if is_reflect {
			for ty in &types {
				if compare_path(&ty.path, "QuakeClass") {
					has_reflect_quake_class = true;
				} else if compare_path(&ty.path, "Component") {
					has_reflect_component = true;
				}
			}
		}
	}
	let mut derive_types = Punctuated::<TypePath, Token![,]>::new();
	if !has_derive_component {
		derive_types.push(syn::parse_quote! { ::bevy::prelude::Component });
	}
	if !has_derive_reflect {
		derive_types.push(syn::parse_quote! { ::bevy::prelude::Reflect });
	}
	if !derive_types.is_empty() {
		item.attrs.insert(
			0,
			Attribute {
				pound_token: Default::default(),
				style: AttrStyle::Outer,
				bracket_token: Default::default(),
				meta: Meta::List(MetaList {
					path: simple_path("derive"),
					delimiter: MacroDelimiter::Paren(Default::default()),
					tokens: derive_types.to_token_stream(),
				}),
			},
		);
	}
	let mut reflect_types = Punctuated::<TypePath, Token![,]>::new();
	if !has_reflect_quake_class {
		reflect_types.push(simple_type_path("QuakeClass"));
	}
	if !has_reflect_component {
		reflect_types.push(simple_type_path("Component"));
	}
	if !reflect_types.is_empty() {
		item.attrs.push(Attribute {
			pound_token: Default::default(),
			style: AttrStyle::Outer,
			bracket_token: Default::default(),
			meta: Meta::List(MetaList {
				path: simple_path("reflect"),
				delimiter: MacroDelimiter::Paren(Default::default()),
				tokens: reflect_types.to_token_stream(),
			}),
		});
	}
}
