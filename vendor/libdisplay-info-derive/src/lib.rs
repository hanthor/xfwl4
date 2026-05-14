use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{
    braced, parse_macro_input, Attribute, Expr, Ident, Path, Result, Token, Type, Visibility,
};

struct FFIFrom {
    path: Path,
    ident: Ident,
    item: FFIItem,
    wrap: bool,
}

enum FFIItem {
    Struct(Punctuated<FFIStructField, Token![,]>),
    Enum(Punctuated<FFIEnumVariant, Token![,]>),
}

struct FFIStructField {
    ident: Ident,
    ty: Type,
    cast_as: Option<Type>,
    optional: Option<Expr>,
    ptr_deref: bool,
    span: Span,
}

struct FFIEnumVariant {
    ident: Ident,
    discriminant: Option<Expr>,
}

#[proc_macro_derive(FFIFrom, attributes(ffi, cast_as, other, optional, ptr_deref, wrap))]
pub fn ffi_from_fn(input: TokenStream) -> TokenStream {
    let FFIFrom {
        path,
        ident,
        item,
        wrap,
    } = parse_macro_input!(input as FFIFrom);

    match item {
        FFIItem::Struct(fields) => {
            let mapped_fields: Result<Vec<_>> = fields
                .into_iter()
                .map(|field| {
                    let ident = field.ident;

                    let val = if let Type::Array(ref array) = field.ty {
                        let elem = &array.elem;

                        let num = match array.len {
                            syn::Expr::Lit(ref lit) => match lit.lit {
                                syn::Lit::Int(ref int) => int.base10_parse::<usize>()?,
                                _ => {
                                    return Err(syn::Error::new(
                                        field.span,
                                        "only int literals are supported",
                                    ))
                                }
                            },
                            _ => {
                                return Err(syn::Error::new(
                                    field.span,
                                    "only int literals are supported",
                                ))
                            }
                        };

                        let vals = (0..num)
                            .map(|idx| {
                                if let Some(cast_as) = field.cast_as.as_ref() {
                                    quote! {
                                        #elem::from(value.#ident[#idx] as #cast_as)
                                    }
                                } else {
                                    quote! {
                                        #elem::from(value.#ident[#idx])
                                    }
                                }
                            })
                            .collect::<Vec<_>>();

                        quote! {
                            [
                                #(#vals ,)*
                            ]
                        }
                    } else {
                        let ty = field.ty;
                        let is_option = matches!(ty, Type::Path(ref path) if path.path.segments[0].ident == "Option");

                        if !is_option && field.optional.is_some() {
                            return Err(syn::Error::new(
                                field.span,
                                "#[optional()] only allowed for Option fields",
                            ));
                        }

                        if is_option {
                            let Type::Path(ref path) = ty else {
                                unreachable!()
                            };

                            let option_type = match path.path.segments[0].arguments {
                                syn::PathArguments::AngleBracketed(ref bracketed) => {
                                    let arg = bracketed.args.first().ok_or_else(|| {
                                        syn::Error::new(
                                            field.span,
                                            "expected a single bracketed type",
                                        )
                                    })?;

                                    match arg {
                                        syn::GenericArgument::Type(ty) => ty,
                                        _ => {
                                            return Err(syn::Error::new(
                                                field.span,
                                                "expected a single bracketed type",
                                            ))
                                        }
                                    }
                                }
                                _ => {
                                    return Err(syn::Error::new(
                                        field.span,
                                        "expected a single bracketed type",
                                    ))
                                }
                            };

                            if let Some(optional) = field.optional {
                                if let Some(cast_as) = field.cast_as.as_ref() {
                                    quote! {
                                        if value.#ident == #optional {
                                            None
                                        } else {
                                            Some(#option_type::from(value.#ident as #cast_as))
                                        }
                                    }
                                } else {
                                    quote! {
                                        if value.#ident == #optional {
                                            None
                                        } else {
                                            Some(#option_type::from(value.#ident))
                                        }
                                    }
                                }
                            } else if field.ptr_deref {
                                if let Some(cast_as) = field.cast_as.as_ref() {
                                    quote! {
                                        if value.#ident.is_null() {
                                            None
                                        } else {
                                            Some(#option_type::from(unsafe { *(value.#ident as #cast_as) }))
                                        }
                                    }
                                } else {
                                    quote! {
                                        if value.#ident.is_null() {
                                            None
                                        } else {
                                            Some(#option_type::from(unsafe { *value.#ident }))
                                        }
                                    }
                                }
                            } else {
                                return Err(syn::Error::new(
                                    field.span,
                                    "#[optional()] is required for non pointer types",
                                ));
                            }
                        } else if let Some(cast_as) = field.cast_as.as_ref() {
                            quote! {
                                #ty::from(value.#ident as #cast_as)
                            }
                        } else {
                            quote! {
                                #ty::from(value.#ident)
                            }
                        }
                    };

                    Ok(quote! {
                        #ident: #val
                    })
                })
                .collect();

            let mapped_fields = match mapped_fields {
                Ok(fields) => fields,
                Err(err) => return TokenStream::from(err.into_compile_error()),
            };

            let expanded = if wrap {
                let ref_ident = quote::format_ident!("{}Ref", ident);
                let ref_doc = format!("Reference for [`{}`]", ident);
                let inner_doc = format!("Access the inner [`{}`]", ident);

                quote! {
                    impl From<#path> for #ident {
                        #[inline]
                        fn from(value: #path) -> #ident {
                            Self {
                                #(#mapped_fields ,)*
                            }
                        }
                    }

                    #[doc = #ref_doc]
                    #[derive(Debug)]
                    #[repr(transparent)]
                    pub struct #ref_ident(*const #path);

                    impl #ref_ident {
                        #[doc = #inner_doc]
                        pub fn inner(&self) -> #ident {
                            #ident::from(unsafe { *self.0 })
                        }
                    }

                    impl #ident {
                        pub(crate) fn from_ptr(ptr: *const #path) -> Option<#ident> {
                            if ptr.is_null() {
                                None
                            } else {
                                Some(Self::from(unsafe { *ptr }))
                            }
                        }
                    }

                    impl #ref_ident {
                        pub(crate) fn from_ptr(ptr: *const #path) -> Option<#ref_ident> {
                            if ptr.is_null() {
                                None
                            } else {
                                Some(Self(ptr))
                            }
                        }
                    }
                }
            } else {
                quote! {
                        impl From<#path> for #ident {
                            #[inline]
                            fn from(value: #path) -> #ident {
                                Self {
                                    #(#mapped_fields ,)*
                                }
                            }
                        }

                        impl #ident {
                            pub(crate) fn from_ptr(ptr: *const #path) -> Option<#ident> {
                                if ptr.is_null() {
                                    None
                                } else {
                                    Some(Self::from(unsafe { *ptr }))
                                }
                            }
                        }
                }
            };

            TokenStream::from(expanded)
        }
        FFIItem::Enum(variants) => {
            let mapped_variants = variants
                .iter()
                .filter(|variant| variant.discriminant.is_some())
                .map(|variant| {
                    let variant_ident = &variant.ident;
                    let disc = &variant.discriminant;
                    quote! {
                        #disc => #ident::#variant_ident
                    }
                })
                .collect::<Vec<_>>();

            let other = variants
                .iter()
                .find(|variant| variant.discriminant.is_none());

            let tail = if let Some(other) = other {
                let other_ident = &other.ident;
                quote! {
                    _ => #ident::#other_ident,
                }
            } else {
                quote! {
                    _ => unreachable!(),
                }
            };

            let expanded = quote! {
                impl From<#path> for #ident {
                    #[inline]
                    fn from(value: #path) -> #ident {
                        match value {
                            #(#mapped_variants ,)*
                            #tail
                        }
                    }
                }
            };

            TokenStream::from(expanded)
        }
    }
}

