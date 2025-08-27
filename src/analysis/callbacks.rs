use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use log::info;
use rustc_driver::Compilation;
use rustc_interface::{Queries, interface};
use rustc_middle::ty::TyCtxt;

use super::hir_visitor::HirVisitor;
use super::parse_context::ParseContext;
use crate::OUT_FILE_PATH;

pub struct RfocxtCallbacks {
    pub source_name: String,
    pub crate_path: PathBuf,
}

impl RfocxtCallbacks {
    pub fn new(crate_path: PathBuf) -> Self {
        Self { source_name: String::new(), crate_path: crate_path }
    }

    fn run_analysis<'tcx, 'compiler>(&mut self, tcx: TyCtxt<'tcx>) {
        let out_dir = self.crate_path.join(OUT_FILE_PATH);
        if out_dir.exists() {
            fs::remove_dir_all(&out_dir).unwrap();
        }

        let hir_map = tcx.hir();
        let mut visitor = HirVisitor::new(tcx, hir_map, self.crate_path.clone());
        hir_map.walk_toplevel_module(&mut visitor);
        let mod_contexts = visitor.get_complete_mod_contexts();
        // println!("hir_visitor\n {:#?}", mod_contexts);

        let output_path = self.crate_path.join(format!("{}/context.txt", OUT_FILE_PATH));
        fs::create_dir_all(output_path.parent().unwrap()).unwrap();
        let mut file = File::create(&output_path).unwrap();
        file.write_all(format!("{:#?}", mod_contexts).as_bytes()).unwrap();

        let parse_context = ParseContext::new(self.crate_path.clone(), &mod_contexts);
        parse_context.parse_context();
    }
}

impl rustc_driver::Callbacks for RfocxtCallbacks {
    fn config(&mut self, config: &mut interface::Config) {
        self.source_name = format!("{:?}", config.input.source_name());
        config.crate_cfg.push("rfocxt".to_string());
        info!("Source file: {}", self.source_name);
    }

    fn after_expansion<'tcx>(
        &mut self,
        _compiler: &interface::Compiler,
        _queries: &'tcx Queries<'tcx>,
    ) -> Compilation {
        _queries.global_ctxt().unwrap().enter(|tcx| self.run_analysis(tcx));
        Compilation::Continue
    }

    // fn after_analysis<'tcx>(
    //     &mut self,
    //     _compiler: &interface::Compiler,
    //     _queries: &'tcx Queries<'tcx>,
    // ) -> Compilation {
    //     _queries
    //         .global_ctxt()
    //         .unwrap()
    //         .enter(|tcx| self.run_analysis(tcx));
    //     Compilation::Continue
    // }
}
