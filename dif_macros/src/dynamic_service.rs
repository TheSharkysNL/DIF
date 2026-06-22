use quote::{quote, ToTokens};
use syn::__private::TokenStream2;
use syn::{parse_quote, ItemTrait, Type};

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
        let generics = &self._trait.generics;
        let ty: Type = parse_quote!(dyn #ident #generics);
        
        // let unique_id_impl = UniqueIdImpl::new(&ty, &self._trait.generics);
        
        let tree = quote! {
            
            impl #generics dif::Injectable for #ty {}
        };
        
        tree.to_tokens(tokens);
        self._trait.to_tokens(tokens);
    }
}