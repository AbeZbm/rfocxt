use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File},
    io::Write,
    path::PathBuf,
};

use log::error;
use syn::{
    parse_str, ImplItemFn, ItemEnum, ItemFn, ItemStruct, ItemTrait, ItemUnion, TraitItem,
    TraitItemConst, TraitItemFn, TraitItemType,
};

use super::{
    mod_context::ModContext,
    syn_file::{
        EnumSynItem, FnSynItem, ImplFnSynItem, ImplSynItem, StructSynItem, SynApplication, SynFile,
        TraitConstSynItem, TraitFnSynItem, TraitSynItem, TraitTypeSynItem, UnionSynItem,
    },
};

use crate::{utils::encoded_name, OUT_FILE_PATH};

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

    fn get_direct_item(&self, application: &String) -> Option<(String, SynApplication)> {
        for mod_context in self.mod_contexts.iter() {
            for fn_item in mod_context.fns.iter() {
                if fn_item.name.eq(application) {
                    let item_fn = parse_str::<ItemFn>(&fn_item.codes);
                    if let Ok(item_fn) = item_fn {
                        return Some((
                            mod_context.name.clone(),
                            SynApplication::Fn(FnSynItem {
                                name: fn_item.name.clone(),
                                item: item_fn,
                            }),
                        ));
                    } else {
                        return None;
                    }
                }
            }
            for enum_item in mod_context.enums.iter() {
                if enum_item.name.eq(application) {
                    let item_enum = parse_str::<ItemEnum>(&enum_item.codes);
                    if let Ok(item_enum) = item_enum {
                        return Some((
                            mod_context.name.clone(),
                            SynApplication::Enum(EnumSynItem {
                                name: enum_item.name.clone(),
                                item: item_enum,
                            }),
                        ));
                    } else {
                        return None;
                    }
                }
            }
            for struct_item in mod_context.structs.iter() {
                if struct_item.name.eq(application) {
                    let item_struct = parse_str::<ItemStruct>(&struct_item.codes);
                    if let Ok(item_struct) = item_struct {
                        return Some((
                            mod_context.name.clone(),
                            SynApplication::Struct(StructSynItem {
                                name: struct_item.name.clone(),
                                item: item_struct,
                            }),
                        ));
                    } else {
                        return None;
                    }
                }
            }
            for union_item in mod_context.unions.iter() {
                if union_item.name.eq(application) {
                    let item_union = parse_str::<ItemUnion>(&union_item.codes);
                    if let Ok(item_union) = item_union {
                        return Some((
                            mod_context.name.clone(),
                            SynApplication::Union(UnionSynItem {
                                name: union_item.name.clone(),
                                item: item_union,
                            }),
                        ));
                    } else {
                        return None;
                    }
                }
            }
            for trait_item in mod_context.traits.iter() {
                if trait_item.name.eq(application) {
                    let trait_syn_item = TraitSynItem::from_trait_item(trait_item);
                    if let Some(trait_syn_item) = trait_syn_item {
                        return Some((
                            mod_context.name.clone(),
                            SynApplication::Trait(trait_syn_item),
                        ));
                    } else {
                        return None;
                    }
                }
                for trait_fn in trait_item.fns.iter() {
                    if trait_fn.name.eq(application) {
                        let trait_syn_item = TraitSynItem::from_trait_item(trait_item);
                        if let Some(mut trait_syn_item) = trait_syn_item {
                            let item_trait_fn: Result<TraitItemFn, syn::Error> =
                                parse_str::<TraitItemFn>(&trait_fn.codes);
                            if let Ok(item_trait_fn) = item_trait_fn {
                                let trait_fn_syn_item = TraitFnSynItem {
                                    name: trait_fn.name.clone(),
                                    item: item_trait_fn,
                                };
                                for trait_fn_item in trait_syn_item.fns.iter_mut() {
                                    if trait_fn_item.name.eq(application) {
                                        *trait_fn_item = trait_fn_syn_item;
                                        break;
                                    }
                                }
                            }
                            return Some((
                                mod_context.name.clone(),
                                SynApplication::Trait(trait_syn_item),
                            ));
                        } else {
                            return None;
                        }
                    }
                }
            }
            for impl_item in mod_context.impls.iter() {
                if impl_item.struct_name.eq(application) {
                    let impl_syn_item = ImplSynItem::from_impl_item(impl_item);
                    if let Some(impl_syn_item) = impl_syn_item {
                        return Some((
                            mod_context.name.clone(),
                            SynApplication::Impl(impl_syn_item),
                        ));
                    } else {
                        return None;
                    }
                }
                if let Some(trait_name) = impl_item.trait_name.clone() {
                    if trait_name.eq(application) {
                        let impl_syn_item = ImplSynItem::from_impl_item(impl_item);
                        if let Some(impl_syn_item) = impl_syn_item {
                            return Some((
                                mod_context.name.clone(),
                                SynApplication::Impl(impl_syn_item),
                            ));
                        } else {
                            return None;
                        }
                    }
                }
                for impl_fn in impl_item.fns.iter() {
                    if impl_fn.name.eq(application) {
                        let impl_syn_item = ImplSynItem::from_impl_item(impl_item);
                        if let Some(mut impl_syn_item) = impl_syn_item {
                            let item_impl_fn = parse_str::<ImplItemFn>(&impl_fn.codes);
                            if let Ok(item_impl_fn) = item_impl_fn {
                                let impl_fn_syn_item = ImplFnSynItem {
                                    name: impl_fn.name.clone(),
                                    item: item_impl_fn,
                                };
                                for impl_fn_item in impl_syn_item.fns.iter_mut() {
                                    if impl_fn_item.name.eq(application) {
                                        *impl_fn_item = impl_fn_syn_item;
                                        break;
                                    }
                                }
                            }
                            return Some((
                                mod_context.name.clone(),
                                SynApplication::Impl(impl_syn_item),
                            ));
                        } else {
                            return None;
                        }
                    }
                }
            }
        }
        None
    }

    fn get_indirect_item(&self, application: &String) -> Option<(String, SynApplication)> {
        for mod_context in self.mod_contexts.iter() {
            for fn_item in mod_context.fns.iter() {
                if fn_item.name.eq(application) {
                    let item_fn = parse_str::<ItemFn>(&fn_item.codes);
                    if let Ok(mut item_fn) = item_fn {
                        item_fn.block.stmts.clear();
                        return Some((
                            mod_context.name.clone(),
                            SynApplication::Fn(FnSynItem {
                                name: fn_item.name.clone(),
                                item: item_fn,
                            }),
                        ));
                    } else {
                        return None;
                    }
                }
            }
            for enum_item in mod_context.enums.iter() {
                if enum_item.name.eq(application) {
                    let item_enum = parse_str::<ItemEnum>(&enum_item.codes);
                    if let Ok(item_enum) = item_enum {
                        return Some((
                            mod_context.name.clone(),
                            SynApplication::Enum(EnumSynItem {
                                name: enum_item.name.clone(),
                                item: item_enum,
                            }),
                        ));
                    } else {
                        return None;
                    }
                }
            }
            for struct_item in mod_context.structs.iter() {
                if struct_item.name.eq(application) {
                    let item_struct = parse_str::<ItemStruct>(&struct_item.codes);
                    if let Ok(item_struct) = item_struct {
                        return Some((
                            mod_context.name.clone(),
                            SynApplication::Struct(StructSynItem {
                                name: struct_item.name.clone(),
                                item: item_struct,
                            }),
                        ));
                    } else {
                        return None;
                    }
                }
            }
            for union_item in mod_context.unions.iter() {
                if union_item.name.eq(application) {
                    let item_union = parse_str::<ItemUnion>(&union_item.codes);
                    if let Ok(item_union) = item_union {
                        return Some((
                            mod_context.name.clone(),
                            SynApplication::Union(UnionSynItem {
                                name: union_item.name.clone(),
                                item: item_union,
                            }),
                        ));
                    } else {
                        return None;
                    }
                }
            }
            for trait_item in mod_context.traits.iter() {
                if trait_item.name.eq(application) {
                    let trait_syn_item = TraitSynItem::from_trait_item(trait_item);
                    if let Some(trait_syn_item) = trait_syn_item {
                        return Some((
                            mod_context.name.clone(),
                            SynApplication::Trait(trait_syn_item),
                        ));
                    } else {
                        return None;
                    }
                }
                for trait_fn in trait_item.fns.iter() {
                    if trait_fn.name.eq(application) {
                        let trait_syn_item = TraitSynItem::from_trait_item(trait_item);
                        if let Some(trait_syn_item) = trait_syn_item {
                            return Some((
                                mod_context.name.clone(),
                                SynApplication::Trait(trait_syn_item),
                            ));
                        } else {
                            return None;
                        }
                    }
                }
            }
            for impl_item in mod_context.impls.iter() {
                if impl_item.struct_name.eq(application) {
                    let impl_syn_item = ImplSynItem::from_impl_item(impl_item);
                    if let Some(impl_syn_item) = impl_syn_item {
                        return Some((
                            mod_context.name.clone(),
                            SynApplication::Impl(impl_syn_item),
                        ));
                    } else {
                        return None;
                    }
                }
                if let Some(trait_name) = impl_item.trait_name.clone() {
                    if trait_name.eq(application) {
                        let impl_syn_item = ImplSynItem::from_impl_item(impl_item);
                        if let Some(impl_syn_item) = impl_syn_item {
                            return Some((
                                mod_context.name.clone(),
                                SynApplication::Impl(impl_syn_item),
                            ));
                        } else {
                            return None;
                        }
                    }
                }
                for impl_fn in impl_item.fns.iter() {
                    if impl_fn.name.eq(application) {
                        let impl_syn_item = ImplSynItem::from_impl_item(impl_item);
                        if let Some(impl_syn_item) = impl_syn_item {
                            return Some((
                                mod_context.name.clone(),
                                SynApplication::Impl(impl_syn_item),
                            ));
                        } else {
                            return None;
                        }
                    }
                }
            }
        }
        None
    }

    fn parse_direct_applications(
        &self,
        direct_applications: &BTreeSet<String>,
        syn_files: &mut Vec<SynFile>,
    ) {
        for direct_application in direct_applications.iter() {
            let syn_application = self.get_direct_item(direct_application);
            // if direct_application == "a::b::{impl#0}::new" {
            //     error!("{:#?}", syn_application);
            // }

            if let Some((mod_name, syn_application)) = syn_application {
                let mut has_syn_file = false;
                for syn_file in syn_files.iter_mut() {
                    if syn_file.name == mod_name {
                        match &syn_application {
                            SynApplication::Fn(fn_syn_item) => {
                                syn_file.fns.push(fn_syn_item.clone());
                            }
                            SynApplication::Enum(enum_syn_item) => {
                                syn_file.enums.push(enum_syn_item.clone());
                            }
                            SynApplication::Struct(struct_syn_item) => {
                                syn_file.structs.push(struct_syn_item.clone());
                            }
                            SynApplication::Union(union_syn_item) => {
                                syn_file.unions.push(union_syn_item.clone());
                            }
                            SynApplication::Trait(trait_syn_item) => {
                                let mut has_trait_item = false;
                                for trait_item in syn_file.traits.iter_mut() {
                                    if trait_item.name == trait_syn_item.name {
                                        trait_item.add_direct_application_trait(&trait_syn_item);
                                        has_trait_item = true;
                                        break;
                                    }
                                }
                                if !has_trait_item {
                                    syn_file.traits.push(trait_syn_item.clone());
                                }
                            }
                            SynApplication::Impl(impl_syn_item) => {
                                let mut has_impl_item = false;
                                for impl_item in syn_file.impls.iter_mut() {
                                    if impl_item.name == impl_syn_item.name {
                                        impl_item.add_direct_application_impl(&impl_syn_item);
                                        has_impl_item = true;
                                        break;
                                    }
                                }
                                if !has_impl_item {
                                    syn_file.impls.push(impl_syn_item.clone());
                                }
                            }
                        }
                        has_syn_file = true;
                        break;
                    }
                }
                if !has_syn_file {
                    let mut syn_file: SynFile;
                    for mod_context in self.mod_contexts.iter() {
                        if mod_context.name == mod_name {
                            syn_file = SynFile::new(mod_context);
                            match &syn_application {
                                SynApplication::Fn(fn_syn_item) => {
                                    syn_file.fns.push(fn_syn_item.clone());
                                }
                                SynApplication::Enum(enum_syn_item) => {
                                    syn_file.enums.push(enum_syn_item.clone());
                                }
                                SynApplication::Struct(struct_syn_item) => {
                                    syn_file.structs.push(struct_syn_item.clone());
                                }
                                SynApplication::Union(union_syn_item) => {
                                    syn_file.unions.push(union_syn_item.clone());
                                }
                                SynApplication::Trait(trait_syn_item) => {
                                    syn_file.traits.push(trait_syn_item.clone());
                                }
                                SynApplication::Impl(impl_syn_item) => {
                                    syn_file.impls.push(impl_syn_item.clone());
                                }
                            }
                            syn_files.push(syn_file);
                            break;
                        }
                    }
                }
            }
        }
    }

    fn parse_indirect_applications(
        &self,
        indirect_applications: &BTreeSet<String>,
        syn_files: &mut Vec<SynFile>,
    ) {
        for indirect_application in indirect_applications.iter() {
            let syn_application = self.get_indirect_item(indirect_application);
            if let Some((mod_name, syn_application)) = syn_application {
                let mut has_syn_file = false;
                for syn_file in syn_files.iter_mut() {
                    if syn_file.name == mod_name {
                        match &syn_application {
                            SynApplication::Fn(fn_syn_item) => {
                                let mut has_fn = false;
                                for fn_item in syn_file.fns.iter() {
                                    if fn_item.name == fn_syn_item.name {
                                        has_fn = true;
                                        break;
                                    }
                                }
                                if !has_fn {
                                    syn_file.fns.push(fn_syn_item.clone());
                                }
                            }
                            SynApplication::Enum(enum_syn_item) => {
                                let mut has_enum = false;
                                for enum_item in syn_file.enums.iter() {
                                    if enum_item.name == enum_syn_item.name {
                                        has_enum = true;
                                        break;
                                    }
                                }
                                if !has_enum {
                                    syn_file.enums.push(enum_syn_item.clone());
                                }
                            }
                            SynApplication::Struct(struct_syn_item) => {
                                let mut has_struct = false;
                                for struct_item in syn_file.structs.iter() {
                                    if struct_item.name == struct_syn_item.name {
                                        has_struct = true;
                                        break;
                                    }
                                }
                                if !has_struct {
                                    syn_file.structs.push(struct_syn_item.clone());
                                }
                            }
                            SynApplication::Union(union_syn_item) => {
                                let mut has_union = false;
                                for union_item in syn_file.unions.iter() {
                                    if union_item.name == union_syn_item.name {
                                        has_union = true;
                                        break;
                                    }
                                }
                                if !has_union {
                                    syn_file.unions.push(union_syn_item.clone());
                                }
                            }
                            SynApplication::Trait(trait_syn_item) => {
                                let mut has_trait_item = false;
                                for trait_item in syn_file.traits.iter_mut() {
                                    if trait_item.name == trait_syn_item.name {
                                        has_trait_item = true;
                                        break;
                                    }
                                }
                                if !has_trait_item {
                                    syn_file.traits.push(trait_syn_item.clone());
                                }
                            }
                            SynApplication::Impl(impl_syn_item) => {
                                let mut has_impl_item = false;
                                for impl_item in syn_file.impls.iter_mut() {
                                    if impl_item.name == impl_syn_item.name {
                                        has_impl_item = true;
                                        break;
                                    }
                                }
                                if !has_impl_item {
                                    syn_file.impls.push(impl_syn_item.clone());
                                }
                            }
                        }
                        has_syn_file = true;
                        break;
                    }
                }
                if !has_syn_file {
                    let mut syn_file: SynFile;
                    for mod_context in self.mod_contexts.iter() {
                        if mod_context.name == mod_name {
                            syn_file = SynFile::new(mod_context);
                            match &syn_application {
                                SynApplication::Fn(fn_syn_item) => {
                                    syn_file.fns.push(fn_syn_item.clone());
                                }
                                SynApplication::Enum(enum_syn_item) => {
                                    syn_file.enums.push(enum_syn_item.clone());
                                }
                                SynApplication::Struct(struct_syn_item) => {
                                    syn_file.structs.push(struct_syn_item.clone());
                                }
                                SynApplication::Union(union_syn_item) => {
                                    syn_file.unions.push(union_syn_item.clone());
                                }
                                SynApplication::Trait(trait_syn_item) => {
                                    syn_file.traits.push(trait_syn_item.clone());
                                }
                                SynApplication::Impl(impl_syn_item) => {
                                    syn_file.impls.push(impl_syn_item.clone());
                                }
                            }
                            syn_files.push(syn_file);
                            break;
                        }
                    }
                }
            }
        }
    }

    pub fn parse_context(&self) {
        let mut name_map: BTreeMap<String, String> = BTreeMap::new();
        for mod_context in self.mod_contexts.iter() {
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
                let mut syn_files: Vec<SynFile> = Vec::new();
                self.parse_direct_applications(&direct_applications, &mut syn_files);
                self.parse_indirect_applications(&indirect_applications, &mut syn_files);

                let s = to_string(&syn_files);

                let encoded_name = encoded_name(&fn_item.name);
                name_map.insert(fn_item.name.to_string(), encoded_name.clone());
                let output_path = self
                    .crate_path
                    .join(format!("{}/{}.rs", OUT_FILE_PATH, encoded_name));
                fs::create_dir_all(output_path.parent().unwrap()).unwrap();
                let mut file = File::create(&output_path).unwrap();
                file.write_all(s.as_bytes()).unwrap();
            }
            for trait_item in mod_context.traits.iter() {
                for trait_fn in trait_item.fns.iter() {
                    let mut direct_applications: BTreeSet<String> = BTreeSet::new();
                    direct_applications.insert(trait_fn.name.clone());
                    direct_applications.extend(trait_fn.applications.clone());
                    direct_applications.extend(trait_item.applications.clone());
                    direct_applications.extend(mod_context.applications.clone());
                    let mut indirect_applications: BTreeSet<String> = BTreeSet::new();
                    parse_direct_applications(
                        self.mod_contexts,
                        &mut direct_applications,
                        &mut indirect_applications,
                    );
                    let mut syn_files: Vec<SynFile> = Vec::new();
                    self.parse_direct_applications(&direct_applications, &mut syn_files);
                    self.parse_indirect_applications(&indirect_applications, &mut syn_files);

                    let s = to_string(&syn_files);

                    let encoded_name = encoded_name(&trait_fn.name);
                    name_map.insert(trait_fn.name.to_string(), encoded_name.clone());
                    let output_path = self
                        .crate_path
                        .join(format!("{}/{}.rs", OUT_FILE_PATH, encoded_name));
                    fs::create_dir_all(output_path.parent().unwrap()).unwrap();
                    let mut file = File::create(&output_path).unwrap();
                    file.write_all(s.as_bytes()).unwrap();
                }
            }
            for impl_item in mod_context.impls.iter() {
                for impl_fn in impl_item.fns.iter() {
                    let mut direct_applications: BTreeSet<String> = BTreeSet::new();
                    direct_applications.insert(impl_fn.name.clone());
                    direct_applications.extend(impl_fn.applications.clone());
                    direct_applications.extend(impl_item.applications.clone());
                    direct_applications.extend(mod_context.applications.clone());
                    let mut indirect_applications: BTreeSet<String> = BTreeSet::new();
                    parse_direct_applications(
                        self.mod_contexts,
                        &mut direct_applications,
                        &mut indirect_applications,
                    );

                    let mut syn_files: Vec<SynFile> = Vec::new();
                    self.parse_direct_applications(&direct_applications, &mut syn_files);
                    // error!("{}: {:#?}", impl_fn.name, syn_files);
                    self.parse_indirect_applications(&indirect_applications, &mut syn_files);

                    let s = to_string(&syn_files);

                    // s = format!("{:#?}\n{:#?}\n", direct_applications, indirect_applications) + &s;

                    let encoded_name = encoded_name(&impl_fn.name);
                    name_map.insert(impl_fn.name.to_string(), encoded_name.clone());
                    let output_path = self
                        .crate_path
                        .join(format!("{}/{}.rs", OUT_FILE_PATH, encoded_name));
                    fs::create_dir_all(output_path.parent().unwrap()).unwrap();
                    let mut file = File::create(&output_path).unwrap();
                    file.write_all(s.as_bytes()).unwrap();
                }
            }
        }
        let name_map_path = self
            .crate_path
            .join(format!("{}/name_map.json", OUT_FILE_PATH));
        let file = File::create(name_map_path).unwrap();
        serde_json::to_writer_pretty(file, &name_map).unwrap();
    }
}

fn to_string(syn_files: &Vec<SynFile>) -> String {
    let mut s = String::new();
    for syn_file in syn_files.iter() {
        s = s + "// " + &syn_file.name + "\n";
        s = s + &syn_file.to_string() + "\n";
    }
    s
}
