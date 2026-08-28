mod service;
mod helpers;
mod id;
mod dynamic_service;

use crate::dynamic_service::DynamicService;
use crate::service::Service;
use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{parse_macro_input, parse_quote, GenericParam, ItemImpl, Type};
use syn::parse::{Parse, Parser};

/// Turns the type in the impl block into an injectable type.
/// It uses the `pub fn new() -> Self` method as the factory method.
///
/// This macro can also link a type to a dynamic trait type by placing it
/// above a trait implementation block.
/// 
/// # Examples
/// 
/// With a factory method. If you do not need initialization, you can use
/// `#[derive(Service)]` instead. 
/// 
/// You can also specify lock choice using #[service(MutexMarker)].
/// ```rust
/// #[service]
/// impl ConsoleLogger {
///     pub fn new() -> Self {
///         println!("ConsoleLogger initialized"); // print out when the logger is initialized
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
/// With a dynamic trait.
/// ```rust
/// // This allows ConsoleLogger to be used as dyn Logger.
/// // For example: injector.singleton_dyn::<ConsoleLogger, dyn Logger>();
/// 
/// #[service]
/// impl Logger for ConsoleLogger {
///     fn write(&mut self, message: &str) {
///         self.write(message);
///     }
/// }
/// ```
/// 
/// With dependency injection. This can be used with any type that implements
/// the `dilian::sync::Lock` trait.
/// ```rust
/// impl UserService {
///     pub fn new(dependency: <dilian::sync::Mutex as dilian::sync::Lock>::Lock<Dependency>) -> Self {
///         println!("UserService initialized"); // print out when the service is initialized
///         Self {
///             dependency,
///             dependency2
///         }
///     }
///     
///     pub fn get_user(&mut self, user_id: u32) -> Option<User> {
///         let dependency_guard = self.dependency.lock()
///             .unwrap();
/// 
///         // use dependency here...
///     }
/// }
/// ```
///
/// With dependency injection. You can also use the simplified type.
/// For custom locks this will replace the Lock portion of the identifier with Marker.
/// You'll need to import the custom marker if it cannot be found.
/// If you specify the full path to the lock type you won't need to import the marker type.
/// ```rust
/// use dilian::sync::MutexLock;
/// 
/// impl UserService {
///     pub fn new(dependency: MutexLock<Dependency>) -> Self {
///         println!("UserService initialized"); // print out when the service is initialized
///         Self {
///             dependency,
///             dependency2
///         }
///     }
///     
///     pub fn get_user(&mut self, user_id: u32) -> Option<User> {
///         let dependency_guard = self.dependency.lock()
///             .unwrap();
///
///         // use dependency here...
///     }
/// }
/// ```
/// 
/// With a generic lock. This is not recommended because it is harder to use,
/// but it is still possible.
/// ```rust
/// impl<L : dilian::sync::Lock> UserService<L> {
///     pub fn new(dependency: L::Lock<Dependency>) -> Self {
///         println!("UserService initialized"); // print out when the service is initialized
///         Self {
///             dependency,
///         }
///     }
///     
///     pub fn get_user(&mut self, user_id: u32) -> Option<User> 
///         where <L as dilian::sync::Lock>::Lock<Dependency> : dif::sync::Lockable<Dependency> // check type is lockable
///     {
///         let dependency_guard = self.dependency.write() // get dependency as write
///             .unwrap();
/// 
///         let dependency_guard = self.dependency.read() // get dependency as read
///             .unwrap();
///
///         // use dependency here...
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn service(args: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::ItemImpl);
    let ty = Type::parse.parse2(args.into()).ok();
    
    Service::from((input, ty))
        .into_token_stream()
        .into()
}

/// Turns a trait declaration into an injectable trait.
/// 
/// # Examples
/// 
/// ```rust
/// #[dynamic_service]
/// pub trait Logger : Send {
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

/// Turns a struct declaration into an injectable type.
/// All fields in the struct are resolved from the injector.
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
    
    Service::from((item_impl, None))
        .into_token_stream()
        .into()
}