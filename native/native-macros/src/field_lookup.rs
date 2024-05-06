use proc_macro2::TokenStream;
use quote::{format_ident, quote_spanned, ToTokens};
use syn::{spanned::Spanned, *};

fn pop_attr(attrs: &mut Vec<Attribute>, name: &str) -> Option<Attribute> {
    let mut i = attrs.iter().enumerate();
    let attr = loop {
        match i.next() {
            Some((idx, a)) => {
                if let Some(ident) = a.path.get_ident()
                    && *ident == name
                {
                    break Some(idx);
                }
            }
            None => break None,
        }
    };

    attr.map(|idx| attrs.remove(idx))
}

pub fn field_lookup(attr: TokenStream, item: TokenStream) -> TokenStream {
    let class: LitStr = match syn::parse2(attr) {
        Ok(x) => x,
        Err(e) => {
            return e.to_compile_error();
        }
    };

    let mut item: ItemStruct = match syn::parse2(item) {
        Ok(x) => x,
        Err(e) => {
            return e.to_compile_error();
        }
    };

    let mut fields = Vec::new();

    for field in &mut item.fields {
        let field_type = match pop_attr(&mut field.attrs, "field_type") {
            Some(x) => x,
            None => {
                return quote_spanned! {field.span()=>
                    compile_error!("field must have a #[field_type(\"...\")] attribute");
                };
            }
        };

        let field_type: LitStr = match field_type.parse_args() {
            Ok(x) => x,
            Err(e) => {
                return e.to_compile_error();
            }
        };

        fields.push((
            field.ident.as_ref().unwrap().to_string(),
            field_type.value(),
        ));
    }

    let item_name = item.ident.to_string();

    quote! {
        #item

        impl #item_name {
            pub fn get(env: jni::JNIEnv<'_>) -> Self {

            }
        }
    }
}
