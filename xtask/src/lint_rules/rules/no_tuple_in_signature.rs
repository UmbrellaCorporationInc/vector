//! RULE-5: Type Aliases for Complex Signatures.
//!
//! Three conditions are checked on every `fn` item:
//!
//! - **RULE-5A**: flag any tuple with 2+ elements in any `fn` signature (parameters or return type); exempt `*_test.rs`.
//! - **RULE-5B**: flag `pub` fn signatures containing a generic type with 2+ type params not in the wrapper exemption list (Arc, Rc, Mutex, `RwLock`, Result, Option).
//! - **RULE-5C**: flag any `impl Trait` in any `fn` signature whose trait carries at least one concrete generic argument (e.g. `impl Iterator<Item = String>`); concrete means not a bare uppercase type parameter. Use a type alias instead.

use std::path::Path;

use syn::spanned::Spanned;
use syn::visit::{self, Visit};

use crate::lint_rules::rule::{Rule, RuleViolation};

pub struct NoTupleInSignature;

impl Rule for NoTupleInSignature {
    fn is_active(&self, _future: bool) -> bool {
        true
    }

    fn check_rust(&self, path: &Path, ast: &syn::File, _raw: &str, out: &mut Vec<RuleViolation>) {
        // RULE-5A is exempt for test files.
        // We skip the whole file if it's a test file.
        if path.to_string_lossy().ends_with("_test.rs") {
            return;
        }

        let mut visitor = Visitor { path, out, in_pub_trait: false };
        visitor.visit_file(ast);
    }
}

// ─── visitor ────────────────────────────────────────────────────────────────

struct Visitor<'a> {
    path: &'a Path,
    out: &'a mut Vec<RuleViolation>,
    in_pub_trait: bool,
}

impl Visit<'_> for Visitor<'_> {
    fn visit_item_trait(&mut self, node: &syn::ItemTrait) {
        let old = self.in_pub_trait;
        self.in_pub_trait = matches!(node.vis, syn::Visibility::Public(_));
        visit::visit_item_trait(self, node);
        self.in_pub_trait = old;
    }

    fn visit_item_fn(&mut self, node: &syn::ItemFn) {
        let is_pub = matches!(node.vis, syn::Visibility::Public(_));
        self.check_fn(is_pub, &node.sig);
        visit::visit_item_fn(self, node);
    }

    fn visit_impl_item_fn(&mut self, node: &syn::ImplItemFn) {
        let is_pub = matches!(node.vis, syn::Visibility::Public(_));
        self.check_fn(is_pub, &node.sig);
        visit::visit_impl_item_fn(self, node);
    }

    fn visit_trait_item_fn(&mut self, node: &syn::TraitItemFn) {
        // Trait methods are public if the trait is public.
        self.check_fn(self.in_pub_trait, &node.sig);
        visit::visit_trait_item_fn(self, node);
    }

    fn visit_local(&mut self, _node: &syn::Local) {
        // RULE-5: Tuple destructuring in `let` bindings is not part of a function signature.
        // We explicitly skip these to avoid any false positives.
    }
}

