use quote::ToTokens;
use syn::__private::TokenStream2;
use syn::{GenericArgument, ImplItem, ImplItemFn, ItemImpl, PathArguments, ReturnType, Type, TypeParamBound, TraitBound};
use syn::spanned::Spanned;

pub fn get_method<'a>(_impl: &'a ItemImpl, name: &str) -> Result<&'a ImplItemFn, syn::Error> {
    for item in &_impl.items {
        if let ImplItem::Fn(method) = item {
            if method.sig.ident == name {
                return Ok(method);
            }
        }
    }
    
    Err(syn::Error::new(_impl.span(), format!("The method with the name '{}' was not found within the impl statement", name)))
    
}

pub fn returns_self(func: &ImplItemFn) -> bool {
    match &func.sig.output {
        ReturnType::Default => false,
        ReturnType::Type(_, ty) => {
            if let Type::Path(path) = ty.as_ref() {
                if let Some(segment) = path.path.segments.first() {
                    segment.ident == "Self"
                } else { false }
            } else { false }
        }
    }
}

pub fn match_path_type(name: &str, ty: &syn::Type) -> bool {
    let split = name.split("::").collect::<Vec<_>>();

    let segments = match ty {
        syn::Type::Path(path) => {
            &path.path.segments
        }
        _ => return false,
    };

    for (segment_index, segment) in segments.iter().enumerate() {
        for (path_index, path) in split.iter().enumerate() {
            if segment.ident == path {
                if split.len() - path_index != segments.len() - segment_index {
                    return false;
                }
                let length_remaining = split.len() - path_index;

                for i in 1..length_remaining {
                    let path = split[path_index + i];
                    let segment = &segments[segment_index + i];

                    if segment.ident != path {
                        return false;
                    }
                }

                return true;
            }
        }
    }

    false
}

pub fn get_generic(ty: &syn::Type, name: &str) -> TokenStream2 {
    match get_generic_type(ty, name) {
        Ok(t) => t.into_token_stream(),
        Err(e) => e.to_compile_error(),
    }
}

pub fn get_generic_type<'a>(ty: &'a syn::Type, name: &str) -> Result<&'a syn::Type, syn::Error> {
    match ty {
        Type::Path(path) => {
            let generic = get_generic_path(&path.path, name)?;
            match generic {
                GenericArgument::Type(t) => Ok(t),
                _ => {
                    let error = syn::Error::new(generic.span(),  format!("Expected type of T from {}.", name));
                    Err(error)
                }
            }
        },
        _ => unreachable!("Should not be reachable as match_path_type should have handled this case"),
    }
}

fn get_generic_path<'a>(path: &'a syn::Path, name: &str) -> Result<&'a syn::GenericArgument, syn::Error> {
    let last_segment = path.segments.last().expect("Should not be reachable as match_path_type should have handled this unwrap case.");
    let arguments = &last_segment.arguments;
    match arguments {
        PathArguments::AngleBracketed(args) => {
            if args.args.len() != 1 {
                let error = syn::Error::new(arguments.span(), format!("Expected type of T from {}.", name));
                Err(error)
            } else {
                let first = &args.args[0];
                Ok(first)
            }
        }
        _ => {
            let error = syn::Error::new(arguments.span(), format!("Expected type of T from {}.", name));
            Err(error)
        }
    }
}

pub fn get_associated_generic_type<'a>(path: &'a syn::Path, name: &str) -> Result<&'a syn::Type, syn::Error> {
    let generic = get_generic_path(path, name)?;
    match generic {
        GenericArgument::AssocType(t) => Ok(&t.ty),
        _ => Err(syn::Error::new(generic.span(), format!("Expected type of associated T from {}.", name)))
    }
}

pub fn get_iterator_impl(ty: &syn::Type) -> Option<Result<&TraitBound, syn::Error>> {
    match ty {
        Type::ImplTrait(impl_trait) => {
            if impl_trait.bounds.len() != 1 {
                return Some(Err(syn::Error::new(impl_trait.bounds.span(), "Expected iterator trait bound.".to_string())));
            }
            
            let first = &impl_trait.bounds[0];
            match first {
                TypeParamBound::Trait(_trait) => Some(Ok(&_trait)),
                _ => Some(Err(syn::Error::new(impl_trait.bounds.span(), "Expected iterator trait bound.".to_string()))),
            }
        },
        _ => None
    }
}