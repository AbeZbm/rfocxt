use log::error;
use rustc_hir::def::Res;
use rustc_hir::intravisit;
use rustc_hir::Body;
use rustc_hir::FnDecl;
use rustc_hir::FnRetTy;
use rustc_hir::GenericArg;
use rustc_hir::GenericBound;
use rustc_hir::GenericParam;
use rustc_hir::GenericParamKind;
use rustc_hir::Pat;
use rustc_hir::PatKind;
use rustc_hir::Path;
use rustc_hir::QPath;
use rustc_hir::Ty;
use rustc_hir::Node;
use rustc_hir::OpaqueTyOrigin;
use rustc_hir::TyKind;
use rustc_hir::Variant;
use rustc_hir::VariantData;
use rustc_hir::WherePredicate;
use rustc_middle::hir::map::Map;
use rustc_middle::ty::TyCtxt;
use std::collections::BTreeSet;

use super::expr_visitor::ExprVisitor;

pub fn recursively_parse_ty<'a, 'tcx>(
    tcx: &'a TyCtxt<'tcx>,
    ty: &Ty<'tcx>,
    ty_strings: &mut BTreeSet<String>,
) {
    let ty = ty.peel_refs();
    match ty.kind {
        TyKind::InferDelegation(def_id, _) => {
            let crate_name = tcx.crate_name(def_id.krate).to_string();
            let mut def_str = tcx.def_path(def_id).to_string_no_crate_verbose();
            def_str = crate_name + &def_str;
            ty_strings.insert(def_str);
        }
        TyKind::Slice(inner_ty) => {
            recursively_parse_ty(tcx, inner_ty, ty_strings);
        }
        TyKind::Array(inner_ty, _) => {
            recursively_parse_ty(tcx, inner_ty, ty_strings);
        }
        TyKind::Ptr(mut_ty) => {
            recursively_parse_ty(tcx, mut_ty.ty, ty_strings);
        }
        TyKind::Ref(_, mut_ty) => {
            recursively_parse_ty(tcx, mut_ty.ty, ty_strings);
        }
        TyKind::BareFn(bare_fn_ty) => {
            for generic_param in bare_fn_ty.generic_params.iter() {
                parse_generic_param(tcx, &generic_param, ty_strings);
            }
            let fn_decl = bare_fn_ty.decl;
            parse_fn_decl(tcx, &fn_decl, ty_strings);
        }
        TyKind::Tup(inner_tys) => {
            for inner_ty in inner_tys.iter() {
                recursively_parse_ty(tcx, inner_ty, ty_strings);
            }
        }
        TyKind::Path(q_path) => {
            parse_q_path(tcx, &q_path, ty_strings);
        }
        // TyKind::OpaqueDef(_, generic_args, _) => {
        //     for generic_arg in generic_args.iter() {
        //         match generic_arg {
        //             GenericArg::Type(inner_ty) => {
        //                 recursively_parse_ty(tcx, inner_ty, ty_strings);
        //             }
        //             _ => {}
        //         }
        //     }
        // }
        TyKind::OpaqueDef(opaque_ty) => {
            for generic_bound in opaque_ty.bounds.iter() {
                parse_generic_bound(tcx, generic_bound, ty_strings);
            }
            let mut local_def_id;
            match opaque_ty.origin{
                OpaqueTyOrigin::FnReturn{
                    parent,..
                }|
                OpaqueTyOrigin::AsyncFn{
                    parent,..
                }|
                OpaqueTyOrigin::TyAlias{
                    parent,..
                }=>{
                    local_def_id=parent;
                }
            }
            let node=tcx.hir_node_by_def_id(local_def_id);
            if let Node::Ty(ty)=node{
                recursively_parse_ty(tcx,ty,ty_strings);
            }
        }
        TyKind::TraitObject(poly_trait_refs, ..) => {
            for poly_trait_ref in poly_trait_refs.iter() {
                for bound_generic_param in poly_trait_ref.bound_generic_params.iter() {
                    parse_generic_param(tcx, bound_generic_param, ty_strings);
                }
                let path = poly_trait_ref.trait_ref.path;
                parse_path(tcx, &path, ty_strings);
            }
        }
        TyKind::Pat(ty, pat) => {
            recursively_parse_ty(tcx, &ty, ty_strings);
            recursively_parse_pat(tcx, &tcx.hir(), &pat, ty_strings);
        }
        _ => {}
    }
}

