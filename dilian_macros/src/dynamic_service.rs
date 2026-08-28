use quote::{quote, ToTokens};
use syn::__private::TokenStream2;
use syn::{parse_quote, ItemTrait, Type, TypeParamBound};
use crate::helpers::{match_path};

pub struct DynamicService {
    _trait: ItemTrait,
}

impl From<ItemTrait> for DynamicService {
    fn from(item_trait: ItemTrait) -> Self {
        DynamicService { _trait: item_trait }
    }
}

impl ToTokens for DynamicService {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let ident = &self._trait.ident;
        let original_generics = &self._trait.generics;
        let mut generics = self._trait.generics.clone();
        let ty: Type = parse_quote!(dyn #ident #generics);
        
        generics.params.push(parse_quote!(Lock : dilian::sync::Lock));
        
        // let unique_id_impl = UniqueIdImpl::new(&ty, &self._trait.generics);
        
        let tree = quote! {
            
            impl #original_generics dilian::Injectable for #ty {}
            
            #[allow(unsafe_code)]
            unsafe impl #generics dilian::cell::AnyMetadata<Lock> for #ty {
                fn any_vtable(instance: &Lock::Lock<Self>) -> (std::ptr::NonNull<()>, Option<std::ptr::NonNull<()>>) {
                    let lock = Lock::as_raw(instance);
                    let dilian::cell::RawFatPtr { vtable: trait_vtable, .. } = unsafe { std::mem::transmute_copy(&lock) };
                    let dangling = dilian::cell::RawFatPtr {
                        data: std::ptr::NonNull::dangling().as_ptr(),
                        vtable: trait_vtable,
                    };
                    let trait_ptr: *const #ty = unsafe { std::mem::transmute(dangling) };
                    let any_ptr: *const dyn std::any::Any = trait_ptr;
                    let dilian::cell::RawFatPtr { vtable: any_vtable, .. } = unsafe { std::mem::transmute(any_ptr) };
                    
                    (unsafe { std::ptr::NonNull::new_unchecked(any_vtable as *mut ()) }, Some(unsafe { std::ptr::NonNull::new_unchecked(trait_vtable as *mut ()) }))
                }
            }
        };
        
        tree.to_tokens(tokens);
        
        let any_super_trait = self._trait.supertraits
            .iter()
            .find(|super_trait| match super_trait {
                TypeParamBound::Trait(_trait) => match_path("std::any::Any", _trait.path.segments.iter()),
                _ => false,
            });
        
        let mut _trait = self._trait.clone();
        if any_super_trait.is_none() {
            _trait.supertraits.push(parse_quote!(std::any::Any));
        }
        
        _trait.to_tokens(tokens);
    }
}