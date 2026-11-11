use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, parse_macro_input};

#[derive(Default)]
struct AdminAttributes {
    label: Option<String>,
    label_lang: Option<String>,
    order: Option<u32>,
    enabled: bool,
    list: bool,
    detail: bool,
    create: bool,
    update: bool,
    delete: bool,
    filter: bool,
    search: bool,
    readonly: bool,
    hidden: bool,
}

#[proc_macro_derive(AFasterAdmin, attributes(admin))]
pub fn derive_afaster_admin(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match expand(input) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

fn expand(input: DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let ident = input.ident;
    let Data::Struct(data) = input.data else {
        return Err(syn::Error::new_spanned(
            ident,
            "AFasterAdmin can only be derived for structs",
        ));
    };
    let Fields::Named(fields) = data.fields else {
        return Err(syn::Error::new_spanned(
            ident,
            "AFasterAdmin requires a struct with named fields",
        ));
    };

    let mut model = AdminAttributes::default();
    for attribute in input
        .attrs
        .iter()
        .filter(|attribute| attribute.path().is_ident("admin"))
    {
        parse_attributes(attribute, &mut model, true)?;
    }

    let mut field_metadata = Vec::new();
    for field in fields.named {
        let field_ident = field.ident.expect("named field");
        let mut attributes = AdminAttributes::default();
        for attribute in field
            .attrs
            .iter()
            .filter(|attribute| attribute.path().is_ident("admin"))
        {
            parse_attributes(attribute, &mut attributes, false)?;
        }
        let rust_name = field_ident.to_string();
        let label = option_string(attributes.label);
        let label_lang = option_string(attributes.label_lang);
        let order = attributes
            .order
            .map(|value| quote!(Some(#value)))
            .unwrap_or_else(|| quote!(None));
        let list = attributes.list;
        let detail = attributes.detail;
        let create = attributes.create;
        let update = attributes.update;
        let filter = attributes.filter;
        let search = attributes.search;
        let readonly = attributes.readonly;
        let hidden = attributes.hidden;
        field_metadata.push(quote! {
            ::afaster::orm::AdminFieldInfo {
                rust_name: #rust_name,
                label: #label,
                label_lang: #label_lang,
                order: #order,
                list: #list,
                detail: #detail,
                create: #create,
                update: #update,
                filter: #filter,
                search: #search,
                readonly: #readonly,
                hidden: #hidden,
            }
        });
    }

    let label = option_string(model.label);
    let label_lang = option_string(model.label_lang);
    let enabled = model.enabled;
    let list = model.list;
    let detail = model.detail;
    let create = model.create;
    let update = model.update;
    let delete = model.delete;
    let tokens = quote! {
        impl ::afaster::orm::AFasterAdmin for #ident {
            const ADMIN_INFO: ::afaster::orm::AdminModelInfo =
                ::afaster::orm::AdminModelInfo {
                    rust_name: stringify!(#ident),
                    enabled: #enabled,
                    label: #label,
                    label_lang: #label_lang,
                    list: #list,
                    detail: #detail,
                    create: #create,
                    update: #update,
                    delete: #delete,
                    fields: &[#(#field_metadata),*],
                };
        }
    };
    Ok(tokens)
}

fn option_string(value: Option<String>) -> proc_macro2::TokenStream {
    value
        .map(|value| quote!(Some(#value)))
        .unwrap_or_else(|| quote!(None))
}

fn parse_attributes(
    attribute: &syn::Attribute,
    attributes: &mut AdminAttributes,
    model: bool,
) -> syn::Result<()> {
    if model {
        attributes.enabled = true;
    }
    attribute.parse_nested_meta(|meta| {
        let Some(ident) = meta.path.get_ident() else {
            return Err(meta.error("expected a simple admin attribute name"));
        };
        let name = ident.to_string();
        if meta.input.peek(syn::token::Paren) {
            return Err(meta.error("admin flags do not accept nested arguments"));
        }
        if meta.input.peek(syn::Token![=]) {
            let value = meta.value()?;
            match name.as_str() {
                "label" | "label_lang" => {
                    let value = value.parse::<syn::LitStr>()?.value();
                    if value.trim().is_empty() {
                        return Err(meta.error("admin labels and label_lang keys cannot be empty"));
                    }
                    let target = if name == "label" {
                        &mut attributes.label
                    } else {
                        &mut attributes.label_lang
                    };
                    if target.replace(value).is_some() {
                        return Err(meta.error("duplicate admin attribute"));
                    }
                }
                "order" if !model => {
                    let value = value.parse::<syn::LitInt>()?;
                    let parsed = value.base10_parse::<u32>().map_err(|_| {
                        syn::Error::new_spanned(value, "admin order must be a non-negative u32")
                    })?;
                    if attributes.order.replace(parsed).is_some() {
                        return Err(meta.error("duplicate admin order"));
                    }
                }
                "order" => return Err(meta.error("order is only valid on fields")),
                _ => return Err(meta.error("unsupported admin attribute value")),
            }
        } else {
            match name.as_str() {
                "list" => attributes.list = true,
                "detail" => attributes.detail = true,
                "create" if model => attributes.create = true,
                "create" => attributes.create = true,
                "update" => attributes.update = true,
                "delete" if model => attributes.delete = true,
                "delete" => return Err(meta.error("delete is only valid on models")),
                "filter" if !model => attributes.filter = true,
                "filter" => return Err(meta.error("filter is only valid on fields")),
                "search" if !model => attributes.search = true,
                "search" => return Err(meta.error("search is only valid on fields")),
                "readonly" if !model => attributes.readonly = true,
                "readonly" => return Err(meta.error("readonly is only valid on fields")),
                "hidden" if !model => attributes.hidden = true,
                "hidden" => return Err(meta.error("hidden is only valid on fields")),
                _ => return Err(meta.error("unsupported admin attribute")),
            }
            if model {
                attributes.enabled = true;
            }
        }
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::{AdminAttributes, parse_attributes};

    #[test]
    fn accepts_optional_labels_and_ordered_model_capabilities() {
        let model_attribute = syn::parse_quote!(
            #[admin(label_lang = "post.model", list, detail, create, update, delete)]
        );
        let mut model = AdminAttributes::default();
        parse_attributes(&model_attribute, &mut model, true).expect("valid model attributes");
        assert!(model.enabled && model.list && model.detail && model.create && model.delete);
        assert_eq!(model.label_lang.as_deref(), Some("post.model"));

        let field_attribute = syn::parse_quote!(
            #[admin(label_lang = "post.title", list, order = 7, update)]
        );
        let mut field = AdminAttributes::default();
        parse_attributes(&field_attribute, &mut field, false).expect("valid field attributes");
        assert_eq!(field.order, Some(7));
        assert!(field.list && field.update);
        assert_eq!(field.label, None);
    }

    #[test]
    fn rejects_invalid_admin_attribute_values() {
        for attribute in [
            syn::parse_quote!(#[admin(order = "first")]),
            syn::parse_quote!(#[admin(order = 4294967296)]),
            syn::parse_quote!(#[admin(label = "")]),
            syn::parse_quote!(#[admin(list = true)]),
            syn::parse_quote!(#[admin(list(nested))]),
        ] {
            let mut field = AdminAttributes::default();
            assert!(
                parse_attributes(&attribute, &mut field, false).is_err(),
                "expected invalid attribute to be rejected"
            );
        }
    }
}
