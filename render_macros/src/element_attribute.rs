use quote::quote;
use std::hash::{Hash, Hasher};
use syn::ext::IdentExt;
use syn::parse::{Parse, ParseStream, Result};
use syn::spanned::Spanned;

pub type AttributeKey = syn::punctuated::Punctuated<proc_macro2::Ident, proc_macro2::Punct>;

#[derive(Clone)]
pub enum ElementAttribute {
    Punned(AttributeKey),
    WithValue(AttributeKey, syn::Block),
}

impl ElementAttribute {
    pub fn ident(&self) -> &AttributeKey {
        match self {
            Self::Punned(ident) | Self::WithValue(ident, _) => ident,
        }
    }

    pub fn idents(&self) -> Vec<&syn::Ident> {
        self.ident().iter().collect::<Vec<_>>()
    }

    pub fn value_tokens(&self) -> proc_macro2::TokenStream {
        match self {
            Self::WithValue(_, value) => {
                if value.stmts.len() == 1 {
                    let first = &value.stmts[0];
                    quote!(#first)
                } else {
                    quote!(#value)
                }
            }
            Self::Punned(ident) => quote!(#ident),
        }
    }

    pub fn validate(self, is_custom_element: bool) -> Result<Self> {
        if is_custom_element {
            self.validate_for_custom_element()
        } else {
            self.validate_for_simple_element()
        }
    }

    pub fn validate_for_custom_element(self) -> Result<Self> {
        if self.idents().len() < 2 {
            Ok(self)
        } else {
            let alternative_name = self
                .idents()
                .iter()
                .map(|x| x.to_string())
                .collect::<Vec<_>>()
                .join("_");

            let error_message = format!(
                "Can't use dash-delimited values on custom components. Did you mean `{}`?",
                alternative_name
            );

            Err(syn::Error::new(self.ident().span(), error_message))
        }
    }

    pub fn validate_for_simple_element(self) -> Result<Self> {
        match (&self, self.idents().len()) {
            (Self::Punned(ref key), len) if len > 1 => {
                let error_message = "Can't use punning with dash-delimited values";
                Err(syn::Error::new(key.span(), error_message))
            }
            _ => Ok(self),
        }
    }
}

impl PartialEq for ElementAttribute {
    fn eq(&self, other: &Self) -> bool {
        let self_idents: Vec<_> = self.ident().iter().collect();
        let other_idents: Vec<_> = other.ident().iter().collect();
        self_idents == other_idents
    }
}

impl Eq for ElementAttribute {}

impl Hash for ElementAttribute {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let ident = self.idents();
        Hash::hash(&ident, state)
    }
}

impl Parse for ElementAttribute {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut name: syn::punctuated::Punctuated<proc_macro2::Ident, proc_macro2::Punct> =
            syn::punctuated::Punctuated::new();

        // Parse the input up to the space
        loop {
            let value = syn::Ident::parse_any(&input).unwrap();
            name.push_value(value);

            if input.peek(syn::Token![=]) {
                break;
            }

            let punct = input.parse().unwrap();
            name.push_punct(punct);
        }

        // Peak for incoming equals to check if its punned
        let mut not_punned = input.peek(syn::Token![=]);

        if !not_punned {
            not_punned = input.peek2(syn::Token![=]);
        }

        if !not_punned {
            not_punned = input.peek3(syn::Token![=]);
        }

        if !not_punned {
            return Ok(Self::Punned(name));
        }

        // Parse equals
        input.parse::<syn::Token![=]>()?;

        // Parse body
        let value = input.parse::<syn::Block>()?;

        Ok(Self::WithValue(name, value))
    }
}
