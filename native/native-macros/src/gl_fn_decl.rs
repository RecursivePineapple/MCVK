use proc_macro2::TokenStream;
use quote::{format_ident, quote_spanned, ToTokens};
use syn::{
    parse::{discouraged::AnyDelimiter, Parse},
    punctuated::Punctuated,
    spanned::Spanned,
    token::{Bracket, Comma},
    *,
};

struct GlFnDeclSpec {
    pub class: LitStr,
    pub prefix: Ident,
    pub valid_param_lengths: Vec<(LitInt, usize)>,
    pub defaults: Vec<Expr>,
    pub insn: ExprClosure,
}

impl Parse for GlFnDeclSpec {
    fn parse(input: parse::ParseStream) -> Result<Self> {
        let class: LitStr = input.parse()?;

        let _: Token![,] = input.parse()?;

        let prefix: Ident = input.parse()?;

        let _: Token![,] = input.parse()?;

        let lens;
        bracketed!(lens in input);
        let lens = Punctuated::<LitInt, Token![,]>::parse_terminated(&lens)?;

        let _: Token![,] = input.parse()?;

        let defaults;
        bracketed!(defaults in input);
        let defaults = Punctuated::<Expr, Token![,]>::parse_terminated(&defaults)?;

        let _: Token![,] = input.parse()?;

        let insn: ExprClosure = input.parse()?;

        if defaults.len() != insn.inputs.len() {
            return Err(Error::new(
                insn.span(),
                format!("expected default count to equal closure param count"),
            ));
        }

        for input in &insn.inputs {
            match input {
                Pat::Ident(_) => {}
                other => {
                    return Err(Error::new(
                        other.span(),
                        format!("invalid parameter, must only be an ident"),
                    ));
                }
            }
        }

        Ok(Self {
            class,
            prefix,
            valid_param_lengths: lens
                .into_iter()
                .map(|l| l.base10_parse::<_>().map(|i| (l, i)))
                .collect::<Result<_>>()?,
            defaults: defaults.into_iter().collect(),
            insn,
        })
    }
}

impl ToTokens for GlFnDeclSpec {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        for (lit, len) in &self.valid_param_lengths {
            let class = &self.class;

            let body = &self.insn.body;

            macro_rules! emit {
                ($jtype:ident, $type:ident, $norm:expr, $suffix:literal) => {{
                    let fn_name = format_ident!("{}{}{}", self.prefix, len.to_string(), $suffix);

                    let mut params = Vec::<TokenStream>::new();
                    let mut prelude = Vec::<Stmt>::new();

                    for i in 0..self.insn.inputs.len() {
                        let input = match &self.insn.inputs[i] {
                            Pat::Ident(i) => &i.ident,
                            _ => panic!(),
                        };

                        let default = &self.defaults[i];

                        if i < *len {
                            params.push(quote_spanned! {input.span()=>
                                #input: $jtype
                            });

                            prelude.push(parse_quote_spanned! {input.span()=>
                                let #input: f32 = (#input as f32) / ($norm as f32);
                            });
                        } else {
                            prelude.push(parse_quote_spanned! {input.span()=>
                                let #input: f32 = #default;
                            });
                        }
                    }

                    let f: ItemFn = parse_quote_spanned!(lit.span()=>
                        #[jni_export(#class)]
                        pub fn #fn_name(_: JNIEnv<'_>, _: JClass<'_> #(, #params)*) {
                            #(#prelude)*
                            push_instruction(#body);
                        }
                    );

                    f.to_tokens(tokens);
                }};
            }

            emit!(jfloat, f32, 1.0f32, "f");
            emit!(jdouble, f64, 1.0f32, "d");

            emit!(jint, i32, i32::MAX, "i");
            emit!(jshort, i16, i16::MAX, "s");
            emit!(jbyte, i8, i8::MAX, "b");
            emit!(jlong, i64, i64::MAX, "l");
        }
    }
}

pub fn gl_fn_decl(item: TokenStream) -> TokenStream {
    let item: GlFnDeclSpec = match syn::parse2(item) {
        Ok(x) => x,
        Err(e) => {
            return e.to_compile_error();
        }
    };

    item.into_token_stream()
}