impl Visitor<'_> {
    fn check_fn(&mut self, is_pub: bool, sig: &syn::Signature) {
        // Check inputs (parameters)
        for arg in &sig.inputs {
            if let syn::FnArg::Typed(pat_type) = arg {
                self.check_type_recursive(&pat_type.ty, "parameter", is_pub);
            }
        }

        // Check output (return type)
        if let syn::ReturnType::Type(_, ty) = &sig.output {
            self.check_type_recursive(ty, "return type", is_pub);
        }
    }

    fn check_type_recursive(&mut self, ty: &syn::Type, context: &str, is_pub: bool) {
        match ty {
            syn::Type::Tuple(syn::TypeTuple { elems, .. }) => {
                // RULE-5A: Tuples with 2+ elements in signatures.
                // Exempt tuples whose elements contain `impl Trait`: those cannot be
                // expressed as a type alias in stable Rust, so requiring one is wrong.
                let has_impl_trait = elems.iter().any(contains_impl_trait);
                if elems.len() >= 2 && !has_impl_trait {
                    let span = ty.span();
                    self.out.push(RuleViolation {
                        file: self.path.to_path_buf(),
                        line: Some(span.start().line as u32),
                        column: Some(span.start().column as u32 + 1),
                        rule_id: "RULE-5A",
                        message: format!(
                            "Function {context} contains a tuple with {} elements — use a type alias instead",
                            elems.len()
                        ),
                    });
                }
                // Recurse into tuple elements only when there is no impl Trait involved,
                // to avoid spurious RULE-5C hits on types that are already exempt.
                if !has_impl_trait {
                    for elem in elems {
                        self.check_type_recursive(elem, context, is_pub);
                    }
                }
            }
            syn::Type::Path(syn::TypePath { qself: None, path }) => {
                if let Some(segment) = path.segments.last() {
                    let name = segment.ident.to_string();

                    // RULE-5B: Public generic types with 2+ params
                    if is_pub {
                        let exempt = ["Arc", "Rc", "Mutex", "RwLock", "Result", "Option"];
                        if !exempt.contains(&name.as_str())
                            && let syn::PathArguments::AngleBracketed(args) = &segment.arguments
                        {
                            // Only count type args that are concrete types, not bare generic type
                            // parameters like `T`, `RQ`, `M`, `RS`. A bare type param is a
                            // single-segment path with no angle-bracket arguments of its own
                            // whose name is composed entirely of uppercase ASCII letters (the
                            // universal Rust convention for type parameters). Concrete types like
                            // `String` or `HashMap` always contain at least one lowercase letter.
                            let param_count = args.args.iter().filter(|arg| {
                                if let syn::GenericArgument::Type(syn::Type::Path(tp)) = arg {
                                    let is_bare_type_param = tp.qself.is_none()
                                        && tp.path.segments.len() == 1
                                        && matches!(
                                            tp.path.segments[0].arguments,
                                            syn::PathArguments::None
                                        )
                                        && tp.path.segments[0]
                                            .ident
                                            .to_string()
                                            .chars()
                                            .all(|c| c.is_ascii_uppercase());
                                    !is_bare_type_param
                                } else {
                                    matches!(arg, syn::GenericArgument::Type(_))
                                }
                            }).count();

                            if param_count >= 2 {
                                let span = ty.span();
                                self.out.push(RuleViolation {
                                    file: self.path.to_path_buf(),
                                    line: Some(span.start().line as u32),
                                    column: Some(span.start().column as u32 + 1),
                                    rule_id: "RULE-5B",
                                    message: format!(
                                        "Public function {context} contains complex generic `{name}` with {param_count} type parameters — use a type alias instead"
                                    ),
                                });
                            }
                        }
                    }

                    // Recurse into generic arguments regardless of pub
                    if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                        for arg in &args.args {
                            if let syn::GenericArgument::Type(t) = arg {
                                self.check_type_recursive(t, context, is_pub);
                            }
                        }
                    }
                }
            }
            syn::Type::Reference(r) => {
                self.check_type_recursive(&r.elem, context, is_pub);
            }
            syn::Type::Slice(s) => {
                self.check_type_recursive(&s.elem, context, is_pub);
            }
            syn::Type::Array(a) => {
                self.check_type_recursive(&a.elem, context, is_pub);
            }
            syn::Type::Ptr(p) => {
                self.check_type_recursive(&p.elem, context, is_pub);
            }
            syn::Type::ImplTrait(impl_trait) => {
                self.check_impl_trait(impl_trait, ty, context);
            }
            _ => {}
        }
    }

    fn check_impl_trait(
        &mut self,
        impl_trait: &syn::TypeImplTrait,
        ty: &syn::Type,
        context: &str,
    ) {
        for bound in &impl_trait.bounds {
            let syn::TypeParamBound::Trait(trait_bound) = bound else { continue };
            let Some(segment) = trait_bound.path.segments.last() else { continue };

            // Count concrete args the same way RULE-5B does: >= 2 required to fire.
            let concrete_count = match &segment.arguments {
                syn::PathArguments::AngleBracketed(args) => args.args.iter().filter(|arg| {
                    match arg {
                        syn::GenericArgument::Type(t) => is_concrete_type(t),
                        syn::GenericArgument::AssocType(assoc) => is_concrete_type(&assoc.ty),
                        _ => false,
                    }
                }).count(),
                syn::PathArguments::Parenthesized(args) => {
                    // `impl Fn(A, B) -> C` — count concrete inputs + concrete output
                    let inputs = args.inputs.iter().filter(|t| is_concrete_type(t)).count();
                    let output = match &args.output {
                        syn::ReturnType::Type(_, t) => usize::from(is_concrete_type(t)),
                        syn::ReturnType::Default => 0,
                    };
                    inputs + output
                }
                syn::PathArguments::None => 0,
            };

            if concrete_count >= 2 {
                let span = ty.span();
                self.out.push(RuleViolation {
                    file: self.path.to_path_buf(),
                    line: Some(span.start().line as u32),
                    column: Some(span.start().column as u32 + 1),
                    rule_id: "RULE-5C",
                    message: format!(
                        "Function {context} uses `impl Trait` with {concrete_count} concrete generic arguments — define a type alias and use `impl Alias` instead"
                    ),
                });
                return;
            }
        }
    }
}

/// Returns true when `ty` is or contains an `impl Trait` anywhere in its tree.
fn contains_impl_trait(ty: &syn::Type) -> bool {
    match ty {
        syn::Type::ImplTrait(_) => true,
        syn::Type::Tuple(t) => t.elems.iter().any(contains_impl_trait),
        syn::Type::Reference(r) => contains_impl_trait(&r.elem),
        syn::Type::Slice(s) => contains_impl_trait(&s.elem),
        syn::Type::Array(a) => contains_impl_trait(&a.elem),
        syn::Type::Ptr(p) => contains_impl_trait(&p.elem),
        syn::Type::Path(tp) => {
            if let Some(seg) = tp.path.segments.last()
                && let syn::PathArguments::AngleBracketed(args) = &seg.arguments
            {
                return args.args.iter().any(|arg| {
                    if let syn::GenericArgument::Type(t) = arg {
                        contains_impl_trait(t)
                    } else {
                        false
                    }
                });
            }
            false
        }
        _ => false,
    }
}

/// Returns true when `ty` is NOT a bare type parameter.
///
/// A bare type parameter is a single-segment path, no angle-bracket args,
/// composed entirely of ASCII uppercase letters (e.g. `T`, `RQ`, `ERR`).
fn is_concrete_type(ty: &syn::Type) -> bool {
    let syn::Type::Path(tp) = ty else { return true };
    if tp.qself.is_some() || tp.path.segments.len() != 1 {
        return true;
    }
    let seg = &tp.path.segments[0];
    if !matches!(seg.arguments, syn::PathArguments::None) {
        return true;
    }
    !seg.ident.to_string().chars().all(|c| c.is_ascii_uppercase())
}

#[cfg(test)]
#[path = "no_tuple_in_signature_test.rs"]
mod tests;
