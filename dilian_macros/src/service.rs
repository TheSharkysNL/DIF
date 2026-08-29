use crate::helpers::{get_associated_generic_type, get_generic_path, get_iterator_impl, get_method, match_path, returns_self};
use quote::{quote, ToTokens};
use syn::spanned::Spanned;
use syn::{parse_quote, Error, FnArg, GenericArgument, GenericParam, Generics, Ident, ImplItem, Pat, PatType, Type, TypeParamBound};
use syn::__private::{Span, TokenStream2};
use syn::parse::{Parse, Parser};

pub struct Service {
    item_impl: syn::ItemImpl,
    lock_type: Option<Type>,
}

pub struct FromInjectorImpl<'a> {
    new_method: Option<&'a syn::ImplItemFn>,
    ty: &'a syn::Type,
    generics: &'a syn::Generics,
    lock_type: Option<&'a Type>,
}

pub struct DynamicInjectableImpl<'a> {
    item_impl: &'a syn::ItemImpl,
    generics: &'a syn::Generics,
}

pub struct ParameterType<'a> {
    lock_name: Option<Type>,
    ty: &'a Type,
    is_iterator: bool,
}

impl From<(syn::ItemImpl, Option<Type>)> for Service {
    fn from((item_impl, lock_type): (syn::ItemImpl, Option<Type>)) -> Self {
        Self {
            item_impl,
            lock_type
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
                generics,
                lock_type: self.lock_type.as_ref(),
            };
            
            from_injector_impl.into_token_stream()
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
        let mut generics = self.generics.clone();
        
        let (body, injections, lock_type) = if let Some(new_method) = self.new_method {
            if !returns_self(new_method) {
                let error = syn::Error::new(new_method.sig.output.span(), "'new' function must return 'Self'.").to_compile_error();
                error.to_tokens(tokens);
                return;
            }
            
            let args = new_method.sig.inputs.iter()
                .filter_map(|arg| match arg {
                    FnArg::Typed(PatType { pat, .. }) => if let Pat::Ident(pat) = pat.as_ref() { Some(&pat.ident) } else { None },
                    _ => None,
                });
            
            let mut block = quote! { Self::new(#(#args),*) };
            if new_method.sig.unsafety.is_some() {
                block = quote! {
                    unsafe {
                        #block
                    }
                }
            }
            
            let mut lock_type = None;
            let injections = new_method.sig.inputs
                .iter()
                .map(|arg| match arg {
                    FnArg::Receiver(_) => Err(Error::new(arg.span(), "Didn't expect the 'self' keyword. The method must be a static method.")),
                    FnArg::Typed(arg) => {
                        let result: Result<ParameterType, _> = (arg.ty.as_ref(), self.generics).try_into();
                        
                        let name = arg.pat.as_ref();
                        result.and_then(|parameter| {
                            if (lock_type.is_some() && parameter.lock_name.is_some()) && lock_type.to_token_stream().to_string() != parameter.lock_name.to_token_stream().to_string() {
                                return Err(Error::new(parameter.lock_name.span(), "Parameter does not use the same lock as the other parameters. All lock types must be the same."))
                            }
                            
                            if parameter.lock_name.is_some() {
                                lock_type = parameter.lock_name.clone();
                            }
                            
                            let ty = parameter.ty;
                            match (parameter.lock_name, parameter.is_iterator) {
                                (Some(_), false) => {
                                    Ok(quote! {
                                        let #name = injector.get::<#ty>().expect(concat!("The type '", stringify!(#ty), "' has not been added as a service."));
                                    })
                                },
                                (None, false) => {
                                    Ok(quote! {
                                        let #name = injector.produce::<#ty>().expect(concat!("The type '", stringify!(#ty), "' has not been added as a service."));
                                    })
                                },
                                (Some(_), true) => {
                                    Ok(quote! {
                                        let #name = injector.get_list::<#ty>();
                                    })
                                },
                                (None, true) => {
                                    Err(Error::new(arg.ty.span(), "Iterator must contain a lockable type."))
                                }
                            }
                        })
                    }
                })
                .collect::<Result<Vec<_>, Error>>();
            
            let injections = match injections {
                Ok(value) => value,
                Err(error) => { 
                    let error = error.to_compile_error();
                    quote! { #error }.to_tokens(tokens); 
                    return; 
                },
            };
            
            (block, injections, lock_type)
        } else {
            (quote! {
                Self {}
            }, Vec::new(), None)  
        };
        
        let lock_type = match (self.lock_type, lock_type) {
            (Some(lock_type), _) => lock_type.clone(),
            (None, Some(lock_type)) => lock_type,
            _ => {
                generics.params.push(parse_quote!(Lock : dilian::sync::Lock));
                
                parse_quote!(Lock)
            }
        };

        let tree = quote! {
            impl #generics dilian::FromInjector<#lock_type> for #ty {
                fn from_injector(injector: &dilian::Injector<#lock_type>) -> Self {
                    #(#injections)*
                    
                    #body
                }
            }
        };

        tree.to_tokens(tokens);
    }
}

impl ToTokens for DynamicInjectableImpl<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let _trait = self.item_impl.trait_.as_ref().unwrap();
        let _trait = &_trait.1;
        let ty = self.item_impl.self_ty.as_ref();
        let mut generics = self.generics.clone();
        generics.params.push(parse_quote!(Lock : dilian::sync::Lock));
        
        let types = self.item_impl.items
            .iter()
            .filter_map(|item| match item {
                ImplItem::Type(ty) => Some(ty),
                _ => None,
            })
            .map(|ty| {
                let name = &ty.ident;
                let ty = &ty.ty;
                quote! { #name = #ty }
            })
            .collect::<Vec<_>>();
        
        let types = if types.is_empty() {
            TokenStream2::new()
        } else {
            quote! { <#(#types),*> }
        };
        
        let tree = quote! {                
            #[allow(unsafe_code)]
            impl #generics dilian::DynamicInjectable<dyn #_trait #types, Lock> for #ty 
                where #ty : dilian::FromInjector<Lock>
            {
                fn create_dynamic(s: Lock::Lock<Self>) -> Lock::Lock<dyn #_trait #types> {
                    let dangling: *const Self = std::ptr::NonNull::dangling().as_ptr();
                    let fat_ptr = dangling as *const dyn #_trait #types;
                    let dilian::cell::RawFatPtr { vtable, .. } = unsafe { std::mem::transmute(fat_ptr) };
                    
                    unsafe { dilian::cell::coerce::<Lock, _, _>(s, unsafe { std::ptr::NonNull::new_unchecked(vtable as *mut ()) }) }
                }
            }
        };
        
        tree.to_tokens(tokens);
    }
}

impl<'a> TryFrom<(&'a Type, &'a Generics)> for ParameterType<'a> {

