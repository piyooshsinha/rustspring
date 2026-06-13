//! Derive macros for rustspring.

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields, GenericArgument, PathArguments, Type};

/// Derive `rustspring::Component` — constructor-based dependency injection,
/// the `@Component` of the framework.
///
/// Each `Arc<T>` field is a dependency, resolved from the `AppContext` at
/// startup; any other field is initialized with `Default::default()`.
///
/// ```ignore
/// #[derive(Component)]
/// struct UserService {
///     pool: Arc<PgPool>,            // dependency: resolved from the context
///     greeter: Arc<GreetingService>, // dependency: another component
///     cache: RwLock<Vec<User>>,      // plain state: Default::default()
/// }
///
/// Application::new()
///     .manage(GreetingService::new())
///     .component::<UserService>()    // constructed and registered in order
/// ```
#[proc_macro_derive(Component, attributes(component))]
pub fn derive_component(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let fields = match &input.data {
        Data::Struct(s) => match &s.fields {
            Fields::Named(named) => &named.named,
            Fields::Unit => {
                return component_impl(name, quote! { Ok(Self) }).into();
            }
            Fields::Unnamed(_) => {
                return syn::Error::new_spanned(
                    name,
                    "#[derive(Component)] requires named fields (or a unit struct)",
                )
                .to_compile_error()
                .into();
            }
        },
        _ => {
            return syn::Error::new_spanned(name, "#[derive(Component)] only supports structs")
                .to_compile_error()
                .into();
        }
    };

    let inits = fields.iter().map(|field| {
        let field_name = field.ident.as_ref().unwrap();
        match arc_inner_type(&field.ty) {
            Some(dep) => quote! {
                #field_name: ctx.get::<#dep>().ok_or_else(|| {
                    ::rustspring::ComponentError::missing(
                        ::std::any::type_name::<#name>(),
                        ::std::any::type_name::<#dep>(),
                    )
                })?
            },
            None => quote! { #field_name: ::core::default::Default::default() },
        }
    });

    component_impl(name, quote! { Ok(Self { #(#inits),* }) }).into()
}

fn component_impl(name: &syn::Ident, body: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    quote! {
        impl ::rustspring::Component for #name {
            fn construct(
                ctx: &::rustspring::AppContext,
            ) -> ::core::result::Result<Self, ::rustspring::ComponentError> {
                #body
            }
        }
    }
}

/// If `ty` is `Arc<T>` (by path, e.g. `Arc<..>` / `std::sync::Arc<..>`),
/// return `T`.
fn arc_inner_type(ty: &Type) -> Option<&Type> {
    let Type::Path(type_path) = ty else {
        return None;
    };
    let segment = type_path.path.segments.last()?;
    if segment.ident != "Arc" {
        return None;
    }
    let PathArguments::AngleBracketed(args) = &segment.arguments else {
        return None;
    };
    match args.args.first()? {
        GenericArgument::Type(inner) => Some(inner),
        _ => None,
    }
}
