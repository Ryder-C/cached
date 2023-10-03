use proc_macro::TokenStream;

use inflector;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote, ToTokens};
use syn::{parse::Parse, FnArg, ItemFn, Token, Type};

struct ReturnType {
    ty: Type,
}

impl Parse for ReturnType {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        input.parse::<Token![->]>()?;

        let ty: Type = input.parse()?;

        Ok(Self { ty })
    }
}

#[proc_macro_attribute]
pub fn cached(_attr: TokenStream, input: TokenStream) -> TokenStream {
    let input: TokenStream2 = TokenStream2::from(input);
    let funcdef: ItemFn = syn::parse2(input).unwrap();

    let tracker_upper = format_ident!(
        "{}Tracker",
        inflector::cases::pascalcase::to_pascal_case(funcdef.sig.ident.to_string().as_str())
    );
    let tracker_lower = format_ident!("{}_tracker", funcdef.sig.ident.to_string());

    let signature = format_ident!(
        "{}Signature",
        inflector::cases::pascalcase::to_pascal_case(funcdef.sig.ident.to_string().as_str())
    );

    let fake_output = &funcdef.sig.output;
    let output = syn::parse2::<ReturnType>(quote! { #fake_output })
        .unwrap()
        .ty;

    let key = &funcdef.sig.inputs;
    let params: Vec<TokenStream2> = key.iter().map(|x| x.to_token_stream()).collect();
    let just_names: Vec<TokenStream2> = key.iter().map(|x| match x {
            FnArg::Typed(value) => value.pat.to_token_stream(),
            _ => panic!("Invalid function argument signature."),
        }).collect();

    let vis = &funcdef.vis;
    let sig = &funcdef.sig;
    let block = &funcdef.block;

    quote! {
        #[derive(PartialEq, Eq, Hash)]
        struct #signature { #(#params),* }

        struct #tracker_upper {
            cache: Option<std::collections::HashMap<#signature, #output>>,
        }

        impl #tracker_upper {
            const fn new() -> Self {
                Self { cache: None }
            }
        }

        #vis #sig {
            static #tracker_lower: std::sync::RwLock<#tracker_upper> = std::sync::RwLock::new(#tracker_upper::new());

            {
                let mut tracker = #tracker_lower.try_write().unwrap();
                match tracker.cache {
                    Some(_) => (),
                    None => tracker.cache = Some(std::collections::HashMap::new()),
                };
            }

            let key = #signature { #(#just_names),* };

            let entry = #tracker_lower.try_read().unwrap().cache.as_ref().unwrap().get(&key).copied();

            match entry {
                Some(value) => value,
                None => {
                    let value = #block;
                    #tracker_lower.try_write().unwrap().cache.as_mut().unwrap().insert(key, value.clone());
                    value
                }
            }
        }
    }
    .into()
}
