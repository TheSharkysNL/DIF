// use std::sync::atomic::Ordering;
// use quote::{quote, ToTokens};
// use syn::{GenericParam, Generics, Type};
// use portable_atomic::AtomicU128;
// use syn::__private::TokenStream2;
// 
// static COUNT: AtomicU128 = AtomicU128::new(0);
// 
// pub struct UniqueIdImpl<'a> {
//     ty: &'a Type,
//     generics: &'a Generics,
// }
// 
// impl<'a> UniqueIdImpl<'a> {
//     pub fn new(ty: &'a Type, generics: &'a Generics) -> Self {
//         Self {
//             ty,
//             generics,
//         }
//     }
// }
// 
// impl ToTokens for UniqueIdImpl<'_> {
//     fn to_tokens(&self, tokens: &mut TokenStream2) {
//         let ty = self.ty;
//         let generics = self.generics;
//         
//         let id = COUNT.fetch_add(1, Ordering::SeqCst);
//         let mut id = id.into_token_stream();
//         for generic in generics.params.iter() {
//             match generic {
//                 GenericParam::Type(ty) => {
//                     let ident = &ty.ident;
//                     id = quote! { 
//                         (#id).wrapping_mul(0xA5555529).wrapping_add(<#ident as dif::HasUniqueId>::unique_id())
//                     };
//                 }
//                 _ => continue,
//             }
//         }
//         
//         let tree = quote! {
//             impl #generics dif::HasUniqueId for #ty {
//                 fn unique_id() -> u128 {
//                     #id
//                 }
//             }
//         };
//         
//         tree.to_tokens(tokens);
//     }
// }