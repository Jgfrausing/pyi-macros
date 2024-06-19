#![allow(dead_code)]

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse_macro_input, Attribute, DataStruct, Expr, ImplItem, ItemImpl, Lit, PathSegment,
    ReturnType, Type,
};

fn any() -> Option<String> {
    Some("Any".to_string())
}
fn option_type(class_name: &str, segment: &PathSegment) -> Option<String> {
    if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
        if let Some(syn::GenericArgument::Type(inner_type)) = args.args.first() {
            Some(format!(
                "Optional[{}]",
                rust_to_python_type(class_name, inner_type)?
            ))
        } else {
            Some("Optional[Any]".to_string())
        }
    } else {
        Some("Optional[Any]".to_string())
    }
}

fn rust_to_python_type(class_name: &str, ty: &Type) -> Option<String> {
    match ty {
        Type::Path(type_path) => {
            let segment = &type_path.path.segments[0];
            let type_name = segment.ident.to_string();

            match type_name.as_str() {
                "Python" => None,
                "Self" => Some(class_name.to_string()),
                "i32" => Some("int".to_string()),
                "i64" => Some("int".to_string()),
                "f64" => Some("float".to_string()),
                "PyObject" => Some("Any".to_string()),
                "String" => Some("str".to_string()),
                "Option" => option_type(class_name, segment),
                "PyResult" => {
                    if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                        if let Some(syn::GenericArgument::Type(inner_type)) = args.args.first() {
                            rust_to_python_type(class_name, inner_type)
                        } else {
                            any()
                        }
                    } else {
                        any()
                    }
                }
                "Result" => {
                    if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                        let mut args_iter = args.args.iter();
                        let ok_type = args_iter.next();
                        let err_type = args_iter.next();

                        let ok_type_str =
                            if let Some(syn::GenericArgument::Type(inner_type)) = ok_type {
                                rust_to_python_type(class_name, inner_type)?
                            } else {
                                any()?
                            };

                        let err_type_str =
                            if let Some(syn::GenericArgument::Type(inner_type)) = err_type {
                                rust_to_python_type(class_name, inner_type)?
                            } else {
                                any()?
                            };

                        Some(format!("Union[{}, {}]", ok_type_str, err_type_str))
                    } else {
                        any()
                    }
                }
                _ => Some(type_name), // Default case, return the Rust type as is
            }
        }
        _ => any(),
    }
}
fn extract_doc_comments(attrs: &[Attribute], indent: String, postfix: &'static str) -> String {
    fn extract_token_string(expr: &Expr) -> Option<String> {
        if let Expr::Lit(syn::ExprLit {
            lit: Lit::Str(lit_str),
            ..
        }) = expr
        {
            let value = lit_str.value().clone().to_string();
            Some(value)
        } else {
            None
        }
    }
    let mut doc_lines = Vec::new();

    for attr in attrs {
        if attr.path().is_ident("doc") {
            if let syn::Meta::NameValue(meta_name_value) = &attr.meta {
                if let Some(comment) = extract_token_string(&meta_name_value.value) {
                    doc_lines.push(comment.trim_start().to_string());
                }
            }
        }
    }
    if doc_lines.is_empty() {
        postfix.to_string()
    } else {
        let doc_marker = indent.to_string() + "\"\"\"\n";
        format!(
            "\n{}{}{}\n{}{}{}",
            doc_marker,
            indent,
            doc_lines.join(format!("\n{}", indent).as_str()),
            doc_marker,
            indent,
            postfix.trim()
        )
    }
}

fn attributes_contain(attr: &[Attribute], value: &'static str) -> bool {
    format!("{:?}", attr).contains(value)
}

