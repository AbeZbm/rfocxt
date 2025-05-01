use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

use super::{mod_context::ModContext, syn_file::SynFile};

fn clear_codes(codes: &mut String) {
    let left = codes.find('{').unwrap();
    let mut right = -1;
    for s in codes.chars().into_iter().rev().enumerate() {
        if s.1 == '}' {
            right = s.0 as i32;
            break;
        }
    }
    *codes = codes[..left].to_string() + &codes[right as usize..].to_string();
}

fn parse_direct_applications(
    mod_contexts: &Vec<ModContext>,
    direct_applications: &mut BTreeSet<String>,
    indirect_applications: &mut BTreeSet<String>,
) {
    let mut left_applications: BTreeSet<String> = direct_applications.clone();
    while !left_applications.is_empty() {
        let application = left_applications.pop_first().unwrap();
        if !indirect_applications.contains(&application) {
            indirect_applications.insert(application.clone());
            for mod_context in mod_contexts.iter() {
                for fn_item in mod_context.fns.iter() {
                    if fn_item.name == application {
                        left_applications.extend(fn_item.applications.clone());
                    }
                }
                for enum_item in mod_context.enums.iter() {
                    if enum_item.name == application {
                        left_applications.extend(enum_item.applications.clone());
                    }
                }
                for struct_item in mod_context.structs.iter() {
                    if struct_item.name == application {
                        left_applications.extend(struct_item.applcaitions.clone());
                    }
                }
                for union_item in mod_context.unions.iter() {
                    if union_item.name == application {
                        left_applications.extend(union_item.applications.clone());
                    }
                }
                for trait_item in mod_context.traits.iter() {
                    if trait_item.name == application {
                        left_applications.extend(trait_item.applications.clone());
                    }
                }
                for impl_item in mod_context.impls.iter() {
                    if impl_item.struct_name == application {
                        left_applications.extend(impl_item.applications.clone());
                    }
                    if let Some(trait_name) = impl_item.trait_name.clone() {
                        if trait_name == application {
                            left_applications.extend(impl_item.applications.clone());
                        }
                    }
                    for fn_item in impl_item.fns.iter() {
                        if fn_item.name == application {
                            left_applications.extend(fn_item.applications.clone());
                        }
                    }
                }
            }
        }
    }
}

struct ContextString {
    files: BTreeMap<String, BTreeSet<String>>,
}

pub struct ParseContext<'a> {
    crate_path: PathBuf,
    mod_contexts: &'a Vec<ModContext>,
}

impl<'a> ParseContext<'a> {
    pub fn new(crate_path: PathBuf, mod_contexts: &'a Vec<ModContext>) -> Self {
        ParseContext {
            crate_path: crate_path,
            mod_contexts: mod_contexts,
        }
    }

    pub fn parse_context(&self) {
        for mod_context in self.mod_contexts.iter() {
            let basic_syn_file = SynFile::new(mod_context);
            for fn_item in mod_context.fns.iter() {
                let mut direct_applications: BTreeSet<String> = BTreeSet::new();
                direct_applications.insert(fn_item.name.clone());
                direct_applications.extend(fn_item.applications.clone());
                direct_applications.extend(mod_context.applications.clone());
                let mut indirect_applications: BTreeSet<String> = BTreeSet::new();
                parse_direct_applications(
                    self.mod_contexts,
                    &mut direct_applications,
                    &mut indirect_applications,
                );
                println!("{:#?}", indirect_applications);
            }
        }
    }
}
