use super::mod_context::EnumItem;
use super::mod_context::FnItem;
use super::mod_context::ImplItem;
use super::mod_context::InnerConstItem;
use super::mod_context::InnerFnItem;
use super::mod_context::InnerTypeItem;
use super::mod_context::ModContext;
use super::mod_context::StructItem;
use super::mod_context::TraitItem;
use super::mod_context::UnionItem;
use super::parses::parse_body;
use super::parses::parse_fn_decl;
use super::parses::parse_generic_bound;
use super::parses::parse_generic_param;
use super::parses::parse_path;
use super::parses::parse_variant;
use super::parses::parse_variant_data;
use super::parses::parse_where_predicate;
use super::parses::recursively_parse_ty;
use crate::analysis::source_info;
use crate::analysis::source_info::SourceInfo;
use log::error;
use log::info;
use log::warn;
use rustc_ast::ast::AttrKind;
use rustc_hir::def::Res;
use rustc_hir::intravisit::Visitor;
use rustc_hir::GenericBound;
use rustc_hir::ImplItemKind;
use rustc_hir::ImplItemRef;
use rustc_hir::Item;
use rustc_hir::ItemKind;
use rustc_hir::QPath;
use rustc_hir::TraitFn;
use rustc_hir::TraitItemKind;
use rustc_hir::TraitItemRef;
use rustc_hir::TyKind;
use rustc_middle::hir::map::Map;
use rustc_middle::hir::nested_filter;
use rustc_middle::ty::TyCtxt;
use std::collections::BTreeSet;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use crate::OUT_FILE_PATH;

fn is_impl(codes: &str) -> bool {
    match syn::parse_str::<syn::Item>(codes) {
        Ok(syn::Item::Impl(_)) => true,
        _ => false,
    }
}

pub struct HirVisitor<'tcx> {
    tcx: TyCtxt<'tcx>,
    hir_map: Map<'tcx>,
    mod_contexts: Vec<ModContext>,
    complete_mod_contexts: Vec<ModContext>,
    inner_fn_item: Option<InnerFnItem>,
    crate_path: PathBuf,
}

impl<'tcx> HirVisitor<'tcx> {
    pub fn new(tcx: TyCtxt<'tcx>, hir_map: Map<'tcx>, crate_path: PathBuf) -> Self {
        HirVisitor {
            tcx: tcx,
            hir_map: hir_map,
            mod_contexts: Vec::new(),
            complete_mod_contexts: Vec::new(),
            inner_fn_item: None,
            crate_path: crate_path,
        }
    }

    pub fn get_complete_mod_contexts(&self) -> Vec<ModContext> {
        self.complete_mod_contexts.clone()
    }
}

