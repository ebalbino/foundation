//! Procedural macros for `foundation`.
//!
//! This crate currently exposes `#[derive(Reflect)]`, which generates an
//! implementation of [`foundation::reflect::Introspectable`] for struct types.
//!
//! The generated description:
//!
//! - uses the Rust type name as [`foundation::reflect::Description::name`]
//! - records `size_of::<Self>()` as the reflected size
//! - emits [`foundation::reflect::Value::Composite`] with one
//!   [`foundation::reflect::Field`] per struct field
//! - computes byte offsets with `MaybeUninit<Self>` and raw pointer arithmetic
//!
//! Supported inputs:
//!
//! - named-field structs
//! - tuple structs
//! - unit structs
//!
//! Unsupported inputs:
//!
//! - enums
//! - unions
//!
//! Every field type must already implement
//! [`foundation::reflect::Introspectable`].

use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, Ident, parse_macro_input};

/// Derives [`foundation::reflect::Introspectable`] for a struct.
///
/// The generated implementation describes the struct as a composite type and
/// records each field's name, reflected type, and byte offset within the struct.
///
/// For tuple structs, field names are emitted as `"0"`, `"1"`, and so on.
/// For unit structs, the generated field list is empty.
///
/// This derive only supports structs. Enums and unions produce a compile error.
///
/// # Requirements
///
/// Every field type must implement `foundation::reflect::Introspectable`.
///
/// # Layout Notes
///
/// Field offsets are computed from the compiler's actual layout for `Self` at
/// compile time using `MaybeUninit<Self>` and address calculation. If the struct
/// layout changes, the generated description changes with it.
#[proc_macro_derive(Reflect)]
pub fn derive_reflect(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let ty_ident = input.ident;

    let expanded = match input.data {
        Data::Struct(s) => derive_struct(&ty_ident, s.fields),
        Data::Enum(_) => {
            // You *can* support enums, but the representation story gets tricky fast.
            // Start
            // with structs; add enums later once you decide what "composite" means there.
            syn::Error::new_spanned(ty_ident, "Reflect derive currently supports structs only")
                .to_compile_error()
        }
        Data::Union(_) => syn::Error::new_spanned(ty_ident, "Reflect cannot be derived for unions")
            .to_compile_error(),
    };

    expanded.into()
}

fn derive_struct(ty_ident: &Ident, fields: Fields) -> proc_macro2::TokenStream {
    // Generate field descriptors that mirror the compiler's actual field layout.
    let mut field_inits = Vec::new();

    match fields {
        Fields::Named(named) => {
            for f in named.named {
                let field_ident = f.ident.expect("named field ident");
                let field_name = field_ident.to_string();
                let field_ty = f.ty;

                // Compute the field offset from a MaybeUninit<Self> base pointer without
                // reading from uninitialized memory.
                let offset_expr = quote!({
                    let uninit = ::core::mem::MaybeUninit::<Self>::uninit();
                    let base = uninit.as_ptr();
                    // SAFETY: We never read, we only take addresses.
                    unsafe {
                        let field_ptr = ::core::ptr::addr_of!((*base).#field_ident);
                        (field_ptr as usize) - (base as usize)
                    }
                });

                field_inits.push(quote!({
                    let desc = <#field_ty as ::foundation::reflect::Introspectable>::description();
                    ::foundation::reflect::Field {
                        desc,
                        name: #field_name,
                        offset: #offset_expr,
                    }
                }));
            }
        }
        Fields::Unnamed(unnamed) => {
            for (idx, f) in unnamed.unnamed.into_iter().enumerate() {
                let index = syn::Index::from(idx);
                let field_name = idx.to_string();
                let field_ty = f.ty;

                let offset_expr = quote!({
                    let uninit = ::core::mem::MaybeUninit::<Self>::uninit();
                    let base = uninit.as_ptr();
                    unsafe {
                        let field_ptr = ::core::ptr::addr_of!((*base).#index);
                        (field_ptr as usize) - (base as usize)
                    }
                });

                field_inits.push(quote!({
                    let desc = <#field_ty as ::foundation::reflect::Introspectable>::description();
                    ::foundation::reflect::Field {
                        desc,
                        name: #field_name,
                        offset: #offset_expr,
                    }
                }));
            }
        }
        Fields::Unit => {
            // no fields
        }
    }

    // Reflect the Rust type name directly into the runtime description.
    let ty_name = ty_ident.to_string();

    quote! {
        impl ::foundation::reflect::Introspectable for #ty_ident {
            fn description() -> ::foundation::reflect::Description {
                let mut fields: ::std::vec::Vec<::foundation::reflect::Field> = ::std::vec::Vec::new();
                #( fields.push(#field_inits); )*

                ::foundation::reflect::Description {
                    name: #ty_name,
                    size: ::core::mem::size_of::<Self>(),
                    value: ::foundation::reflect::Value::Composite { fields },
                }
            }
        }
    }
}