pub fn recursively_parse_pat<'a, 'tcx>(
    tcx: &'a TyCtxt<'tcx>,
    hir_map: &Map<'tcx>,
    pat: &Pat<'tcx>,
    ty_strings: &mut BTreeSet<String>,
) {
    match pat.kind {
        PatKind::Binding(.., inner_pat) => {
            if let Some(inner_pat) = inner_pat {
                recursively_parse_pat(tcx, hir_map, inner_pat, ty_strings);
            }
        }
        PatKind::Struct(q_path, pat_feilds, _) => {
            parse_q_path(tcx, &q_path, ty_strings);
            for path_feild in pat_feilds.iter() {
                recursively_parse_pat(tcx, hir_map, path_feild.pat, ty_strings);
            }
        }
        PatKind::TupleStruct(q_path, pats, _) => {
            parse_q_path(tcx, &q_path, ty_strings);
            for inner_pat in pats.iter() {
                recursively_parse_pat(tcx, hir_map, inner_pat, ty_strings);
            }
        }
        PatKind::Or(pats) => {
            for inner_pat in pats.iter() {
                recursively_parse_pat(tcx, hir_map, inner_pat, ty_strings);
            }
        }
        PatKind::Path(q_path) => {
            parse_q_path(tcx, &q_path, ty_strings);
        }
        PatKind::Tuple(pats, _) => {
            for inner_pat in pats.iter() {
                recursively_parse_pat(tcx, hir_map, inner_pat, ty_strings);
            }
        }
        PatKind::Box(inner_pat) | PatKind::Deref(inner_pat) | PatKind::Ref(inner_pat, _) => {
            recursively_parse_pat(tcx, hir_map, inner_pat, ty_strings);
        }
        PatKind::Lit(expr) => {
            let mut expr_visitor = ExprVisitor::new(tcx.clone(), hir_map.clone());
            intravisit::walk_expr::<ExprVisitor>(&mut expr_visitor, expr);
            ty_strings.extend(expr_visitor.get_applications());
        }
        PatKind::Range(expr1, expr2, _) => {
            if let Some(expr) = expr1 {
                let mut expr_visitor = ExprVisitor::new(tcx.clone(), hir_map.clone());
                intravisit::walk_expr::<ExprVisitor>(&mut expr_visitor, expr);
                ty_strings.extend(expr_visitor.get_applications());
            }
            if let Some(expr) = expr2 {
                let mut expr_visitor = ExprVisitor::new(tcx.clone(), hir_map.clone());
                intravisit::walk_expr::<ExprVisitor>(&mut expr_visitor, expr);
                ty_strings.extend(expr_visitor.get_applications());
            }
        }
        PatKind::Slice(pats1, some_pat, pats2) => {
            for pat1 in pats1.iter() {
                recursively_parse_pat(tcx, hir_map, pat1, ty_strings);
            }
            if let Some(some_pat) = some_pat {
                recursively_parse_pat(tcx, hir_map, some_pat, ty_strings);
            }
            for pat2 in pats2.iter() {
                recursively_parse_pat(tcx, hir_map, pat2, ty_strings);
            }
        }
        _ => {}
    }
}

