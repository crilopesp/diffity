//! `#[command]` for `diffity-server`, where `tauri::command` resolves to this macro instead of Tauri's.
//!
//! It keeps the function as it is and adds, in the type namespace, a struct with the same name that implements
//! `crate::server::glue::WebCommand`: it reads the arguments from the JSON the browser sends (camelCase keys, as
//! Tauri's `invoke` does), passes the server's `State`, `AppHandle` and `Channel`s, and serializes the result.
//! `crate::server::handler` lists the commands by path, like `tauri::generate_handler!`.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as Tokens;
use quote::quote;
use syn::{parse_macro_input, FnArg, ItemFn, Pat, ReturnType, Type};

#[proc_macro_attribute]
pub fn command(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let function = parse_macro_input!(item as ItemFn);
    match web_command(&function) {
        Ok(adapter) => quote!(#function #adapter).into(),
        Err(error) => error.to_compile_error().into(),
    }
}

fn web_command(function: &ItemFn) -> syn::Result<Tokens> {
    let name = &function.sig.ident;
    let vis = &function.vis;
    let command = name.to_string();
    let arguments = function.sig.inputs.iter().map(argument).collect::<syn::Result<Vec<_>>>()?;
    let call = quote!(#name(#(#arguments),*));
    let call = if function.sig.asyncness.is_some() { quote!(#call.await) } else { call };
    let result = match &function.sig.output {
        ReturnType::Default => quote!({ #call; Ok::<(), crate::core::AppError>(()) }),
        ReturnType::Type(_, ty) if last_segment(ty).as_deref() == Some("Result") => call,
        ReturnType::Type(..) => quote!(Ok::<_, crate::core::AppError>(#call)),
    };
    Ok(quote! {
        #[doc(hidden)]
        #[allow(non_camel_case_types)]
        #vis struct #name {}

        impl crate::server::glue::WebCommand for #name {
            const NAME: &'static str = #command;

            fn run<'a>(server: &'a crate::server::Server, args: ::serde_json::Value) -> crate::server::glue::Reply<'a> {
                ::std::boxed::Box::pin(async move { crate::server::glue::reply(#result) })
            }
        }
    })
}

/// How the browser's call provides one parameter of the command.
fn argument(input: &FnArg) -> syn::Result<Tokens> {
    let FnArg::Typed(typed) = input else {
        return Err(syn::Error::new_spanned(input, "commands take no `self`"));
    };
    let Pat::Ident(pattern) = typed.pat.as_ref() else {
        return Err(syn::Error::new_spanned(&typed.pat, "command parameters must be plain names"));
    };
    let key = camel_case(&pattern.ident.to_string());
    let ty = &typed.ty;
    Ok(match last_segment(ty).as_deref() {
        Some("State") => quote!(crate::server::glue::state(server)),
        Some("AppHandle") => quote!(crate::server::glue::app_handle(server)),
        Some("Channel") => quote!(match crate::server::glue::channel(server, &args, #key) {
            Ok(channel) => channel,
            Err(error) => return crate::server::glue::reply::<()>(Err(error)),
        }),
        Some("Window" | "WebviewWindow" | "Webview") => {
            return Err(syn::Error::new_spanned(ty, "diffity-server has no windows; leave this command out of the web app"));
        }
        _ => quote!(match crate::server::glue::argument::<#ty>(&args, #key) {
            Ok(value) => value,
            Err(error) => return crate::server::glue::reply::<()>(Err(error)),
        }),
    })
}

fn last_segment(ty: &Type) -> Option<String> {
    match ty {
        Type::Path(path) => path.path.segments.last().map(|segment| segment.ident.to_string()),
        Type::Reference(reference) => last_segment(&reference.elem),
        _ => None,
    }
}

/// Tauri's argument names: `repo_path` → `repoPath`, `r#ref` → `ref`.
fn camel_case(name: &str) -> String {
    let name = name.trim_start_matches("r#");
    let mut out = String::with_capacity(name.len());
    let mut upper = false;
    for c in name.chars() {
        if c == '_' {
            upper = true;
        } else if upper {
            out.extend(c.to_uppercase());
            upper = false;
        } else {
            out.push(c);
        }
    }
    out
}