impl Parse for FFIFrom {
    fn parse(input: ParseStream) -> Result<Self> {
        let attributes = input.call(Attribute::parse_outer)?;
        let path = attributes
            .iter()
            .find(|attr| attr.path().segments[0].ident == "ffi")
            .ok_or_else(|| input.error("#[ffi()] attribute is required"))?
            .parse_args::<Path>()?;

        let wrap = attributes
            .iter()
            .any(|attr| attr.path().segments[0].ident == "wrap");

        input.parse::<Visibility>()?;

        let lookahead = input.lookahead1();
        if lookahead.peek(Token![struct]) {
            input.parse::<Token![struct]>()?;
            let ident = input.parse::<Ident>()?;
            let fields = {
                let content;
                braced!(content in input);
                content.parse_terminated(parse_ffi_struct_field, Token![,])?
            };
            Ok(FFIFrom {
                path,
                ident,
                item: FFIItem::Struct(fields),
                wrap,
            })
        } else if lookahead.peek(Token![enum]) {
            input.parse::<Token![enum]>()?;
            let ident = input.parse::<Ident>()?;
            let variants = {
                let content;
                braced!(content in input);
                content.parse_terminated(parse_ffi_enum_variant, Token![,])?
            };

            if variants
                .iter()
                .filter(|attr| attr.discriminant.is_none())
                .count()
                > 1
            {
                return Err(input.error("only a single variant marked with #[other] is supported"));
            }

            Ok(FFIFrom {
                path,
                ident,
                item: FFIItem::Enum(variants),
                wrap,
            })
        } else {
            Err(lookahead.error())
        }
    }
}

fn parse_ffi_struct_field(input: ParseStream) -> Result<FFIStructField> {
    let attributes = input.call(Attribute::parse_outer)?;
    let cast_as = attributes
        .iter()
        .find(|attr| attr.path().segments[0].ident == "cast_as");

    let cast_as = if let Some(cast_as) = cast_as {
        Some(cast_as.parse_args::<Type>()?)
    } else {
        None
    };

    let optional = attributes
        .iter()
        .find(|attr| attr.path().segments[0].ident == "optional");

    let ptr_deref = attributes
        .iter()
        .any(|attr| attr.path().segments[0].ident == "ptr_deref");

    let optional = if let Some(optional) = optional {
        Some(optional.parse_args::<Expr>()?)
    } else {
        None
    };

    input.parse::<Visibility>()?;
    let ident = input.parse::<Ident>()?;
    input.parse::<Token![:]>()?;
    let ty = input.parse::<Type>()?;

    Ok(FFIStructField {
        ident,
        ty,
        cast_as,
        optional,
        ptr_deref,
        span: input.span(),
    })
}

fn parse_ffi_enum_variant(input: ParseStream) -> Result<FFIEnumVariant> {
    let is_other = input
        .call(Attribute::parse_outer)?
        .into_iter()
        .any(|attr| attr.path().segments[0].ident == "other");
    input.parse::<Visibility>()?;
    let ident: Ident = input.parse()?;
    let discriminant = if is_other {
        None
    } else {
        input.parse::<Token![=]>()?;
        Some(input.parse::<Expr>()?)
    };
    Ok(FFIEnumVariant {
        ident,
        discriminant,
    })
}
