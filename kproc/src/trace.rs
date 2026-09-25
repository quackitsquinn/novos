use std::io::{Read, Write};

use rustix::fs::FlockOperation;
use syn::{Expr, LitStr};

pub fn derive_should_trace_target(expr: LitStr) -> proc_macro::TokenStream {
    let log_filters = option_env!("TAU_TRACE_FEATURES")
        .map(|features| {
            features
                .split(',')
                .map(str::trim)
                .map(str::to_lowercase)
                .filter(|s| !s.is_empty())
                .map(|s| syn::LitStr::new(&s, proc_macro2::Span::call_site()))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let match_arms = log_filters.iter().map(|filter| {
        quote::quote! {
            #filter => true,
        }
    });

    let _ = add_trace_target(&expr);

    quote::quote! {
        match #expr {
            #(#match_arms)*
            _ => false,
        }
    }
    .into()
}

fn add_trace_target(expr: &LitStr) -> std::io::Result<()> {
    let target = expr.value();

    let mut list = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open("target/trace_targets.txt")?;

    rustix::fs::flock(&list, FlockOperation::LockExclusive)?;

    let mut contents = String::new();
    list.read_to_string(&mut contents)?;
    if contents.contains(&target) {
        rustix::fs::flock(&list, FlockOperation::Unlock)?;
        return Ok(());
    }

    writeln!(list, "{}", target)?;
    rustix::fs::flock(&list, FlockOperation::Unlock)?;

    Ok(())
}
