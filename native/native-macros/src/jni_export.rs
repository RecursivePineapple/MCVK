use proc_macro2::TokenStream;
use quote::{format_ident, ToTokens};
use syn::*;

pub fn jni_export(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attr: LitStr = match syn::parse2(attr) {
        Ok(x) => x,
        Err(e) => {
            return e.to_compile_error();
        }
    };

    let mut item: ItemFn = match syn::parse2(item) {
        Ok(x) => x,
        Err(e) => {
            return e.to_compile_error();
        }
    };

    let mut fn_prefix = "Java_".to_owned();

    let mut underscore_count = 0;

    for c in attr.value().chars() {
        if c == '_' {
            underscore_count += 1;
            continue;
        } else if underscore_count > 0 {
            fn_prefix.push_str(&format!("_{underscore_count}"));
            underscore_count = 0;
        }
        if c == '.' {
            fn_prefix.push('_');
        } else {
            fn_prefix.push(c);
        }
    }

    item.attrs.push(parse_quote! {
        #[no_mangle]
    });

    item.sig.abi = Some(parse_quote!(extern "system"));

    item.sig.ident = format_ident!("{}_{}", fn_prefix, item.sig.ident);

    item.into_token_stream()
}
