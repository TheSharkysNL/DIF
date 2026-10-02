use crate::helpers::{get_associated_generic_type, get_generic_path, get_generic_type, get_iterator_impl, get_method, match_path, match_path_type, returns_self};
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

#[derive(Clone, Eq, PartialEq)]
pub enum IteratorType {
    None, 
    ImplIterator,
    Vec,
}

pub struct ParameterType<'a> {
    lock_name: Option<Type>,
    ty: &'a Type,
    iterator_type: IteratorType,
    is_optional: bool,
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
                            let mut generic_ty = None;
                            for generic in generics.params.iter_mut() {
                                if let GenericParam::Type(ref mut type_param) = *generic {
                                    if matches!(ty, Type::Path(path) if path.path.is_ident(&type_param.ident)) {
                                        generic_ty = Some(type_param);
                                    }
                                }
                            }
                            
                            let expect = if parameter.is_optional {
                                quote! {}
                            } else {
                                quote! { .expect(concat!("The type '", stringify!(#ty), "' has not been added as a service.")) }
                            };
                            
                            let collect = if parameter.iterator_type == IteratorType::Vec {
                                quote! { .collect::<std::vec::Vec<_>>() }
                            } else {
                                quote! {}
                            };
                            
                            match (parameter.lock_name, parameter.iterator_type) {
                                (Some(_), IteratorType::None) => {
                                    if let Some(generic) = generic_ty {
                                        generic.bounds.push(parse_quote!(?Sized));
                                        generic.bounds.push(parse_quote!('static));
                                    }
                                    Ok(quote! {
                                        let #name = injector.get::<#ty>() #expect;
                                    })
                                },
                                (None, IteratorType::None) => {
                                    if let Some(generic) = generic_ty {
                                        generic.bounds.push(parse_quote!('static));
                                    }
                                    Ok(quote! {
                                        let #name = injector.produce::<#ty>() #expect;
                                    })
                                },
                                (Some(_), _) => {
                                    if let Some(generic) = generic_ty {
                                        generic.bounds.push(parse_quote!(?Sized));
                                        generic.bounds.push(parse_quote!('static));
                                    }
                                    
                                    if parameter.is_optional {
                                        return Err(Error::new(arg.ty.span(), "Iterator cannot be optional. If the service was not found the iterator will be empty."))
                                    }
                                    
                                    Ok(quote! {
                                        let #name = injector.get_list::<#ty>() #collect;
                                    })
                                },
                                (None, _) => {
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

fn get_parameter_type_inner<'a>(ty: &'a Type, generics: &'a Generics) -> syn::Result<(Option<Type>, &'a Type, IteratorType)> {
    if let Type::Path(path) = ty {
        if match_path_type("std::vec::Vec", ty) {
            let (lock_name, ty, _) = get_generic_type(ty, "std::vec::Vec<T>")
                .and_then(|x| get_parameter_type_inner(x, generics))?;

            return Ok((
                lock_name,
                ty,
                IteratorType::Vec,
            ))
        }

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
            return Ok((None, ty, crate::service::IteratorType::None));
        }

        let ty = get_generic_path(&path.path, "Lock<T>")?;
        Ok((
            Some(lock_type),
            match ty {
                GenericArgument::Type(ty) => ty,
                generic => return Err(Error::new(generic.span(), "Expected generic type."))
            },
            IteratorType::None,
        ))
    } else if let Some(result) = get_iterator_impl(ty) {
        match result {
            Ok(iterator) => {
                let inner_argument = get_associated_generic_type(&iterator.path, "std::iter::Iterator<Item = T>")
                    .and_then(|x| get_parameter_type_inner(x, generics))?;
                
                Ok((
                    inner_argument.0,
                    inner_argument.1,
                    IteratorType::ImplIterator,
                ))
            },
            Err(error) => Err(error),
        }
    } else {
        Ok((None,
            ty,
            IteratorType::None,
        ))
    }
}

impl<'a> TryFrom<(&'a Type, &'a Generics)> for ParameterType<'a> {

    type Error = syn::Error;

    fn try_from((ty, generics): (&'a Type, &'a Generics)) -> Result<Self, Self::Error> {
        if match_path_type("std::option::Option", ty) {
            let inner = get_generic_type(ty, "std::option::Option")?;
            let (lock_name, ty, is_iterator) = get_parameter_type_inner(inner, generics)?;
            Ok(Self {
                lock_name,
                ty,
                iterator_type: is_iterator,
                is_optional: true
            })
        } else {
            let (lock_name, ty, is_iterator) = get_parameter_type_inner(ty, generics)?;
            Ok(Self {
                lock_name,
                ty,
                iterator_type: is_iterator,
                is_optional: false
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