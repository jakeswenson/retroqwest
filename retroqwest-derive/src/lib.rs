use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{quote, quote_spanned};
use std::convert::TryFrom;
use syn::spanned::Spanned;
use syn::{parse_macro_input, parse_quote, Attribute, FnArg, Ident, ImplItemMethod, ItemTrait, LitStr, Pat, PatType, Signature, TraitItem, TraitItemMethod, PatIdent};

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

    segments.next().expect("the http segment has to exist");

    let uri: LitStr = att.parse_args()?;

    let path_seg = segments.next().ok_or(syn::Error::new(att.span(), "http attribute missing method"))?;

    Ok(Self {
      method: path_seg.ident.clone(),
      _response: None,
      uri,
    })
  }
}

fn get_att(
  attrs: &mut Vec<Attribute>,
  name: &'static str
) -> Option<Attribute> {
  attrs
    .iter()
    .enumerate()
    .find_map(move |(i, a)| {
      a.path
        .segments
        .first()
        .filter(|p| p.ident == name)
        .map(move |_| i)
    }).map(|i| attrs.remove(i))
}

enum HttpArg {
  JsonBody {
    arg: Ident,
    span: Span,
  },
  Uri {
    arg: Ident,
    span: Span,
  },
  Query {
    name: LitStr,
    arg: Ident,
    span: Span,
  },
}

impl HttpArg {
  fn parse(arg: &mut FnArg) -> Option<Self> {
    match arg {
      FnArg::Typed(PatType { attrs, pat, .. }) => {
        if let Pat::Ident(PatIdent { ident, .. }) = pat.as_ref() {
          if let Some(_json_att) = get_att(attrs, "json") {
            Some(HttpArg::JsonBody {
              arg: ident.clone(),
              span: ident.span()
            })
          }
          else if let Some(_query_att) = get_att(attrs, "query") {
            Some(HttpArg::Query {
              name: LitStr::new(ident.to_string().as_str(), ident.span()),
              arg: ident.clone(),
              span: ident.span()
            })
          }
          else {
            Some(HttpArg::Uri {
              arg: ident.clone(),
              span: ident.span(),
            })
          }
        } else {
          None
        }
      }
      _ => None,
    }
  }
}

fn build_method(
  attrs: &mut Vec<Attribute>,
  sig: &mut Signature,
) -> Result<ImplItemMethod, syn::Error> {
  let attr = get_att(attrs, "http").ok_or(syn::Error::new(sig.span(), "Missing http method attribute"))?;
  let att = HttpMethodAttribute::try_from(attr)?;

  let args = sig.inputs.iter_mut()
    .filter_map(HttpArg::parse)
    .collect::<Vec<_>>();

  let uri_args = args.iter().filter_map(|a| match a {
    HttpArg::Uri {
      arg,
      span
    } => Some(quote_spanned!(*span=> #arg = #arg)),
    _ => None
  });

  let query_args = args.iter().filter_map(|a| match a {
    HttpArg::Query {
      name,
      arg,
      span
    } => Some(quote_spanned!(*span=> (#name, #arg))),
    _ => None
  }).collect::<Vec<_>>();

  let query = if query_args.is_empty() {None} else {
    Some(quote! { .query(&[#(#query_args)*]) })
  };

  let body_args = args.iter().filter_map(|a| match a {
    HttpArg::JsonBody {
      arg,
      span
    } => Some(quote_spanned!(*span=> .json(#arg))),
    _ => None
  });

  let uri = att.uri;
  let method = att.method;

  let uri = quote_spanned!(uri.span()=>concat!("{}", #uri));

  Ok(parse_quote! {
      #(#attrs)*
      #sig {
       Ok(self.client.#method(format!(#uri, self.endpoint#(, #uri_args)*))
          #query
          #(#body_args)*
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
        client: retroqwest::reqwest::Client,
      }

      #[async_trait::async_trait]
      impl #trait_name for #name {
          #(#methods)*
      }

      impl #name {
        fn from_builder<T: Into<String>>(
          base_url: T,
          client_builder: retroqwest::reqwest::ClientBuilder)
        -> Result<Self, retroqwest::RetroqwestError>  {
          Ok(Self {
            endpoint: base_url.into().trim_end_matches('/').to_string(),
            client: client_builder.build().map_err(retroqwest::RetroqwestError::FailedToBuildClient)?
          })
        }
      }
    };

  def.attrs.push(parse_quote!(#[retroqwest::async_trait::async_trait]));

  Ok(quote! {
      #def

      #client
    })
}
