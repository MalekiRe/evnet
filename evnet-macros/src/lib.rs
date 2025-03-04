use proc_macro::{self, TokenStream};
use proc_macro2::{Ident, TokenTree};
use quote::{quote, TokenStreamExt};
use syn::{parse_macro_input, DeriveInput};
use syn::spanned::Spanned;

enum Reliability {
    Reliable,
    Unreliable,
    UnreliableOrdered,
}
impl quote::ToTokens for Reliability {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        tokens.append(TokenTree::Ident(Ident::new(match self {
            Reliability::Reliable => "Reliable",
            Reliability::Unreliable => "Unreliable",
            Reliability::UnreliableOrdered => "UnreliableOrdered",
        }, tokens.span())))
    }
}
#[proc_macro_derive(NetworkMessage, attributes(Reliable, Unreliable, UnreliableOrdered))]
pub fn derive(input: TokenStream) -> TokenStream {
    let input: DeriveInput = parse_macro_input!(input);
    let mut reliability = Reliability::Reliable;
    for attr in &input.attrs {
        if attr.tokens.to_string() == "Unreliable" {
            reliability = Reliability::Unreliable;
        }
        if attr.tokens.to_string() == "UnreliableOrdered" {
            reliability = Reliability::UnreliableOrdered;
        }
    }
    let reliability = quote!{
        const RELIABILITY: ::evnet::Reliability = ::evnet::Reliability::#reliability;
    };
    let DeriveInput { ident, .. } = input;
    let output = quote! {
        impl ::evnet::message_layer::NetworkMessage for #ident {
            #reliability
        }
    };
    output.into()
}