use log::info;
use rustc_hir::intravisit;
use rustc_hir::intravisit::Visitor;
use rustc_middle::hir::map::Map;
use rustc_middle::hir::nested_filter;
use rustc_middle::ty::TyCtxt;

pub struct HirVisitor<'tcx> {
    tcx: TyCtxt<'tcx>,
    hir_map: Map<'tcx>,
}

impl<'tcx> HirVisitor<'tcx> {
    pub fn new(tcx: TyCtxt<'tcx>, hir_map: Map<'tcx>) -> Self {
        HirVisitor {
            tcx: tcx,
            hir_map: hir_map,
        }
    }
}

impl<'tcx> Visitor<'tcx> for HirVisitor<'tcx> {
    type NestedFilter = nested_filter::All;

    fn nested_visit_map(&mut self) -> Self::Map {
        self.hir_map
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
        intravisit::walk_mod(self, m, n);
        info!("Leaving module: {}", module_name);
    }
}
