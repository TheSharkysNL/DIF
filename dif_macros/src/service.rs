use crate::helpers::{get_associated_generic_type, get_generic, get_generic_type, get_iterator_impl, get_method, match_path_type, returns_self};
use quote::{quote, ToTokens};
use syn::spanned::Spanned;
use syn::FnArg;
use syn::__private::TokenStream2;

pub struct Service {
    item_impl: syn::ItemImpl,
}

pub struct FromInjectorImpl<'a> {
    new_method: Option<&'a syn::ImplItemFn>,
    ty: &'a syn::Type,
    generics: &'a syn::Generics,
}

pub struct DynamicInjectableImpl<'a> {
    item_impl: &'a syn::ItemImpl,
    generics: &'a syn::Generics,
}

impl From<syn::ItemImpl> for Service {
    fn from(item_impl: syn::ItemImpl) -> Self {
        Self {
            item_impl,
        }
    }
}

impl ToTokens for Service {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let new_method = get_method(&self.item_impl, "new")
            .ok();
        let generics = &self.item_impl.generics;
        
        let _impl = if let Some(_trait) = &self.item_impl.trait_ {
            let dyn_injectable_impl = DynamicInjectableImpl {
                item_impl: &self.item_impl,
                generics,
            };
            
            dyn_injectable_impl.into_token_stream()
        } else {
            let from_injector_impl = FromInjectorImpl {
                new_method,
                ty: &self.item_impl.self_ty,
                generics
            };
            
            // let unique_id_impl = UniqueIdImpl::new(&self.item_impl.self_ty, generics);
            
            quote! {
                #from_injector_impl
                
            }
        };
        
        let original_impl = &self.item_impl;
        let tree = quote! {
            #_impl
            
            #original_impl
        };
        
        tree.to_tokens(tokens);
    }
}

impl ToTokens for FromInjectorImpl<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let ty = self.ty;
        let generics = self.generics;
        
        let (body, injections) = if let Some(new_method) = self.new_method {
            if !returns_self(new_method) {
                let error = syn::Error::new(new_method.sig.output.span(), "'new' function must return 'Self'.").to_compile_error();
                error.to_tokens(tokens);
                return;
            }
            
            let args = new_method.sig.inputs.iter()
                .filter_map(|arg| match arg {
                    FnArg::Typed(ty) => Some(&ty.pat),
                    FnArg::Receiver(_) => None,
                });
            
            let mut block = quote! { Self::new(#(#args),*) };
            if new_method.sig.unsafety.is_some() {
                block = quote! {
                    unsafe {
                        #block
                    }
                }
            }

            let injections = new_method.sig.inputs
                .iter()
                .map(arg_to_injection)
                .collect::<Vec<_>>();
            
            (block, injections)
        } else {
            (quote! {
                Self {}
            }, Vec::new())  
        };

        let tree = quote! {
            impl #generics dif::FromInjector for #ty {
                fn from_injector(injector: &dif::Injector) -> Self {
                    #(#injections)*
                    
                    #body
                }
            }
        };

        tree.to_tokens(tokens);
    }
}

fn arg_to_injection(arg: &FnArg) -> TokenStream2 {
    match arg {
        FnArg::Receiver(_) => {
            let error = syn::Error::new(arg.span(), "Didn't expect the 'self' keyword. The method must be a static method.");
            error.to_compile_error()
        },
        FnArg::Typed(arg) => {
            let name = arg.pat.as_ref();
            let ty = arg.ty.as_ref();
            
            if match_path_type("dif::sync::InjectorLock", ty) {
                let injection_type = get_generic(ty, "dif::sync::InjectorLock<T>");
                quote! {
                    let #name = injector.get::<#injection_type>().expect(concat!("The type: ", stringify!(#injection_type), " could not be resolved from the injection container."));
                }
            } else if match_path_type("dif::sync::InjectorLockDyn", ty) {
                let injection_type = get_generic(ty, "dif::sync::InjectorLockDyn<T>");
                quote! {
                    let #name = injector.get_dyn::<#injection_type>().expect(concat!("The type: ", stringify!(#injection_type), " could not be resolved from the injection container."));
                }
            } else if let Some(result) = get_iterator_impl(ty) {
                match result { 
                    Ok(iterator) => {
                        let inner_argument = get_associated_generic_type(&iterator.path, "std::iter::Iterator<Item = T>")
                            .and_then(|x| get_generic_type(x, "dif::sync::InjectorLockDyn<T>"));
                        
                        let inner_argument = match inner_argument {
                            Ok(x) => x,
                            Err(e) => {
                                return e.to_compile_error();
                            }
                        };
                        
                        quote! {
                            let #name = injector.get_list::<#inner_argument>().expect(concat!("The type: ", stringify!(#inner_argument), " could not be resolved from the injection container."));
                        }
                    },
                    Err(error) => error.to_compile_error(),
                }
            } else {
                quote! { 
                    let #name = injector.produce::<#ty>(); 
                }
            }
        }
    }
}

impl ToTokens for DynamicInjectableImpl<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let _trait = self.item_impl.trait_.as_ref().unwrap();
        let _trait = &_trait.1;
        let ty = self.item_impl.self_ty.as_ref();
        let generics = self.generics;
        
        let tree = quote! {
            impl #generics dif::DynamicInjectable<dyn #_trait> for #ty {
                fn into_dynamic(self) -> std::sync::Arc<dif::sync::LockOrCell<dyn #_trait>> {
                    std::sync::Arc::new(dif::sync::LockOrCell::new(self))
                }
            }
        };
        
        tree.to_tokens(tokens);
    }
}