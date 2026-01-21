//! Proc macros for `bevy_expected_components`.
//!
//! This crate provides the `#[derive(ExpectComponents)]` macro. You should not
//! depend on this crate directly; instead use `bevy_expected_components` which
//! re-exports the macro.

use proc_macro::TokenStream;
use quote::quote;
use syn::punctuated::Punctuated;
use syn::{parse_macro_input, DeriveInput, Path, Token};

/// Derive macro for generating `ExpectComponents` implementation.
///
/// Use with the `#[expect(...)]` attribute to specify which components must
/// exist when this component is inserted.
///
/// # Example
///
/// ```rust,ignore
/// use bevy::prelude::*;
/// use bevy_expected_components::prelude::*;
///
/// #[derive(Component, ExpectComponents)]
/// #[expect(Transform, Velocity)]
/// struct PhysicsBody;
/// ```
///
/// # Multiple Attributes
///
/// You can use multiple `#[expect(...)]` attributes:
///
/// ```rust,ignore
/// #[derive(Component, ExpectComponents)]
/// #[expect(Transform)]
/// #[expect(Velocity)]
/// struct PhysicsBody;
/// ```
///
/// # Qualified Paths
///
/// Full paths are supported:
///
/// ```rust,ignore
/// #[derive(Component, ExpectComponents)]
/// #[expect(bevy::transform::components::Transform)]
/// struct MyComponent;
/// ```
#[proc_macro_derive(ExpectComponents, attributes(expect))]
pub fn derive_expect_components(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    // Extract component paths from all #[expect(...)] attributes
    let expected: Vec<Path> = input
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident("expect"))
        .flat_map(|attr| {
            attr.parse_args_with(Punctuated::<Path, Token![,]>::parse_terminated)
                .unwrap_or_default()
        })
        .collect();

    if expected.is_empty() {
        return syn::Error::new_spanned(
            &input.ident,
            "ExpectComponents derive requires at least one #[expect(Component)] attribute",
        )
        .to_compile_error()
        .into();
    }

    // Generate TypeId expressions for each expected component
    let type_ids = expected.iter().map(|p| {
        quote! { ::std::any::TypeId::of::<#p>() }
    });

    // Generate type name expressions for error messages
    let type_names = expected.iter().map(|p| {
        quote! { ::std::any::type_name::<#p>() }
    });

    let expanded = quote! {
        impl ::bevy_expected_components::ExpectComponents for #name {
            fn expected_components() -> &'static [::std::any::TypeId] {
                static IDS: ::std::sync::OnceLock<::std::vec::Vec<::std::any::TypeId>> =
                    ::std::sync::OnceLock::new();
                IDS.get_or_init(|| ::std::vec![#(#type_ids),*]).as_slice()
            }

            fn expected_component_names() -> &'static [&'static str] {
                static NAMES: ::std::sync::OnceLock<::std::vec::Vec<&'static str>> =
                    ::std::sync::OnceLock::new();
                NAMES.get_or_init(|| ::std::vec![#(#type_names),*]).as_slice()
            }
        }

        ::bevy_expected_components::inventory::submit! {
            ::bevy_expected_components::ExpectRegistration::of::<#name>()
        }
    };

    expanded.into()
}
