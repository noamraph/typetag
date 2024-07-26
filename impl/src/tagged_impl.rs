use crate::{ImplArgs, Mode};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_quote, Error, ItemImpl, Type, TypePath};

pub(crate) fn expand(args: ImplArgs, mut input: ItemImpl, mode: Mode) -> TokenStream {
    if mode.de && !input.generics.params.is_empty() {
        let msg = "deserialization of generic impls is not supported yet; \
                   use #[typetag::serialize] to generate serialization only";
        return Error::new_spanned(input.generics, msg).to_compile_error();
    }

    let name = match args.name {
        Some(name) => quote!(#name),
        None => match type_name(&input.self_ty) {
            Some(name) => quote!(#name),
            None => {
                let msg = "use #[typetag::serde(name = \"...\")] to specify a unique name";
                return Error::new_spanned(&input.self_ty, msg).to_compile_error();
            }
        },
    };

    augment_impl(&mut input, &name, mode, args.write_tag);

    let object = &input.trait_.as_ref().unwrap().1;
    let this = &input.self_ty;

    let mut expanded = quote! {
        #input
    };

    if mode.de {
        expanded.extend(quote! {
            typetag::__private::inventory::submit! {
                <dyn #object>::typetag_register(
                    #name,
                    (|deserializer| typetag::__private::Result::Ok(
                        typetag::__private::Box::new(
                            typetag::__private::erased_serde::deserialize::<#this>(deserializer)?
                        ),
                    )) as typetag::__private::DeserializeFn<<dyn #object as typetag::__private::Strictest>::Object>,
                )
            }
        });
    }

    expanded
}

fn augment_impl(input: &mut ItemImpl, name: &TokenStream, mode: Mode, write_tag: bool) {
    if mode.ser {
        let write_tag_token = if write_tag {
            format_ident!("true")
        } else {
            format_ident!("false")
        };
        input.items.push(parse_quote! {
            #[doc(hidden)]
            fn typetag_name(&self) -> &'static str {
                #name
            }
            // #[doc(hidden)]
            // fn typetag_write_tag(&self) -> bool {
            //     true
            // }
        });
        // if write_tag {
        //     input.items.push(parse_quote! {
        //         #[doc(hidden)]
        //         fn typetag_is_write_tag(&self) -> bool {
        //             true
        //         }
        //     });
        // } else {
        //     input.items.push(parse_quote! {
        //         #[doc(hidden)]
        //         fn typetag_is_write_tag(&self) -> bool {
        //             false
        //         }
        //     });
        // }
    }

    if mode.de {
        input.items.push(parse_quote! {
            #[doc(hidden)]
            fn typetag_deserialize(&self) {}
        });
    }
}

fn type_name(mut ty: &Type) -> Option<String> {
    loop {
        match ty {
            Type::Path(TypePath { qself: None, path }) => {
                return Some(path.segments.last().unwrap().ident.to_string());
            }
            Type::Group(group) => {
                ty = &group.elem;
            }
            _ => return None,
        }
    }
}
