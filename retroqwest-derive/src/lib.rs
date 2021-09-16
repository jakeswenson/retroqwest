use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{quote, quote_spanned};
use std::convert::TryFrom;
use syn::spanned::Spanned;
use syn::{
    parse_macro_input, parse_quote, Attribute, FnArg, Ident, ImplItemMethod, ItemTrait, LitStr,
    Pat, PatType, Signature, TraitItem, TraitItemMethod,
};

#[proc_macro_attribute]
pub fn retroqwest(_args: TokenStream, input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as ItemTrait);

    expand(item).unwrap_or_else(to_compile_errors).into()
}

fn to_compile_errors(errors: syn::Error) -> proc_macro2::TokenStream {
    let compile_errors = errors.to_compile_error();
    compile_errors
}

struct HttpMethodAttribute {
    method: Ident,
    _response: Option<Ident>,
    uri: LitStr,
}

impl TryFrom<Attribute> for HttpMethodAttribute {
    type Error = syn::Error;
    fn try_from(att: Attribute) -> Result<Self, Self::Error> {
        let mut segments = att.path.segments.iter();
        let uri: LitStr = att.parse_args()?;

        Ok(Self {
            method: segments.next().unwrap().ident.clone(),
            _response: segments.next().map(|s| s.ident.clone()),
            uri,
        })
    }
}

fn get_att(
    attrs: &mut Vec<Attribute>,
    name: &'static str,
    span: Span,
) -> Result<Attribute, syn::Error> {
    let index = attrs
        .iter()
        .enumerate()
        .find_map(move |(i, a)| {
            a.path
                .segments
                .first()
                .filter(|p| p.ident == name)
                .map(move |_| i)
        })
        .ok_or(syn::Error::new(span, "Missing http method attribute"));

    index.map(|i| attrs.remove(i))
}

fn build_method(
    attrs: &mut Vec<Attribute>,
    sig: &mut Signature,
) -> Result<ImplItemMethod, syn::Error> {
    let attr = get_att(attrs, "get", sig.span())?;
    let att = HttpMethodAttribute::try_from(attr)?;

    let uri_format_args = sig.inputs.iter().filter_map(|a| match a {
        FnArg::Typed(PatType { pat, .. }) => {
            if let Pat::Ident(ident) = pat.as_ref() {
                Some(quote_spanned!(ident.span()=>#ident = #ident))
            } else {
                None
            }
        }
        _ => None,
    });

    let uri = att.uri;
    let method = att.method;

    let uri = quote_spanned!(uri.span()=>concat!("{}", #uri));

    Ok(parse_quote! {
      #(#attrs)*
      #sig {
       Ok(self.client.#method(format!(#uri, self.endpoint#(, #uri_format_args)*))
          .send().await.map_err(retroqwest::RetroqwestError::RequestError)?
          .error_for_status().map_err(retroqwest::RetroqwestError::ResponseError)?
          .json().await.map_err(retroqwest::RetroqwestError::JsonParse)?)
      }
    })
}

fn expand(mut def: ItemTrait) -> Result<proc_macro2::TokenStream, syn::Error> {
    let trait_name = &def.ident;
    let name = Ident::new(&format!("{}Client", trait_name), def.ident.span());
    let vis = &def.vis;

    let mut methods: Vec<ImplItemMethod> = vec![];

    for x in &mut def.items {
        match x {
            TraitItem::Method(TraitItemMethod {
                attrs,
                sig,
                default,
                ..
            }) => {
                if default.is_some() {
                    return Err(syn::Error::new(
                        default.as_ref().unwrap().span(),
                        "retroquest trait methods cannot have defaults",
                    ));
                }

                methods.push(build_method(attrs, sig)?)
            }
            _ => (),
        }
    }

    let client = quote! {
      #[derive(Clone, Debug)]
      #vis struct #name {
        endpoint: String,
        client: reqwest::Client,
      }

      #[async_trait::async_trait]
      impl #trait_name for #name {
          #(#methods)*
      }

      impl #name {
        fn from_builder<T: Into<String>>(
          base_url: T,
          client_builder: reqwest::ClientBuilder)
        -> Result<Self, retroqwest::RetroqwestError>  {
          Ok(Self {
            endpoint: base_url.into().trim_end_matches('/').to_string(),
            client: client_builder.build().map_err(retroqwest::RetroqwestError::FailedToBuildClient)?
          })
        }
      }
    };

    def.attrs.push(parse_quote!(#[async_trait::async_trait]));

    Ok(quote! {
      #def

      #client
    })
}
