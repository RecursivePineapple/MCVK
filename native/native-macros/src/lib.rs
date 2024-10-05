use proc_macro::TokenStream;

mod gl_fn_decl;
mod jni_export;

#[proc_macro_attribute]
pub fn jni_export(attr: TokenStream, item: TokenStream) -> TokenStream {
    jni_export::jni_export(attr.into(), item.into()).into()
}

#[proc_macro]
pub fn gl_fn_decl(item: TokenStream) -> TokenStream {
    gl_fn_decl::gl_fn_decl(item.into()).into()
}
