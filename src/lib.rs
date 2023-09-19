use proc_macro::TokenStream;

use inflector;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote, ToTokens};
use syn::{parse::Parse, FnArg, ItemFn, Token, Type};

struct FuckingOutputType {
    ty: Type,
}

impl Parse for FuckingOutputType {
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

    let output = syn::parse2::<FuckingOutputType>(quote! { #fake_output })
        .unwrap()
        .ty;

    let key = &funcdef.sig.inputs;
    let params: Vec<TokenStream2> = key.iter().map(|x| x.to_token_stream()).collect();
    let just_names: Vec<TokenStream2> = key
        .iter()
        .map(|x| match x {
            FnArg::Typed(value) => value.pat.to_token_stream(),
            _ => {
                panic!("Invalid function argument signature.");
            }
        })
        .collect();

    let vis = &funcdef.vis;
    let sig = &funcdef.sig;
    let block = &funcdef.block;

    quote! {
        #[derive(PartialEq, Eq, Hash)]
        struct #signature {
            #(#params),*
        }

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

            println!("Initialized tracker locker!");

            match #tracker_lower.read() {
                Ok(maybe_tracker) => match maybe_tracker.cache {
                    Some(_) => (),
                    None => {
                        let mut tracker = #tracker_lower.write().unwrap();
                        tracker.cache = Some(std::collections::HashMap::new());
                        println!("Initialized tracker!");
                    }
                }
                Err(_) => {
                    panic!("Failed to get lock on cache.");
                }
            };

            let key = #signature { #(#just_names),* };

            match #tracker_lower.read() {
                Ok(tracker) => match tracker.cache.as_ref().unwrap().get(&key) {
                    Some(value) => *value,
                    None => {
                        let value = #block;
                        let mut tracker = #tracker_lower.write().unwrap();
                        tracker.cache.as_mut().unwrap().insert(key, value.clone());
                        value
                    }
                },
                Err(_) => {
                    panic!("Failed to get lock on cache.");
                }
            }
        }
    }
    .into()
}
