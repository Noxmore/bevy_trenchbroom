use crate::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum QuakeClassType {
	Base,
	Point,
	Solid,
}

#[derive(Default)]
struct Opts {
	model: Option<TokenStream>,
	color: Option<TokenStream>,
	iconsprite: Option<TokenStream>,
	size: Option<TokenStream>,
	geometry: Option<TokenStream>,
	classname: Option<TokenStream>,

	/// `true` if the `#[base(...)]` attribute is used to override, otherwise uses `#[require(...)]`
	base_override: bool,
	base: Vec<Type>,
	no_register: bool,
	doc: Option<String>,
}

fn extract_type_list(tokens: TokenStream) -> impl Iterator<Item = Type> {
	let types: syn::punctuated::Punctuated<Type, Token![,]> = syn::parse_quote!(#tokens);

	types.into_iter()
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

pub(super) fn class_derive(input: DeriveInput, ty: QuakeClassType) -> TokenStream {
	let DeriveInput { ident, attrs, data, .. } = input;
	let ty_ident = format_ident!("{ty:?}");

	let mut opts = Opts::default();

	for attr in attrs {
		match attr.meta {
			Meta::NameValue(meta) => {
				if ty != QuakeClassType::Base && compare_path(&meta.path, "doc") {
					extract_doc(meta, &mut opts.doc);
				}
			}
			Meta::List(meta) => {
				if compare_path(&meta.path, "model") {
					opts.model = Some(meta.tokens);
				} else if compare_path(&meta.path, "color") {
					opts.color = Some(meta.tokens);
				} else if compare_path(&meta.path, "iconsprite") {
					opts.iconsprite = Some(meta.tokens);
				} else if compare_path(&meta.path, "size") {
					opts.size = Some(meta.tokens);
				} else if compare_path(&meta.path, "geometry") {
					opts.geometry = Some(meta.tokens);
				} else if compare_path(&meta.path, "classname") {
					opts.classname = Some(meta.tokens);
				} else if compare_path(&meta.path, "require") && !opts.base_override {
					opts.base.extend(extract_type_list(meta.tokens));
				} else if compare_path(&meta.path, "base") {
					if !opts.base_override {
						opts.base.clear();
					}
					opts.base_override = true;

					opts.base.extend(extract_type_list(meta.tokens));
				}
			}
			Meta::Path(path) => {
				if compare_path(&path, "no_register") {
					opts.no_register = true;
				}
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
		.and_then(|tokens| tokens.into_iter().next())
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

	let bases = opts.base;

	let model = option(opts.model.map(|model| quote! { stringify!(#model) }));
	let color = option(opts.color.map(|color| quote! { stringify!(#color) }));
	let iconsprite = option(opts.iconsprite.map(|iconsprite| quote! { stringify!(#iconsprite) }));
	let size = option(opts.size.map(|size| quote! { stringify!(#size) }));

	let geometry_provider = opts.geometry.map(|tokens| {
		quote! { (|| #tokens) }
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