pub fn parse_body<'a, 'tcx>(
    tcx: &'a TyCtxt<'tcx>,
    hir_map: &Map<'tcx>,
    body: &'tcx Body,
    ty_strings: &mut BTreeSet<String>,
) {
    for param in body.params.iter() {
        recursively_parse_pat(tcx, hir_map, param.pat, ty_strings);
    }

    // error!("{:#?}", tcx.typeck(body.id().hir_id.owner));
    let mut expr_visitor = ExprVisitor::new(
        tcx.clone(),
        hir_map.clone(),
        // tcx.typeck(body.id().hir_id.owner),
    );
    intravisit::walk_expr::<ExprVisitor>(&mut expr_visitor, body.value);
    ty_strings.extend(expr_visitor.get_applications());
    let type_dependent_defs = tcx.typeck(body.id().hir_id.owner).type_dependent_defs();
    let items = type_dependent_defs.items();
    let new_names: Vec<String> = items
        .filter_map(|(item_local_id, result)| {
            // 使用 filter_map 过滤并生成名称
            if let Ok((_def_kind, def_id)) = result {
                let fn_path = tcx.def_path(*def_id).to_string_no_crate_verbose();
                let crate_name = tcx.crate_name(def_id.krate).to_string();
                Some(crate_name + &fn_path) // 返回生成的完整名称
            } else {
                None // 忽略错误项
            }
        })
        .into_sorted(&tcx);
    ty_strings.extend(new_names);
    // .map(|(item_local_id, result)| {
    //     if let Ok((def_kind, def_id)) = result {
    //         let mut fn_name = tcx.def_path(*def_id).to_string_no_crate_verbose();
    //         fn_name = tcx.crate_name(def_id.krate).to_string() + &fn_name;
    //         ty_strings.insert(fn_name);
    //     }
    // });
}

pub fn parse_generic_param<'a, 'tcx>(
    tcx: &'a TyCtxt<'tcx>,
    generic_param: &GenericParam<'tcx>,
    ty_strings: &mut BTreeSet<String>,
) {
    match generic_param.kind {
        GenericParamKind::Type { default, .. } => {
            if let Some(default) = default {
                recursively_parse_ty(tcx, default, ty_strings);
            }
        }
        GenericParamKind::Const { ty, default, .. } => {
            recursively_parse_ty(tcx, &ty, ty_strings);
        }
        _ => {}
    }
}

pub fn parse_where_predicate<'a, 'tcx>(
    tcx: &'a TyCtxt<'tcx>,
    where_predicate: &WherePredicate<'tcx>,
    ty_strings: &mut BTreeSet<String>,
) {
    match where_predicate {
        WherePredicate::BoundPredicate(where_bound_predicate) => {
            recursively_parse_ty(tcx, where_bound_predicate.bounded_ty, ty_strings);
        }
        WherePredicate::RegionPredicate(where_region_predicate) => {
            for generic_bound in where_region_predicate.bounds.iter() {
                parse_generic_bound(tcx, &generic_bound, ty_strings);
            }
        }
        WherePredicate::EqPredicate(where_eq_predicate) => {
            recursively_parse_ty(tcx, where_eq_predicate.lhs_ty, ty_strings);
            recursively_parse_ty(tcx, where_eq_predicate.rhs_ty, ty_strings);
        }
    }
}

pub fn parse_path<'a, 'tcx>(
    tcx: &'a TyCtxt<'tcx>,
    path: &Path<'tcx>,
    ty_strings: &mut BTreeSet<String>,
) {
    let res = path.res;
    match res {
        Res::Def(_, def_id) => {
            let crate_name = tcx.crate_name(def_id.krate).to_string();
            let mut def_str = tcx.def_path(def_id).to_string_no_crate_verbose();
            def_str = crate_name + &def_str;
            ty_strings.insert(def_str);
        }
        Res::SelfTyParam { trait_: def_id } => {
            let crate_name = tcx.crate_name(def_id.krate).to_string();
            let mut def_str = tcx.def_path(def_id).to_string_no_crate_verbose();
            def_str = crate_name + &def_str;
            ty_strings.insert(def_str);
        }
        Res::SelfTyAlias {
            alias_to: def_id, ..
        } => {
            let crate_name = tcx.crate_name(def_id.krate).to_string();
            let mut def_str = tcx.def_path(def_id).to_string_no_crate_verbose();
            def_str = crate_name + &def_str;
            ty_strings.insert(def_str);
        }
        Res::SelfCtor(def_id) => {
            let crate_name = tcx.crate_name(def_id.krate).to_string();
            let mut def_str = tcx.def_path(def_id).to_string_no_crate_verbose();
            def_str = crate_name + &def_str;
            ty_strings.insert(def_str);
        }
        _ => {}
    }
}

