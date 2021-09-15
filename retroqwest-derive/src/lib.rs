use proc_macro::TokenStream;
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{
    parse_macro_input, parse_quote, FnArg, Ident, ImplItemMethod, ItemTrait, LitStr, Pat, PatType,
    TraitItem, TraitItemMethod,
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

                let attr = attrs.iter().enumerate().find_map(|(i, a)| {
                    a.path
                        .segments
                        .first()
                        .filter(|p| p.ident.to_string() == "get")
                        .map(|_| i)
                });

                if attr.is_some() {
                    let get: LitStr = attrs.remove(attr.unwrap()).parse_args()?;

                    let uri_args = sig.inputs.iter().filter_map(|a| match a {
                        FnArg::Typed(PatType { pat, .. }) => {
                            if let Pat::Ident(ident) = pat.as_ref() {
                                Some(quote_spanned!(ident.span()=>#ident = #ident))
                            } else {
                                None
                            }
                        }
                        _ => None,
                    });

                    let uri = quote_spanned!(get.span()=>concat!("{}", #get));

                    methods.push(parse_quote! {
                      #sig {
                       Ok(self.client.get(format!(#uri, self.endpoint#(, #uri_args)*))
                          .send().await.map_err(retroqwest::RetroqwestError::RequestError)?
                          .error_for_status().map_err(retroqwest::RetroqwestError::ResponseError)?
                          .json().await.map_err(retroqwest::RetroqwestError::JsonParse)?)
                      }
                    });
                } else if attrs[0]
                    .path
                    .segments
                    .first()
                    .unwrap()
                    .ident
                    .to_string()
                    .starts_with("post")
                {
                    attrs.remove(0);

                    methods.push(parse_quote! {
                      #sig {
                        unimplemented!();
                      }
                    });
                }
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
