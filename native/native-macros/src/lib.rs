use proc_macro::TokenStream;

mod jni_export;

#[proc_macro_attribute]
pub fn jni_export(attr: TokenStream, item: TokenStream) -> TokenStream {
    jni_export::jni_export(attr.into(), item.into()).into()
}