pub fn parse_q_path<'a, 'tcx>(
    tcx: &'a TyCtxt<'tcx>,
    q_path: &QPath<'tcx>,
    ty_strings: &mut BTreeSet<String>,
) {
    match q_path {
        QPath::Resolved(inner_ty, path) => {
            if let Some(inner_ty) = inner_ty {
                recursively_parse_ty(tcx, inner_ty, ty_strings);
            }
            let res = path.res;
            match res {
                Res::Def(_, def_id) => {
                    let crate_name = tcx.crate_name(def_id.krate).to_string();
                    let mut def_str = tcx.def_path(def_id).to_string_no_crate_verbose();
                    def_str = crate_name + &def_str;
                    ty_strings.insert(def_str);
                }
                Res::SelfTyParam { trait_: def_id } => {
                    let crate_name = tcx.crate_name(def_id.krate).to_string();
                    let mut def_str = tcx.def_path(def_id).to_string_no_crate_verbose();
                    def_str = crate_name + &def_str;
                    ty_strings.insert(def_str);
                }
                Res::SelfTyAlias {
                    alias_to: def_id, ..
                } => {
                    let crate_name = tcx.crate_name(def_id.krate).to_string();
                    let mut def_str = tcx.def_path(def_id).to_string_no_crate_verbose();
                    def_str = crate_name + &def_str;
                    ty_strings.insert(def_str);
                }
                Res::SelfCtor(def_id) => {
                    let crate_name = tcx.crate_name(def_id.krate).to_string();
                    let mut def_str = tcx.def_path(def_id).to_string_no_crate_verbose();
                    def_str = crate_name + &def_str;
                    ty_strings.insert(def_str);
                }
                _ => {}
            }
        }
        QPath::TypeRelative(inner_ty, _) => {
            recursively_parse_ty(tcx, inner_ty, ty_strings);
        }
        _ => {}
    }
}

pub fn parse_fn_decl<'a, 'tcx>(
    tcx: &'a TyCtxt<'tcx>,
    fn_decl: &FnDecl<'tcx>,
    ty_strings: &mut BTreeSet<String>,
) {
    for input in fn_decl.inputs.iter() {
        recursively_parse_ty(tcx, input, ty_strings);
    }
    if let FnRetTy::Return(ty) = fn_decl.output {
        recursively_parse_ty(tcx, &ty, ty_strings);
    }
}

pub fn parse_variant<'a, 'tcx>(
    tcx: &'a TyCtxt<'tcx>,
    variant: &Variant<'tcx>,
    ty_strings: &mut BTreeSet<String>,
) {
    match variant.data {
        VariantData::Struct { fields, .. } => {
            for field in fields.iter() {
                recursively_parse_ty(tcx, field.ty, ty_strings);
            }
        }
        VariantData::Tuple(fields, ..) => {
            for field in fields.iter() {
                recursively_parse_ty(tcx, field.ty, ty_strings);
            }
        }
        _ => {}
    }
}

pub fn parse_variant_data<'a, 'tcx>(
    tcx: &'a TyCtxt<'tcx>,
    variant_data: &VariantData<'tcx>,
    ty_strings: &mut BTreeSet<String>,
) {
    match variant_data {
        VariantData::Struct { fields, .. } => {
            for field in fields.iter() {
                recursively_parse_ty(tcx, field.ty, ty_strings);
            }
        }
        VariantData::Tuple(fields, ..) => {
            for field in fields.iter() {
                recursively_parse_ty(tcx, field.ty, ty_strings);
            }
        }
        _ => {}
    }
}

pub fn parse_generic_bound<'a, 'tcx>(
    tcx: &'a TyCtxt<'tcx>,
    generic_bound: &GenericBound<'tcx>,
    ty_strings: &mut BTreeSet<String>,
) {
    match generic_bound {
        GenericBound::Trait(poly_trait_ref) => {
            for bound_generic_param in poly_trait_ref.bound_generic_params.iter() {
                parse_generic_param(tcx, bound_generic_param, ty_strings);
            }
            let path = poly_trait_ref.trait_ref.path;
            parse_path(tcx, &path, ty_strings);
        }
        _ => {}
    }
}