impl<'tcx> Visitor<'tcx> for HirVisitor<'tcx> {
    type NestedFilter = nested_filter::All;

    fn nested_visit_map(&mut self) -> Self::Map {
        self.tcx.hir().clone()
    }

    fn visit_mod(
        &mut self,
        m: &'tcx rustc_hir::Mod<'tcx>,
        s: rustc_span::Span,
        n: rustc_hir::HirId,
    ) -> Self::Result {
        let def_id = n.owner.to_def_id();
        let mut module_name = self.tcx.def_path(def_id).to_string_no_crate_verbose();
        module_name = self.tcx.crate_name(def_id.krate).to_string() + &module_name;
        info!("Visiting module: {}", module_name);
        let mod_context = ModContext::new(&module_name);
        self.mod_contexts.push(mod_context);
        // intravisit::walk_mod(self, m, n);
        for item_id in m.item_ids {
            self.visit_item(self.tcx.hir().item(*item_id));
        }
        let mut mod_context = self.mod_contexts.pop().unwrap();
        mod_context.derive_to_codes();
        self.complete_mod_contexts.push(mod_context);
        info!("Leaving module: {}", module_name);
    }

    fn visit_trait_item_ref(&mut self, ii: &'tcx TraitItemRef) -> Self::Result {
        let ref_trait_item = self.hir_map.trait_item(ii.id);

        let def_id = ref_trait_item.owner_id.to_def_id();
        let mut fn_name = self.tcx.def_path(def_id).to_string_no_crate_verbose();
        fn_name = self.tcx.crate_name(def_id.krate).to_string() + &fn_name;

        let fn_source = SourceInfo::from_span(ref_trait_item.span, self.tcx.sess.source_map());
        let codes = fn_source.get_string();

        let mut ty_strings: BTreeSet<String> = BTreeSet::new();

        let generics = ref_trait_item.generics;
        for param in generics.params.iter() {
            parse_generic_param(&self.tcx, param, &mut ty_strings);
        }
        for predicate in generics.predicates.iter() {
            parse_where_predicate(&self.tcx, predicate, &mut ty_strings);
        }
        match ref_trait_item.kind {
            TraitItemKind::Fn(fn_sig, trait_fn) => {
                parse_fn_decl(&self.tcx, fn_sig.decl, &mut ty_strings);
                match trait_fn {
                    TraitFn::Provided(body_id) => {
                        let body = self.hir_map.body(body_id);
                        parse_body(&self.tcx, &self.hir_map, &body, &mut ty_strings);
                    }
                    _ => {}
                }
            }
            _ => {
                error!("Trait fn type error");
            }
        }

        let inner_fn_item = InnerFnItem {
            name: fn_name,
            codes: codes,
            source_info: fn_source,
            applications: ty_strings,
        };
        self.inner_fn_item = Some(inner_fn_item);
    }

    fn visit_impl_item_ref(&mut self, ii: &'tcx ImplItemRef) -> Self::Result {
        let ref_impl_item = self.hir_map.impl_item(ii.id);

        let def_id = ref_impl_item.owner_id.to_def_id();
        let mut fn_name = self.tcx.def_path(def_id).to_string_no_crate_verbose();
        fn_name = self.tcx.crate_name(def_id.krate).to_string() + &fn_name;

        let fn_source = SourceInfo::from_span(ref_impl_item.span, self.tcx.sess.source_map());
        let codes = fn_source.get_string();

        let mut ty_strings: BTreeSet<String> = BTreeSet::new();

        let generics = ref_impl_item.generics;
        for param in generics.params.iter() {
            parse_generic_param(&self.tcx, param, &mut ty_strings);
        }
        for predicate in generics.predicates.iter() {
            parse_where_predicate(&self.tcx, predicate, &mut ty_strings);
        }
        match ref_impl_item.kind {
            ImplItemKind::Fn(fn_sig, body_id) => {
                parse_fn_decl(&self.tcx, fn_sig.decl, &mut ty_strings);
                let body = self.hir_map.body(body_id);
                parse_body(&self.tcx, &self.hir_map, &body, &mut ty_strings);
            }
            _ => {
                error!("Trait fn type error");
            }
        }

        let inner_fn_item = InnerFnItem {
            name: fn_name,
            codes: codes,
            source_info: fn_source,
            applications: ty_strings,
        };
        self.inner_fn_item = Some(inner_fn_item);
    }

    fn visit_item(&mut self, i: &'tcx Item<'tcx>) -> Self::Result {
        // println!("{:#?}", i);
        // let hir_id = self.tcx.local_def_id_to_hir_id(i.item_id().owner_id.def_id);
        let hir_id = i.item_id().hir_id();
        let attrs = self.hir_map.attrs(hir_id);
        // error!("{:?}: {:#?}", hir_id, attrs);
        let mut attrs_string = String::new();
        for attr in attrs {
            if let AttrKind::Normal(_) = &attr.kind {
                // let attr_source = SourceInfo::from_span(attr.span, self.tcx.sess.source_map());
                // error!("attr source: {:?}", attr_source);
                // attrs_string = attrs_string + &attr_source.get_string();
                let snippet = self
                    .tcx
                    .sess
                    .source_map()
                    .span_to_snippet(attr.span)
                    .unwrap();
                if snippet == "" || !snippet.contains("#[") {
                    continue;
                }
                // error!("attr source: {}", snippet);
                attrs_string = attrs_string + &snippet + "\n";
            }
            // let attr_item = attr.get_normal_item();
            // attrs_string = attrs_string + &format!("{:?}", attr_item) + "\n";
        }
        match i.kind {
            ItemKind::ExternCrate(..) => {
                let extern_crate_source = SourceInfo::from_span(i.span, self.tcx.sess.source_map());
                let mut codes = extern_crate_source.get_string();
                codes = attrs_string + &codes;
                info!("Visiting extern crate: {}", codes);
                self.mod_contexts
                    .last_mut()
                    .unwrap()
                    .add_extern_crate(extern_crate_source, codes);
            }
            ItemKind::Use(..) => {
                let use_source = SourceInfo::from_span(i.span, self.tcx.sess.source_map());
                let mut codes = use_source.get_string();
                codes = attrs_string + &codes;
                info!("Visiting use: {}", codes);
                self.mod_contexts
                    .last_mut()
                    .unwrap()
                    .add_use(use_source, codes);
            }
            ItemKind::Static(ty, _, body_id) => {
                let static_source = SourceInfo::from_span(i.span, self.tcx.sess.source_map());
                let mut codes = static_source.get_string();
                codes = attrs_string + &codes;
                info!("Visiting static: {}", codes);
                self.mod_contexts
                    .last_mut()
                    .unwrap()
                    .add_static(static_source, codes);

                let mut ty_strings: BTreeSet<String> = BTreeSet::new();
                recursively_parse_ty(&self.tcx, &ty, &mut ty_strings);
                let body = self.hir_map.body(body_id);
                parse_body(&self.tcx, &self.hir_map, &body, &mut ty_strings);
                self.mod_contexts
                    .last_mut()
                    .unwrap()
                    .extend_application(ty_strings);
            }
            ItemKind::Const(ty, generics, body_id) => {
                let const_source = SourceInfo::from_span(i.span, self.tcx.sess.source_map());
                let mut codes = const_source.get_string();
                codes = attrs_string + &codes;
                info!("Visiting const: {}", codes);
                self.mod_contexts
                    .last_mut()
                    .unwrap()
                    .add_const(const_source, codes);

                let mut ty_strings: BTreeSet<String> = BTreeSet::new();
                recursively_parse_ty(&self.tcx, &ty, &mut ty_strings);
                for param in generics.params.iter() {
                    parse_generic_param(&self.tcx, param, &mut ty_strings);
                }
                for predicate in generics.predicates.iter() {
                    parse_where_predicate(&self.tcx, predicate, &mut ty_strings);
                }
                let body = self.hir_map.body(body_id);
                parse_body(&self.tcx, &self.hir_map, &body, &mut ty_strings);
                self.mod_contexts
                    .last_mut()
                    .unwrap()
                    .extend_application(ty_strings);
            }
            ItemKind::Fn(fn_sig, generics, body_id) => {
                let def_id = i.owner_id.to_def_id();
                let mut fn_name = self.tcx.def_path(def_id).to_string_no_crate_verbose();
                fn_name = self.tcx.crate_name(def_id.krate).to_string() + &fn_name;
                info!("Visiting fn: {}", fn_name);
                let fn_source = SourceInfo::from_span(i.span, self.tcx.sess.source_map());
                let mut codes = fn_source.get_string();
                codes = attrs_string + &codes;
                let mut ty_strings: BTreeSet<String> = BTreeSet::new();

                let decl = fn_sig.decl;
                parse_fn_decl(&self.tcx, decl, &mut ty_strings);

                for param in generics.params.iter() {
                    parse_generic_param(&self.tcx, param, &mut ty_strings);
                }
                for predicate in generics.predicates.iter() {
                    parse_where_predicate(&self.tcx, predicate, &mut ty_strings);
                }

                // println!("{:#?}", self.hir_map.body(body_id));

                // let output_path = self
                //     .crate_path
                //     .join(format!("{}/{}.txt", OUT_FILE_PATH, fn_name));
                // fs::create_dir_all(output_path.parent().unwrap()).unwrap();
                // let mut file = File::create(&output_path).unwrap();
                // file.write_all(format!("{:#?}", self.hir_map.body(body_id)).as_bytes())
                //     .unwrap();

                parse_body(
                    &self.tcx,
                    &self.hir_map,
                    self.hir_map.body(body_id),
                    &mut ty_strings,
                );

                let fn_item = FnItem {
                    name: fn_name,
                    codes: codes,
                    source_info: fn_source,
                    applications: ty_strings,
                };
                self.mod_contexts.last_mut().unwrap().add_fn(fn_item);
            }
            ItemKind::Macro(..) => {
                let macro_info = SourceInfo::from_span(i.span, self.tcx.sess.source_map());
                let mut codes = macro_info.get_string();
                codes = attrs_string + &codes;
                info!("Visiting macro: {}", codes);
                self.mod_contexts
                    .last_mut()
                    .unwrap()
                    .add_macro(macro_info, codes);
            }
            ItemKind::Mod(a_mod) => {
                // intravisit::walk_item(self, i);
                self.visit_mod(a_mod, i.span, i.hir_id());
            }
            ItemKind::TyAlias(ty, generics) => {
                let ty_alias_source = SourceInfo::from_span(i.span, self.tcx.sess.source_map());
                let mut codes = ty_alias_source.get_string();
                codes = attrs_string + &codes;
                info!("Visiting tyalias: {}", codes);
                self.mod_contexts
                    .last_mut()
                    .unwrap()
                    .add_ty_alias(ty_alias_source, codes);

                let mut ty_strings: BTreeSet<String> = BTreeSet::new();
                recursively_parse_ty(&self.tcx, &ty, &mut ty_strings);
                for param in generics.params.iter() {
                    parse_generic_param(&self.tcx, param, &mut ty_strings);
                }
                for predicate in generics.predicates.iter() {
                    parse_where_predicate(&self.tcx, predicate, &mut ty_strings);
                }
                self.mod_contexts
                    .last_mut()
                    .unwrap()
                    .extend_application(ty_strings);
            }
            ItemKind::OpaqueTy(opaque_ty) => {
                let opaque_ty_source = SourceInfo::from_span(i.span, self.tcx.sess.source_map());
                let mut codes = opaque_ty_source.get_string();
                codes = attrs_string + &codes;
                info!("Visiting opaquety: {}", codes);
                self.mod_contexts
                    .last_mut()
                    .unwrap()
                    .add_opaque_ty(opaque_ty_source, codes);

                let mut ty_strings: BTreeSet<String> = BTreeSet::new();
                let generics = opaque_ty.generics;
                for param in generics.params.iter() {
                    parse_generic_param(&self.tcx, param, &mut ty_strings);
                }
                for predicate in generics.predicates.iter() {
                    parse_where_predicate(&self.tcx, predicate, &mut ty_strings);
                }

                for bound in opaque_ty.bounds.iter() {
                    match bound {
                        GenericBound::Trait(poly_trait_ref, _) => {
                            for bound_generic_param in poly_trait_ref.bound_generic_params.iter() {
                                parse_generic_param(
                                    &self.tcx,
                                    bound_generic_param,
                                    &mut ty_strings,
                                );
                            }
                            let path = poly_trait_ref.trait_ref.path;
                            parse_path(&self.tcx, &path, &mut ty_strings);
                        }
                        _ => {}
                    }
                }
                self.mod_contexts
                    .last_mut()
                    .unwrap()
                    .extend_application(ty_strings);
            }
            ItemKind::Enum(enum_def, generics) => {
                let def_id = i.owner_id.to_def_id();
                let mut enum_name = self.tcx.def_path(def_id).to_string_no_crate_verbose();
                enum_name = self.tcx.crate_name(def_id.krate).to_string() + &enum_name;
                info!("Visiting enum: {}", enum_name);
                let enum_source = SourceInfo::from_span(i.span, self.tcx.sess.source_map());
                let mut codes = enum_source.get_string();
                codes = attrs_string + &codes;
                let mut ty_strings: BTreeSet<String> = BTreeSet::new();

                for variant in enum_def.variants.iter() {
                    parse_variant(&self.tcx, &variant, &mut ty_strings);
                }
                for param in generics.params.iter() {
                    parse_generic_param(&self.tcx, param, &mut ty_strings);
                }
                for predicate in generics.predicates.iter() {
                    parse_where_predicate(&self.tcx, predicate, &mut ty_strings);
                }

                let enum_item = EnumItem {
                    name: enum_name,
                    codes: codes,
                    derives: BTreeSet::new(),
                    source_info: enum_source,
                    applications: ty_strings,
                };
                self.mod_contexts.last_mut().unwrap().add_enum(enum_item);
            }
            ItemKind::Struct(variant_data, generics) => {
                let def_id = i.owner_id.to_def_id();
                let mut struct_name = self.tcx.def_path(def_id).to_string_no_crate_verbose();
                struct_name = self.tcx.crate_name(def_id.krate).to_string() + &struct_name;
                info!("Visiting struct: {}", struct_name);
                let struct_source = SourceInfo::from_span(i.span, self.tcx.sess.source_map());
                let mut codes = struct_source.get_string();
                codes = attrs_string + &codes;
                let mut ty_strings: BTreeSet<String> = BTreeSet::new();
                parse_variant_data(&self.tcx, &variant_data, &mut ty_strings);
                for param in generics.params.iter() {
                    parse_generic_param(&self.tcx, param, &mut ty_strings);
                }
                for predicate in generics.predicates.iter() {
                    parse_where_predicate(&self.tcx, predicate, &mut ty_strings);
                }

                let struct_item = StructItem {
                    name: struct_name,
                    codes: codes,
                    derives: BTreeSet::new(),
                    source_info: struct_source,
                    applcaitions: ty_strings,
                };
                self.mod_contexts
                    .last_mut()
                    .unwrap()
                    .add_struct(struct_item);
            }
            ItemKind::Union(variant_data, generics) => {
                let def_id = i.owner_id.to_def_id();
                let mut union_name = self.tcx.def_path(def_id).to_string_no_crate_verbose();
                union_name = self.tcx.crate_name(def_id.krate).to_string() + &union_name;
                info!("Visiting union: {}", union_name);
                let union_source = SourceInfo::from_span(i.span, self.tcx.sess.source_map());
                let mut codes = union_source.get_string();
                codes = attrs_string + &codes;
                let mut ty_strings: BTreeSet<String> = BTreeSet::new();
                parse_variant_data(&self.tcx, &variant_data, &mut ty_strings);
                for param in generics.params.iter() {
                    parse_generic_param(&self.tcx, param, &mut ty_strings);
                }
                for predicate in generics.predicates.iter() {
                    parse_where_predicate(&self.tcx, predicate, &mut ty_strings);
                }
                let union_item = UnionItem {
                    name: union_name,
                    codes: codes,
                    derives: BTreeSet::new(),
                    source_info: union_source,
                    applications: ty_strings,
                };
                self.mod_contexts.last_mut().unwrap().add_union(union_item);
            }
            ItemKind::Trait(is_auto, _, generics, generics_bounds, trait_item_refs) => {
                // if let IsAuto::No = is_auto {
                let def_id = i.owner_id.to_def_id();
                let mut trait_name = self.tcx.def_path(def_id).to_string_no_crate_verbose();
                trait_name = self.tcx.crate_name(def_id.krate).to_string() + &trait_name;
                info!("Visiting trait: {}", trait_name);
                let trait_source = SourceInfo::from_span(i.span, self.tcx.sess.source_map());
                let mut codes = trait_source.get_string();
                codes = attrs_string + &codes;
                let mut types: BTreeSet<InnerTypeItem> = BTreeSet::new();
                let mut consts: BTreeSet<InnerConstItem> = BTreeSet::new();
                let mut fns: BTreeSet<InnerFnItem> = BTreeSet::new();

                let mut ty_strings: BTreeSet<String> = BTreeSet::new();
                ty_strings.insert(trait_name.clone());
                for param in generics.params.iter() {
                    parse_generic_param(&self.tcx, param, &mut ty_strings);
                }
                for predicate in generics.predicates.iter() {
                    parse_where_predicate(&self.tcx, predicate, &mut ty_strings);
                }
                for generic_bound in generics_bounds.iter() {
                    parse_generic_bound(&self.tcx, &generic_bound, &mut ty_strings);
                }
                for trait_item_ref in trait_item_refs.iter() {
                    let ref_trait_item = self.hir_map.trait_item(trait_item_ref.id);
                    let generics = ref_trait_item.generics;
                    for param in generics.params.iter() {
                        parse_generic_param(&self.tcx, param, &mut ty_strings);
                    }
                    for predicate in generics.predicates.iter() {
                        parse_where_predicate(&self.tcx, predicate, &mut ty_strings);
                    }
                    match ref_trait_item.kind {
                        TraitItemKind::Const(ty, _) => {
                            recursively_parse_ty(&self.tcx, &ty, &mut ty_strings);
                            let inner_const_source = SourceInfo::from_span(
                                ref_trait_item.span,
                                self.tcx.sess.source_map(),
                            );
                            let inner_const_item = InnerConstItem {
                                codes: inner_const_source.get_string(),
                                source_info: inner_const_source,
                            };
                            consts.insert(inner_const_item);
                        }
                        TraitItemKind::Fn(fn_sig, _) => {
                            self.visit_trait_item_ref(trait_item_ref);
                            fns.insert(self.inner_fn_item.clone().unwrap());
                            self.inner_fn_item = None;
                        }
                        TraitItemKind::Type(generics_bounds, ty) => {
                            for generic_bound in generics_bounds.iter() {
                                parse_generic_bound(&self.tcx, &generic_bound, &mut ty_strings);
                            }
                            if let Some(ty) = ty {
                                recursively_parse_ty(&self.tcx, &ty, &mut ty_strings);
                            }

                            let inner_type_source = SourceInfo::from_span(
                                ref_trait_item.span,
                                self.tcx.sess.source_map(),
                            );
                            let inner_type_item = InnerTypeItem {
                                codes: inner_type_source.get_string(),
                                source_info: inner_type_source,
                            };
                            types.insert(inner_type_item);
                        }
                    }
                }

                let trait_item = TraitItem {
                    name: trait_name,
                    codes: codes,
                    source_info: trait_source,
                    types: types,
                    consts: consts,
                    fns: fns,
                    applications: ty_strings,
                };

                self.mod_contexts.last_mut().unwrap().add_trait(trait_item);
                // }
            }
            ItemKind::TraitAlias(generics, generic_bounds) => {
                let trait_alias_source = SourceInfo::from_span(i.span, self.tcx.sess.source_map());
                let mut codes = trait_alias_source.get_string();
                codes = attrs_string + &codes;
                info!("Visiting traitalias: {}", codes);
                self.mod_contexts
                    .last_mut()
                    .unwrap()
                    .add_trait_alias(trait_alias_source, codes);
                let mut ty_strings: BTreeSet<String> = BTreeSet::new();
                for param in generics.params.iter() {
                    parse_generic_param(&self.tcx, param, &mut ty_strings);
                }
                for predicate in generics.predicates.iter() {
                    parse_where_predicate(&self.tcx, predicate, &mut ty_strings);
                }
                for generic_bound in generic_bounds.iter() {
                    parse_generic_bound(&self.tcx, &generic_bound, &mut ty_strings);
                }
                self.mod_contexts
                    .last_mut()
                    .unwrap()
                    .extend_application(ty_strings);
            }
            ItemKind::Impl(a_impl) => {
                let impl_source = SourceInfo::from_span(i.span, self.tcx.sess.source_map());
                let mut codes = impl_source.get_string();
                codes = attrs_string + &codes;
                info!("Visiting impl: {}", codes);

                // println!("Ty: {:#?}", a_impl.self_ty);
                let def_id = a_impl.self_ty.hir_id.owner.to_def_id();
                let mut impl_name = self.tcx.def_path(def_id).to_string_no_crate_verbose();
                impl_name = self.tcx.crate_name(def_id.krate).to_string() + &impl_name;
                let mut ty_strings: BTreeSet<String> = BTreeSet::new();

                let mut struct_name = String::new();
                let mut ty = a_impl.self_ty;
                while let TyKind::Ref(_, mut_ty) = ty.kind {
                    ty = mut_ty.ty;
                }
                match ty.kind {
                    TyKind::Path(q_path) => match q_path {
                        QPath::Resolved(_, path) => {
                            // println!("{:#?}", path);
                            if let Res::PrimTy(prim_ty) = path.res {
                                let name_source = SourceInfo::from_span(
                                    a_impl.self_ty.span,
                                    self.tcx.sess.source_map(),
                                );
                                struct_name = name_source.get_string();
                                warn!("Struct name prim ty {} {:?}", struct_name, name_source);
                            } else {
                                let def_id = path.res.def_id();
                                struct_name =
                                    self.tcx.def_path(def_id).to_string_no_crate_verbose();
                                struct_name =
                                    self.tcx.crate_name(def_id.krate).to_string() + &struct_name;
                            }
                        }
                        _ => {
                            let name_source = SourceInfo::from_span(
                                a_impl.self_ty.span,
                                self.tcx.sess.source_map(),
                            );
                            struct_name = name_source.get_string();
                            error!(
                                "Wrong struct_name {} {:?} {:#?}",
                                struct_name, name_source, a_impl.self_ty
                            );
                        }
                    },
                    _ => {
                        let name_source =
                            SourceInfo::from_span(a_impl.self_ty.span, self.tcx.sess.source_map());
                        struct_name = name_source.get_string();
                        error!(
                            "Wrong struct_name {} {:?} {:#?}",
                            struct_name, name_source, a_impl.self_ty
                        );
                    }
                }
                ty_strings.insert(struct_name.clone());
                if !is_impl(&codes) {
                    self.mod_contexts
                        .last_mut()
                        .unwrap()
                        .add_derive(struct_name, codes);
                    return;
                }
                let mut trait_name: Option<String> = None;
                if let Some(trait_ref) = a_impl.of_trait {
                    let def_id = trait_ref.path.res.def_id();
                    let mut trait_name_string =
                        self.tcx.def_path(def_id).to_string_no_crate_verbose();
                    trait_name_string =
                        self.tcx.crate_name(def_id.krate).to_string() + &trait_name_string;
                    trait_name = Some(trait_name_string.clone());
                    ty_strings.insert(trait_name_string);
                }

                let mut types: BTreeSet<InnerTypeItem> = BTreeSet::new();
                let mut consts: BTreeSet<InnerConstItem> = BTreeSet::new();
                let mut fns: BTreeSet<InnerFnItem> = BTreeSet::new();

                let generics = a_impl.generics;
                for param in generics.params.iter() {
                    parse_generic_param(&self.tcx, param, &mut ty_strings);
                }
                for predicate in generics.predicates.iter() {
                    parse_where_predicate(&self.tcx, predicate, &mut ty_strings);
                }
                for impl_item_ref in a_impl.items.iter() {
                    let ref_impl_item = self.hir_map.impl_item(impl_item_ref.id);
                    let genrarics = ref_impl_item.generics;
                    for param in generics.params.iter() {
                        parse_generic_param(&self.tcx, param, &mut ty_strings);
                    }
                    for predicate in generics.predicates.iter() {
                        parse_where_predicate(&self.tcx, predicate, &mut ty_strings);
                    }
                    match ref_impl_item.kind {
                        ImplItemKind::Const(ty, _) => {
                            recursively_parse_ty(&self.tcx, &ty, &mut ty_strings);
                            let inner_const_source = SourceInfo::from_span(
                                ref_impl_item.span,
                                self.tcx.sess.source_map(),
                            );
                            let inner_const_item = InnerConstItem {
                                codes: inner_const_source.get_string(),
                                source_info: inner_const_source,
                            };
                            consts.insert(inner_const_item);
                        }
                        ImplItemKind::Fn(fn_sig, _) => {
                            // parse_fn_decl(&self.tcx, fn_sig.decl, &mut ty_strings);
                            self.visit_impl_item_ref(impl_item_ref);
                            fns.insert(self.inner_fn_item.clone().unwrap());
                            self.inner_fn_item = None;
                        }
                        ImplItemKind::Type(ty) => {
                            recursively_parse_ty(&self.tcx, &ty, &mut ty_strings);
                        }
                    }
                }
                let impl_item = ImplItem {
                    name: impl_name,
                    struct_name: struct_name,
                    trait_name: trait_name,
                    codes: codes,
                    source_info: impl_source,
                    types: types,
                    consts: consts,
                    fns: fns,
                    applications: ty_strings,
                };
                self.mod_contexts.last_mut().unwrap().add_impl(impl_item);
            }
            _ => {}
        }
    }
}
