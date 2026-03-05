use crate::*;

pub(super) fn fgd_type_derive(input: DeriveInput) -> TokenStream {
	let DeriveInput { ident, data, attrs, .. } = input;

	let mut property_type_choices = Vec::new();

	let mut variant_idents = Vec::new();
	let mut variants = Vec::new();
	let mut variant_strings = Vec::new();

	let mut number_key = false;

	for attr in attrs {
		let Meta::Path(path) = attr.meta else { continue };

		if compare_path(&path, "number_key") {
			number_key = true;
		}
	}

	match data {
		Data::Enum(data) => {
			for Variant {
				ident: variant_ident,
				attrs,
				fields,
				discriminant,
				..
			} in data.variants
			{
				if !fields.is_empty() {
					panic!("Only unit enums supported, variant {variant_ident} has fields");
				}

				let mut doc = None;

				for attr in attrs {
					let Meta::NameValue(meta) = attr.meta else { continue };

					if compare_path(&meta.path, "doc") {
						doc = Some(meta.value);
					}
				}

				let variant: TokenStream;
				let key: TokenStream;
				let variant_string: String;

				if number_key {
					let Some((_, value)) = discriminant else {
						panic!("Variant `{variant_ident}` doesn't have a discriminant! Add `{variant_ident} = <number>`.");
					};
					let number: i32 = eval_unary_const_i32(&variant_ident, value);

					variant = quote! { #number };
					key = quote! { ::bevy_trenchbroom::class::ChoicesKey::Integer(#number) };
					variant_string = number.to_string();
				} else {
					let variant_ident_string = variant_ident.to_string();

					variant = quote! { #variant_ident_string };
					key = quote! { ::bevy_trenchbroom::class::ChoicesKey::String(#variant_ident_string) };
					variant_string = variant_ident_string;
				}

				variants.push(variant);
				variant_strings.push(variant_string);

				let title = doc.unwrap_or_else(|| {
					Expr::Lit(ExprLit {
						attrs: Vec::new(),
						lit: Lit::new(Literal::string(&variant_ident.to_string())),
					})
				});

				property_type_choices.push(quote! {
					(#key, #title),
				});

				variant_idents.push(variant_ident);
			}
		}
		_ => panic!("Currently only enums supported"),
	}

	let valid_variants = variant_strings.join(", ");

	quote! {
		#[automatically_derived]
		impl ::bevy_trenchbroom::fgd::FgdType for #ident {
			const PROPERTY_TYPE: ::bevy_trenchbroom::class::QuakeClassPropertyType = ::bevy_trenchbroom::class::QuakeClassPropertyType::Choices(&[
				#(#property_type_choices)*
			]);

			fn fgd_parse(input: &str) -> ::bevy_trenchbroom::anyhow::Result<Self> {
				match input {
					#(#variant_strings => Ok(Self::#variant_idents),)*
					input => Err(::bevy_trenchbroom::anyhow::anyhow!(concat!("{} isn't a valid ", stringify!(#ident), "! Valid variants are ", #valid_variants), input)),
				}
			}

			fn fgd_to_string_unquoted(&self) -> String {
				match self {
					#(Self::#variant_idents => #variant_strings.to_string(),)*
				}
			}
		}
	}
}

fn eval_unary_const_i32(ident: &Ident, expr: Expr) -> i32 {
	match expr {
		Expr::Lit(ExprLit { lit: Lit::Int(number), .. }) => number.base10_parse::<i32>().unwrap(),
		Expr::Unary(ExprUnary { op, expr, .. }) => match op {
			UnOp::Not(..) => !eval_unary_const_i32(ident, *expr),
			UnOp::Neg(..) => -eval_unary_const_i32(ident, *expr),
			_ => panic!("unsupported expression for `[number_key]` variant `{ident}`. Only `{{-,!}}<number>` allowed"),
		},
		_ => panic!("expected constant for `[number_key]` variant `{ident}`"),
	}
}
