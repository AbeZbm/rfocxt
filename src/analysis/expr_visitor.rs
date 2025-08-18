use super::parses::parse_body;
use super::parses::parse_fn_decl;
use super::parses::parse_generic_param;
use super::parses::parse_q_path;
use super::parses::recursively_parse_pat;
use super::parses::recursively_parse_ty;
use rustc_hir::intravisit;
use rustc_hir::intravisit::Visitor;
use rustc_hir::Expr;
use rustc_hir::ExprKind;
use rustc_hir::Stmt;
use rustc_hir::StmtKind;
use rustc_middle::hir::map::Map;
use rustc_middle::hir::nested_filter;
use rustc_middle::ty::TyCtxt;
use rustc_middle::ty::TypeckResults;
use std::collections::BTreeSet;

pub struct ExprVisitor<'tcx> {
    tcx: TyCtxt<'tcx>,
    hir_map: Map<'tcx>,
    applications: BTreeSet<String>,
    // type_check_res: &'tcx TypeckResults<'tcx>,
}

impl<'tcx> ExprVisitor<'tcx> {
    pub fn new(
        tcx: TyCtxt<'tcx>,
        hir_map: Map<'tcx>,
        // type_check_res: &'tcx TypeckResults<'tcx>,
    ) -> Self {
        ExprVisitor {
            tcx: tcx,
            hir_map: hir_map,
            applications: BTreeSet::new(),
            // type_check_res: type_check_res,
        }
    }

    pub fn get_applications(&self) -> BTreeSet<String> {
        self.applications.clone()
    }
}

impl<'tcx> Visitor<'tcx> for ExprVisitor<'tcx> {
    type NestedFilter = nested_filter::All;

    fn nested_visit_map(&mut self) -> Self::Map {
        self.tcx.hir().clone()
    }

    fn visit_stmt(&mut self, s: &'tcx Stmt<'tcx>) -> Self::Result {
        match s.kind {
            StmtKind::Let(let_stmt) => {
                recursively_parse_pat(
                    &self.tcx,
                    &self.hir_map,
                    let_stmt.pat,
                    &mut self.applications,
                );
                if let Some(ty) = let_stmt.ty {
                    recursively_parse_ty(&self.tcx, ty, &mut self.applications);
                }
            }
            _ => {}
        }
        intravisit::walk_stmt(self, s);
    }

    fn visit_expr(&mut self, ex: &'tcx Expr<'tcx>) -> Self::Result {
        // error!("1");
        match ex.kind {
            ExprKind::ConstBlock(const_block) => {
                let body = self.hir_map.body(const_block.body);
                parse_body(&self.tcx, &self.hir_map, &body, &mut self.applications);
            }
            ExprKind::Cast(_, ty) => {
                recursively_parse_ty(&self.tcx, &ty, &mut self.applications);
            }
            ExprKind::Type(_, ty) => {
                recursively_parse_ty(&self.tcx, &ty, &mut self.applications);
            }
            ExprKind::Let(let_expr) => {
                recursively_parse_pat(
                    &self.tcx,
                    &self.hir_map,
                    let_expr.pat,
                    &mut self.applications,
                );
                if let Some(ty) = let_expr.ty {
                    recursively_parse_ty(&self.tcx, &ty, &mut self.applications);
                }
            }
            ExprKind::Match(_, arms, _) => {
                for arm in arms.iter() {
                    recursively_parse_pat(
                        &self.tcx,
                        &self.hir_map,
                        arm.pat,
                        &mut self.applications,
                    );
                }
            }
            ExprKind::Closure(closure) => {
                for generic_param in closure.bound_generic_params.iter() {
                    parse_generic_param(&self.tcx, &generic_param, &mut self.applications);
                }
                let fn_decl = closure.fn_decl;
                parse_fn_decl(&self.tcx, &fn_decl, &mut self.applications);
                let inner_body = self.hir_map.body(closure.body);
                parse_body(&self.tcx, &self.hir_map, inner_body, &mut self.applications);
            }
            ExprKind::Path(q_path) => {
                parse_q_path(&self.tcx, &q_path, &mut self.applications);
            }
            ExprKind::OffsetOf(ty, _) => {
                recursively_parse_ty(&self.tcx, &ty, &mut self.applications);
            }
            ExprKind::Struct(q_path, ..) => {
                parse_q_path(&self.tcx, &q_path, &mut self.applications);
            }
            _ => {}
        }
        intravisit::walk_expr(self, ex);
    }
}
