use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream}, parse_macro_input, Data, DataStruct, DeriveInput, Fields, Ident, LitStr, Token, Type, TypePath
};

struct NodeAttribute {
    crate_path: syn::Path,
}

/// Parses the attribute in the format: `crate_path = "path::to::crate"`.
impl Parse for NodeAttribute {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let key: Ident = input.parse()?;
        if key != "crate_path" {
            return Err(syn::Error::new(key.span(), "expected attribute `crate_path`"));
        }

        let _: Token![=] = input.parse()?;
        let value: LitStr = input.parse()?;
        let path: syn::Path = value.parse()?;
        
        Ok(NodeAttribute { crate_path: path })
    }
}

/// Derive macro for creating linked list nodes.
#[proc_macro_derive(Node, attributes(node))]
pub fn node_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    // Find absolute crate path
    let mut crate_path = quote! { ::mola_collections };

    for attr in &input.attrs {
        if attr.path().is_ident("node") {
            match attr.parse_args::<NodeAttribute>() {
                Ok(node_attr) => {
                    let path = node_attr.crate_path;
                    crate_path = quote! { #path };
                    break;
                }
                Err(e) => return e.to_compile_error().into(),
            }
        }
    }

    let intrusive_path = quote! { #crate_path::linked_list::intrusive };

    let mut link_field = None;
    let mut data_field = None;

    if let Data::Struct(DataStruct {
        fields: Fields::Named(ref fields),
        ..
    }) = input.data
    {
        for field in fields.named.iter() {
            if let Some(ident) = &field.ident {
                match ident.to_string().as_str() {
                    "link" => link_field = Some(field.clone()),
                    "data" => data_field = Some(field.clone()),
                    _ => {
                        return syn::Error::new_spanned(
                            ident,
                            "Unexpected field name: expected 'link' or 'data'",
                        )
                        .to_compile_error()
                        .into();
                    }
                }
            }
        }
    } else {
        return syn::Error::new_spanned(
            input,
            "Node derive macro only supports structs with named fields",
        )
        .to_compile_error()
        .into();
    };

    let link_field = match link_field {
        Some(field) => field,
        None => {
            return syn::Error::new_spanned(struct_name, "Struct must have a field named 'link'")
                .to_compile_error()
                .into();
        }
    };
    let link_type = &link_field.ty;

    let type_ident = if let Type::Path(TypePath { path, .. }) = link_type {
        path.segments
            .last()
            .expect("Expected at least one segment in the type path")
            .ident
            .clone()
    } else {
        return syn::Error::new_spanned(link_type, "Field 'link' must be a Link type")
            .to_compile_error()
            .into();
    };

    let is_double_linked = match type_ident.to_string().as_str() {
        "SingleLink" => false,
        "DoubleLink" => true,
        _ => {
            return syn::Error::new_spanned(
                type_ident,
                "Field 'link' must be one of 'SingleLink' or 'DoubleLink'",
            )
            .to_compile_error()
            .into();
        }
    };

    // Generate `Node` and `Link` trait implementations
    let single_link_impl = quote! {
        impl #impl_generics #intrusive_path::traits::Link for #struct_name #ty_generics #where_clause {
            type Target = Self;

            #[inline]
            fn next(&self) -> Option<::core::ptr::NonNull<Self::Target>> {
                self.link.next().map(|n| n.cast())
            }

            #[inline]
            fn set_next(&mut self, next: Option<::core::ptr::NonNull<Self::Target>>) {
                self.link.set_next(next.map(|n| n.cast()));
            }
        }

        impl #impl_generics #intrusive_path::traits::Node for #struct_name #ty_generics #where_clause {
            #[inline]
            fn append_to<L>(&mut self, list: &mut L)
            where
                L: #intrusive_path::traits::List<Target = Self>,
            {
                unsafe {
                    let mut wrapper = #intrusive_path::wrapper::ListWrapper::new(list);
                    self.link.append_to(&mut wrapper);
                }
            }

            #[inline]
            unsafe fn detach<L>(&mut self, parent: Option<&mut L>)
            where
                L: #intrusive_path::traits::Link<Target = Self>,
            {
                unsafe {
                    let mut parent_wrapper = parent.map(|p| #intrusive_path::wrapper::LinkWrapper::new(p));
                    self.link.detach(parent_wrapper.as_mut());
                }
            }
        }
    };

    // Generate `LinkWithPrev` trait implementation for `DoubleLink`
    let double_link_impl = if is_double_linked {
        quote! {
            impl #impl_generics #intrusive_path::traits::LinkWithPrev for #struct_name #ty_generics #where_clause {
                #[inline]
                fn prev(&self) -> Option<::core::ptr::NonNull<Self>> {
                    self.link.prev().map(|n| n.cast())
                }

                #[inline]
                fn set_prev(&mut self, prev: Option<::core::ptr::NonNull<Self>>) {
                    self.link.set_prev(prev.map(|n| n.cast()));
                }
            }
        }
    } else {
        quote! {}
    };

    // Generate `NodeWithData` trait implementation if `data` field exists
    let data_impl = if let Some(data_field) = data_field {
        let data_type = &data_field.ty;
        quote! {
            impl #impl_generics #intrusive_path::traits::NodeWithData for #struct_name #ty_generics #where_clause {
                type Data = #data_type;

                #[inline]
                fn data(&self) -> &Self::Data {
                    &self.data
                }

                #[inline]
                fn data_mut(&mut self) -> &mut Self::Data {
                    &mut self.data
                }
            }
        }
    } else {
        quote! {}
    };

    let expanded = quote! {
        #single_link_impl
        #double_link_impl
        #data_impl
    };

    TokenStream::from(expanded)
}