mod service;
mod helpers;
mod id;
mod dynamic_service;

use crate::dynamic_service::DynamicService;
use crate::service::Service;
use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{parse_macro_input, parse_quote, GenericParam, ItemImpl};

/// Turns the impl type passed into a injectable type. 
/// Then it uses the `pub fn new() -> Self` method as a factory method
/// 
/// This macro can also be used to link types to their dynamic types. 
/// This can be done by adding this macro above a trait implement block
/// 
/// # Examples
/// 
/// With factory method
/// ```rust
/// #[service]
/// impl ConsoleLogger {
///     pub fn new() -> Self {
///         println!("ConsoleLogger intialized"); // print out when the logger is initialized
///         Self {
///         }
///     }
///     
///     pub fn write(&mut self, message: &str) {
///         println!("{}", message);
///     }
/// }
/// ```
/// 
/// With dynamic traits
/// ```rust
/// // this way the ConsoleLogger type can be used with dyn Logger. 
/// // for example injector.singleton_dyn::<ConsoleLogger, dyn Logger>();
/// 
/// #[service]
/// impl Logger for ConsoleLogger {
///     fn write(&mut self, message: &str) {
///         self.write(message);
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn service(args: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::ItemImpl);
    parse_macro_input!(args as syn::parse::Nothing);
    
    Service::from(input)
        .into_token_stream()
        .into()
}

/// Turns a trait declaration into a trait that can be injected.
/// 
/// # Examples
/// 
/// ```rust
/// #[dynamic_service]
/// pub trait Logger : Send + Sync {
///     fn write(&mut self, message: &str);
/// }
/// ```
#[proc_macro_attribute]
pub fn dynamic_service(args: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::ItemTrait);
    parse_macro_input!(args as syn::parse::Nothing);
    
    DynamicService::from(input)
        .into_token_stream()
        .into()
}

/// Turns a struct declaration into a type that can be injected. 
/// This will inject all the fields within the struct. 
/// 
/// ```rust
/// #[derive(Service)]
/// pub struct ServiceWithLogger {
///     logger: InjectorLockDyn<dyn Logger>,
/// }
/// ```
#[proc_macro_derive(Service)]
pub fn service_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::ItemStruct);
    let name = input.ident;
    let generics = input.generics;
    
    let named_generics = generics.params.iter()
        .map(|param| match param {
            GenericParam::Lifetime(l) => l.lifetime.ident.to_token_stream(),
            GenericParam::Type(t) => t.ident.to_token_stream(),
            GenericParam::Const(c) => c.ident.to_token_stream(),
        })
        .collect::<Vec<_>>();
    
    let parameters = input.fields
        .iter()
        .map(|field| {
            let name = field.ident.as_ref();
            let ty = &field.ty;
            quote! { #name: #ty }
        });
    
    let create = input.fields.iter().map(|field| &field.ident);
    
    let item_impl: ItemImpl = parse_quote! {
        impl #generics  #name < #(#named_generics),* > {
            pub fn new(#(#parameters,)*) -> Self {
                Self {
                    #(#create,)*
                }
            }
        }
    };
    
    Service::from(item_impl)
        .into_token_stream()
        .into()
}