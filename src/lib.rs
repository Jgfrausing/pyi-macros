extern crate proc_macro;

mod pyi;
use proc_macro::TokenStream;
use quote::quote;
use syn::parse_macro_input;

#[proc_macro_attribute]
pub fn pyi_impl(_: TokenStream, item: TokenStream) -> TokenStream {
    pyi::pyi_impl(item)
}

#[proc_macro_attribute]
pub fn pyi(_: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as syn::DeriveInput);
    let name = input.ident.to_string();
    let name = name.as_str();

    match &input.data {
        syn::Data::Struct(item) => {
            pyi::struct_def(item, name);
        }
        syn::Data::Enum(item) => {
            pyi::enum_def(item, name);
        }
        _ => {}
    }

    let output = quote! {
        #input
    };

    TokenStream::from(output)
}

fn get_file_path(module: &'static str) -> std::path::PathBuf {
    let mut path = std::env::current_dir().unwrap();
    path.push(module);
    path.push(format!("{}.pyi", module));
    path
}

static FILE_WRITER: std::sync::Once = std::sync::Once::new();
fn write_file(content: String) {
    let path = get_file_path("INSERT_MODULE_NAME_HERE");
    FILE_WRITER.call_once(|| {
        let imports = [
            "from datetime import datetime as DateTime",
            "from typing import *",
            "from enum import Enum",
        ];
        std::fs::write(&path, imports.join("\n")).unwrap();
    });
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open(&path)
        .unwrap();

    write!(file, "{}", content).unwrap();
}