    type Error = syn::Error;

    fn try_from((ty, generics): (&'a Type, &'a Generics)) -> Result<Self, Self::Error> {
        if let Type::Path(path) = ty {
            let (lock_type, is_valid) = match path.qself.as_ref() {
                Some(s) => {
                    let range = path.path.segments
                        .iter()
                        .take(s.position);
                    
                    
                    (s.ty.as_ref().clone(), match_path("dilian::sync::Lock", range))
                },
                None => {
                    let first_segment = path.path.segments.first();
                    if let Some(segment) = first_segment {
                        let lock_generic = generics
                            .params
                            .iter()
                            .find(|generic| match generic {
                                GenericParam::Type(ty) => {
                                    if ty.ident == segment.ident {
                                        let lock_bounds = ty.bounds
                                            .iter()
                                            .find(|bound| match bound {
                                                TypeParamBound::Trait(_trait) => {
                                                    match_path("dilian::sync::Lock", _trait.path.segments.iter())
                                                },
                                                _ => false,
                                            });
                                        
                                        lock_bounds.is_some()
                                    } else {
                                        false
                                    }
                                }
                                _ => false,
                            });

                        // if there is a first there must always be a last
                        let last_segment = path.path.segments.last().unwrap();
                        let last_segment_string = last_segment.ident.to_string();
                        if last_segment_string.ends_with("Lock") && lock_generic.is_none() {
                            let marker_type = get_marker_type(last_segment_string.as_str(), ty.span());
                            
                            (marker_type, true)
                        } else {
                            (parse_quote!(#segment), lock_generic.is_some())
                        }
                    } else {
                        (Type::Verbatim(TokenStream2::new()), false)
                    }
                },
            };
            
            
            
            if !is_valid {
                return Ok(Self {
                    lock_name: None,
                    ty,
                    is_iterator: false,
                });
            }
            
            let ty = get_generic_path(&path.path, "Lock<T>")?;
            Ok(Self {
                lock_name: Some(lock_type),
                ty: match ty {
                    GenericArgument::Type(ty) => ty,
                    generic => return Err(Error::new(generic.span(), "Expected generic type."))
                },
                is_iterator: false,
            })
        } else if let Some(result) = get_iterator_impl(ty) {
            match result {
                Ok(iterator) => {
                    let inner_argument: Result<ParameterType<'a>, _> = get_associated_generic_type(&iterator.path, "std::iter::Iterator<Item = T>")
                        .and_then(|x| (x, generics).try_into());

                    let inner_argument = match inner_argument {
                        Ok(x) => x,
                        Err(e) => {
                            return Err(e);
                        }
                    };

                    Ok(Self {
                        lock_name: inner_argument.lock_name,
                        ty: inner_argument.ty,
                        is_iterator: true,
                    })
                },
                Err(error) => Err(error),
            }
        } else {
            Ok(Self {
                lock_name: None,
                ty,
                is_iterator: false,
            })
        }
    }
}

fn get_marker_type(name: &str, span: Span) -> Type {
    match name {
        "RwLock" | "AsyncRwLock" => {
            let marker = format!("dilian::sync::{}Marker", name);

            Type::parse.parse_str(marker.as_str()).unwrap()
        },
        "MutexLock" | "AsyncMutexLock" | "RefCellLock" => {
            let marker = format!("dilian::sync::{}", name.replace("Lock", "Marker"));
            
            Type::parse.parse_str(marker.as_str()).unwrap()
        }
        name => {
            let marker = name.replace("Lock", "Marker");
            let ident = Ident::new(marker.as_str(), span);
            parse_quote!(#ident)
        }
    }
}