const INDENT: &str = "    ";
pub fn pyi_impl(item: TokenStream) -> TokenStream {
    use quote::ToTokens;
    let input = parse_macro_input!(item as ItemImpl);
    let struct_name = &input.self_ty.clone().into_token_stream().to_string();
    let struct_name = &struct_name;
    let double_indent = INDENT.to_string() + INDENT;
    let mut methods: Vec<(String, String)> = input
        .items
        .iter()
        .filter_map(|item| {
            if let ImplItem::Fn(method) = item {
                let fn_name = &method.sig.ident;
                let fn_args = &method.sig.inputs;
                let fn_return = &method.sig.output;
                let doc_comment =
                    extract_doc_comments(&method.attrs, double_indent.to_string(), " ...");

                // Generate Python function signature
                let mut py_args: Vec<String> = fn_args
                    .iter()
                    .filter_map(|arg| match arg {
                        syn::FnArg::Typed(pat_type) => {
                            let ty = &pat_type.ty;
                            let py_type = rust_to_python_type(struct_name, ty)?;
                            let pat = &pat_type.pat;

                            Some(format!("{}: {}", quote! {#pat}, py_type))
                        }
                        syn::FnArg::Receiver(_) => None,
                    })
                    .collect();

                // check if attribute is pyo3::staticmethod
                let is_static = format!("{:?}", &method.attrs).contains("staticmethod");

                let static_metod_attribute = if is_static {
                    format!("{}@staticmethod\n", INDENT)
                } else {
                    py_args.insert(0, "self".to_string());
                    "".to_string()
                };

                let py_return = match (fn_name.to_string(), fn_return) {
                    (init, _) if &init == "__init__" => "None".to_string(),
                    (_, ReturnType::Default) => "None".to_string(),
                    (_, ReturnType::Type(_, ty)) => {
                        rust_to_python_type(struct_name, ty).unwrap_or("None".to_string())
                    }
                };

                let signature = format!(
                    "{}{}def {}({}) -> {}:{}",
                    static_metod_attribute,
                    INDENT,
                    fn_name,
                    py_args.join(", "),
                    py_return,
                    doc_comment,
                );
                Some((fn_name.to_string(), signature))
            } else {
                None
            }
        })
        .collect();

    methods.sort_by_key(|i| i.0.clone());

    if !methods.is_empty() {
        let py_impl_def = format!(
            "\n\n{}",
            methods
                .iter()
                .map(|m| m.1.to_string())
                .collect::<Vec<String>>()
                .join("\n\n")
        );

        super::write_file(py_impl_def);
    }
    let output = quote! {
        #input
    };

    TokenStream::from(output)
}

fn is_pyo3_getter_setter(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| attr.path().is_ident("pyo3"))
}

pub fn struct_def(item: &DataStruct, name: &str) {
    let mut properties = Vec::new();

    if let syn::Fields::Named(fields_named) = &item.fields {
        for field in &fields_named.named {
            if is_pyo3_getter_setter(&field.attrs) {
                let field_name = field.ident.as_ref().unwrap();
                let field_type = rust_to_python_type(name, &field.ty).unwrap_or("Any".to_string());
                let doc_comment = extract_doc_comments(&field.attrs, INDENT.to_string(), "");

                properties.push(format!(
                    "{}{}: {}{}",
                    INDENT, field_name, field_type, doc_comment
                ));
            }
        }
    }

    let properties_seperator = if properties.is_empty() { "" } else { "\n" };
    let py_class_def = format!(
        "\n\nclass {}:{}{}",
        name,
        properties_seperator,
        properties.join("\n")
    );

    super::write_file(py_class_def);
}

pub fn enum_def(item: &syn::DataEnum, name: &str) {
    let mut variants = Vec::new();
    for variant in &item.variants {
        let variant_name = &variant.ident;
        let doc_comment = extract_doc_comments(&variant.attrs, INDENT.to_string(), "");
        variants.push(format!(
            "{}{} = '{}'{}",
            INDENT, variant_name, variant_name, doc_comment
        ));
    }
    let py_class_def = format!("\n\nclass {}(Enum):\n{}", name, variants.join("\n"));
    super::write_file(py_class_def);
}
