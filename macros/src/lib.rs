use heck::*;
use quote::quote;
use syn::*;
use proc_macro2::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum QuakeClassType {
    Base,
    Point,
    Solid,
}

// #[derive(FromDeriveInput, Default)]
// #[darling(default, attributes(model, color, iconsprite, size, classname, geometry), forward_attrs(doc))]
#[derive(Default)]
struct Opts {
    model: Option<TokenStream>,
    color: Option<TokenStream>,
    iconsprite: Option<TokenStream>,
    size: Option<TokenStream>,
    geometry: Option<TokenStream>,
    classname: Option<TokenStream>,

    required: Option<TokenStream>,
    doc: Option<String>,
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

#[proc_macro_derive(PointClass, attributes(model, color, iconsprite, size, classname))]
pub fn point_class_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    class_derive(parse_macro_input!(input as DeriveInput), QuakeClassType::Point).into()
}

#[proc_macro_derive(SolidClass, attributes(geometry, classname))]
pub fn solid_class_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    class_derive(parse_macro_input!(input as DeriveInput), QuakeClassType::Solid).into()
}

#[proc_macro_derive(BaseClass, attributes(model, color, iconsprite, size, classname))]
pub fn base_class_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    class_derive(parse_macro_input!(input as DeriveInput), QuakeClassType::Base).into()
}

fn class_derive(input: DeriveInput, ty: QuakeClassType) -> TokenStream {
    let DeriveInput { ident, attrs, data, .. } = input;
    let ty_ident = Ident::new(&format!("{ty:?}"), Span::mixed_site());

    let mut opts = Opts::default();

    for attr in attrs {
        match attr.meta {
            Meta::NameValue(meta) => {
                if compare_path(&meta.path, "doc") {
                    let Expr::Lit(ExprLit { lit: Lit::Str(lit), .. }) = meta.value else { continue };

                    opts.doc = Some(lit.value().trim().to_string());
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
                } else if compare_path(&meta.path, "required") {
                    opts.required = Some(meta.tokens);
                }
            }
            Meta::Path(_) => {}
        }
    }

    

    let mut properties: Vec<TokenStream> = Vec::new();

    let spawn_constructor: TokenStream;

    match data {
        Data::Struct(data) => {
            let mut field_constructors = Vec::with_capacity(data.fields.len());
            let fields_type = FieldsType::from_fields(&data.fields);
            
            for (field_idx, field) in data.fields.into_iter().enumerate() {
                let ty = field.ty;
                let field_ident = field.ident.clone();
                let field_name = field.ident.map(|ident| ident.to_string())
                    .unwrap_or_else(|| field_idx.to_string());
                let field_ident_or_number = Ident::new(&field_name, Span::mixed_site());
                
                let mut doc = None;
                // let mut default = None;
                
                for attr in field.attrs {
                    match attr.meta {
                        Meta::Path(_) => {}
                        Meta::List(_) => {
                            // if compare_path(&meta.path, "default") {
                            //     default = Some(meta.tokens);
                            // }
                        }
                        Meta::NameValue(meta) => {
                            if compare_path(&meta.path, "doc") {
                                doc = Some(meta.value);
                            }
                        }
                    }
                }
                
                // let default = option(default.map(|default| quote! { Some(|| (#default).fgd_to_string()) }));
                let doc = doc.and_then(|expr| match expr {
                    Expr::Lit(ExprLit { lit: Lit::Str(lit), .. }) => Some(lit.value().trim().to_string()),
                    _ => None,
                });

                let description = option(doc);
                
                properties.push(quote! {
                    ::bevy_trenchbroom::class::QuakeClassProperty {
                        ty: <#ty as ::bevy_trenchbroom::fgd::FgdType>::PROPERTY_TYPE,
                        name: #field_name,
                        title: None,
                        description: #description,
                        // default_value: #default,
                        default_value: Some(|| ::bevy_trenchbroom::fgd::FgdType::fgd_to_string(&Self::default().#field_ident_or_number)),
                    },
                });

                let setter = field_ident.as_ref().map(|ident| quote! { #ident: });

                field_constructors.push(quote! {
                    #setter src_entity.get(#field_name)?,
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

    // let inventory_import = cfg!(feature = "auto_register").then(|| Ident::new("inventory", Span::mixed_site()));

    let inventory_submit = cfg!(feature = "auto_register").then(|| quote! {
        ::bevy_trenchbroom::inventory::submit! { <#ident as ::bevy_trenchbroom::class::QuakeClass>::ERASED_CLASS_INSTANCE }
    });

    let name = opts.classname
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

    // This is a naive approach, but the Component macro should handle making sure the input is valid anyway.
    let bases = opts.required.unwrap_or_default().into_iter().filter(|tree| matches!(tree, TokenTree::Ident(_)));

    let model = option(opts.model.map(|model| quote! { stringify!(#model) }));
    let color = option(opts.color.map(|color| quote! { stringify!(#color) }));
    let iconsprite = option(opts.iconsprite.map(|iconsprite| quote! { stringify!(#iconsprite) }));
    let size = option(opts.size.map(|size| quote! { stringify!(#size) }));

    let geometry_provider = opts.geometry.map(|geometry| {
        quote! {
            #[allow(unused)]
            fn geometry_provider(src_entity: &::bevy_trenchbroom::qmap::QuakeMapEntity) -> Option<::bevy_trenchbroom::geometry::GeometryProvider> {
                Some(#geometry)
            }
        }
    });
    
    quote! {
        impl ::bevy_trenchbroom::class::QuakeClass for #ident {
            const ERASED_CLASS_INSTANCE: &::bevy_trenchbroom::class::ErasedQuakeClass = &::bevy_trenchbroom::class::ErasedQuakeClass::of::<Self>();
            const CLASS_INFO: ::bevy_trenchbroom::class::QuakeClassInfo = ::bevy_trenchbroom::class::QuakeClassInfo {
                ty: ::bevy_trenchbroom::class::QuakeClassType::#ty_ident,
                name: #name,
                description: #description,
                base: &[#(<#bases as ::bevy_trenchbroom::class::QuakeClass>::ERASED_CLASS_INSTANCE),*],
        
                model: #model,
                color: #color,
                iconsprite: #iconsprite,
                size: #size,

                properties: &[#(#properties)*],
            };

            #geometry_provider

            #[allow(unused)]
            fn class_spawn(config: &::bevy_trenchbroom::config::TrenchBroomConfig, src_entity: &::bevy_trenchbroom::qmap::QuakeMapEntity, entity: &mut ::bevy::ecs::world::EntityWorldMut) -> ::bevy_trenchbroom::anyhow::Result<()> {
                entity.insert(#spawn_constructor);
                Ok(())
            }
        }

        #inventory_submit
    }
}

fn compare_path(path: &Path, s: &str) -> bool {
    path.segments == [PathSegment { ident: Ident::new(s, Span::mixed_site()), arguments: PathArguments::None }].into_iter().collect()
}

fn option(value: Option<impl quote::ToTokens>) -> TokenStream {
    match value {
        Some(value) => quote! { Some(#value) },
        None => quote!(None),
    }
}