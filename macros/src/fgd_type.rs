use crate::*;

pub(super) fn fgd_type_derive(input: DeriveInput) -> TokenStream {
	let DeriveInput { ident, data, .. } = input;

	let mut property_type_choices = Vec::new();

	let mut variant_idents = Vec::new();
	let mut variants = Vec::new();

	match data {
		Data::Enum(data) => {
			for Variant {
				ident: variant_ident,
				attrs,
				fields,
				..
			} in data.variants
			{
				if !fields.is_empty() {
					panic!("Only unit vectors supported, variant {variant_ident} has fields");
				}

				let mut doc = None;

				for attr in attrs {
					match attr.meta {
						Meta::NameValue(meta) => {
							if compare_path(&meta.path, "doc") {
								doc = Some(meta.value);
							}
						}
						_ => {}
					}
				}

				variants.push(variant_ident.to_string());

				let variant_literal_quoted = Literal::string(&format!("\"{variant_ident}\""));

				let title = doc.unwrap_or_else(|| {
					Expr::Lit(ExprLit {
						attrs: Vec::new(),
						lit: Lit::new(Literal::string(&variant_ident.to_string())),
					})
				});

				property_type_choices.push(quote! {
					(#variant_literal_quoted, #title),
				});

				variant_idents.push(variant_ident);
			}
		}
		_ => panic!("Currently only enums supported"),
	}

	let valid_variants = variants.join(", ");

	quote! {
		impl ::bevy_trenchbroom::fgd::FgdType for #ident {
			const PROPERTY_TYPE: ::bevy_trenchbroom::class::QuakeClassPropertyType = ::bevy_trenchbroom::class::QuakeClassPropertyType::Choices(&[
				#(#property_type_choices)*
			]);

			fn fgd_parse(input: &str) -> ::bevy_trenchbroom::anyhow::Result<Self> {
				match input {
					#(#variants => Ok(Self::#variant_idents),)*
					input => Err(::bevy_trenchbroom::anyhow::anyhow!(concat!("{input} isn't a valid ", stringify!(#ident), "! Valid variants are ", #valid_variants))),
				}
			}

			fn fgd_to_string(&self) -> String {
				match self {
					#(Self::#variant_idents => #variants.to_string(),)*
				}
			}
		}
	}
}
