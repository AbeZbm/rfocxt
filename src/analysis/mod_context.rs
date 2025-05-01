use super::source_info::SourceInfo;
use log::info;
use std::cmp::Ordering;
use std::collections::BTreeSet;
use syn::ItemUse;

macro_rules! impl_eq_cmp {
    ($struct:ident) => {
        impl PartialEq for $struct {
            fn eq(&self, other: &Self) -> bool {
                self.codes == other.codes
            }
        }
        impl PartialOrd for $struct {
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                if self.codes < other.codes {
                    Some(Ordering::Less)
                } else if self.codes == other.codes {
                    Some(Ordering::Equal)
                } else {
                    Some(Ordering::Greater)
                }
            }
        }
        impl Eq for $struct {}
        impl Ord for $struct {
            fn cmp(&self, other: &Self) -> Ordering {
                if self.codes < other.codes {
                    Ordering::Less
                } else if self.codes == other.codes {
                    Ordering::Equal
                } else {
                    Ordering::Greater
                }
            }
        }
    };
}

macro_rules! impl_eq_cmp_unique {
    ($struct:ident) => {
        impl PartialEq for $struct {
            fn eq(&self, other: &Self) -> bool {
                self.name == other.name
            }
        }
        impl PartialOrd for $struct {
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                if self.name < other.name {
                    Some(Ordering::Less)
                } else if self.name == other.name {
                    Some(Ordering::Equal)
                } else {
                    Some(Ordering::Greater)
                }
            }
        }
        impl Eq for $struct {}
        impl Ord for $struct {
            fn cmp(&self, other: &Self) -> Ordering {
                if self.name < other.name {
                    Ordering::Less
                } else if self.name == other.name {
                    Ordering::Equal
                } else {
                    Ordering::Greater
                }
            }
        }
    };
}

#[derive(Debug, Clone)]
pub struct UseItem {
    pub codes: String,
    pub source_info: SourceInfo,
}
impl_eq_cmp!(UseItem);

#[derive(Debug, Clone)]

pub struct StaticItem {
    pub codes: String,
    pub source_info: SourceInfo,
}
impl_eq_cmp!(StaticItem);

#[derive(Debug, Clone)]
pub struct ConstItem {
    pub codes: String,
    pub source_info: SourceInfo,
}
impl_eq_cmp!(ConstItem);

#[derive(Debug, Clone)]
pub struct FnItem {
    pub name: String,
    pub codes: String,
    pub source_info: SourceInfo,
    pub applications: BTreeSet<String>,
}
impl_eq_cmp_unique!(FnItem);

#[derive(Debug, Clone)]
pub struct MacroItem {
    pub codes: String,
    pub source_info: SourceInfo,
}
impl_eq_cmp!(MacroItem);

#[derive(Debug, Clone)]
pub struct TyAliasItem {
    pub codes: String,
    pub source_info: SourceInfo,
}
impl_eq_cmp!(TyAliasItem);

#[derive(Debug, Clone)]
pub struct OpaqueTyItem {
    pub codes: String,
    pub source_info: SourceInfo,
}
impl_eq_cmp!(OpaqueTyItem);

#[derive(Debug, Clone)]
pub struct EnumItem {
    pub name: String,
    pub codes: String,
    pub source_info: SourceInfo,
    pub applications: BTreeSet<String>,
}
impl_eq_cmp_unique!(EnumItem);

#[derive(Debug, Clone)]
pub struct StructItem {
    pub name: String,
    pub codes: String,
    pub source_info: SourceInfo,
    pub applcaitions: BTreeSet<String>,
}
impl_eq_cmp_unique!(StructItem);

#[derive(Debug, Clone)]
pub struct UnionItem {
    pub name: String,
    pub codes: String,
    pub source_info: SourceInfo,
    pub applications: BTreeSet<String>,
}
impl_eq_cmp_unique!(UnionItem);

#[derive(Debug, Clone)]

pub struct InnerTypeItem {
    pub codes: String,
    pub source_info: SourceInfo,
}
impl_eq_cmp!(InnerTypeItem);

#[derive(Debug, Clone)]
pub struct InnerConstItem {
    pub codes: String,
    pub source_info: SourceInfo,
}
impl_eq_cmp!(InnerConstItem);

#[derive(Debug, Clone)]
pub struct InnerFnItem {
    pub name: String,
    pub codes: String,
    pub source_info: SourceInfo,
    pub applications: BTreeSet<String>,
}
impl_eq_cmp_unique!(InnerFnItem);

#[derive(Debug, Clone)]
pub struct TraitItem {
    pub name: String,
    pub codes: String,
    pub source_info: SourceInfo,
    pub types: BTreeSet<InnerTypeItem>,
    pub consts: BTreeSet<InnerConstItem>,
    pub fns: BTreeSet<InnerFnItem>,
    pub applications: BTreeSet<String>,
}
impl_eq_cmp_unique!(TraitItem);

#[derive(Debug, Clone)]
pub struct TraitAliasItem {
    pub codes: String,
    pub source_info: SourceInfo,
}
impl_eq_cmp!(TraitAliasItem);

#[derive(Debug, Clone)]
pub struct ImplItem {
    pub struct_name: String,
    pub trait_name: Option<String>,
    pub codes: String,
    pub source_info: SourceInfo,
    pub types: BTreeSet<InnerTypeItem>,
    pub consts: BTreeSet<InnerConstItem>,
    pub fns: BTreeSet<InnerFnItem>,
    pub applications: BTreeSet<String>,
}

impl PartialEq for ImplItem {
    fn eq(&self, other: &ImplItem) -> bool {
        self.struct_name == other.struct_name && self.trait_name == other.trait_name
    }
}

impl PartialOrd for ImplItem {
    fn partial_cmp(&self, other: &ImplItem) -> Option<Ordering> {
        if self.struct_name < other.struct_name {
            Some(Ordering::Less)
        } else if self.struct_name == other.struct_name {
            if self.trait_name < other.trait_name {
                Some(Ordering::Less)
            } else if self.trait_name == other.trait_name {
                Some(Ordering::Equal)
            } else {
                Some(Ordering::Greater)
            }
        } else {
            Some(Ordering::Greater)
        }
    }
}

impl Eq for ImplItem {}

impl Ord for ImplItem {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.struct_name < other.struct_name {
            Ordering::Less
        } else if self.struct_name == other.struct_name {
            if self.trait_name < other.trait_name {
                Ordering::Less
            } else if self.trait_name == other.trait_name {
                Ordering::Equal
            } else {
                Ordering::Greater
            }
        } else {
            Ordering::Greater
        }
    }
}

#[derive(Debug, Clone)]
pub struct ModContext {
    pub name: String,
    pub uses: BTreeSet<UseItem>,
    pub statics: BTreeSet<StaticItem>,
    pub consts: BTreeSet<ConstItem>,
    pub fns: BTreeSet<FnItem>,
    pub macors: BTreeSet<MacroItem>,
    pub ty_aliases: BTreeSet<TyAliasItem>,
    pub opaque_tys: BTreeSet<OpaqueTyItem>,
    pub enums: BTreeSet<EnumItem>,
    pub structs: BTreeSet<StructItem>,
    pub unions: BTreeSet<UnionItem>,
    pub traits: BTreeSet<TraitItem>,
    pub trait_aliases: BTreeSet<TraitAliasItem>,
    pub impls: BTreeSet<ImplItem>,
    pub applications: BTreeSet<String>,
}

impl ModContext {
    pub fn new(name: &String) -> Self {
        ModContext {
            name: name.clone(),
            uses: BTreeSet::new(),
            statics: BTreeSet::new(),
            consts: BTreeSet::new(),
            fns: BTreeSet::new(),
            macors: BTreeSet::new(),
            ty_aliases: BTreeSet::new(),
            opaque_tys: BTreeSet::new(),
            enums: BTreeSet::new(),
            structs: BTreeSet::new(),
            unions: BTreeSet::new(),
            traits: BTreeSet::new(),
            trait_aliases: BTreeSet::new(),
            impls: BTreeSet::new(),
            applications: BTreeSet::new(),
        }
    }

    pub fn add_use(&mut self, source_info: SourceInfo) {
        if source_info.get_string() == "" {
            return;
        }
        for has_use in self.uses.iter() {
            if has_use.source_info.contains(&source_info) {
                return;
            }
        }
        let codes = source_info.get_string();
        info!("Visiting use: {}", codes);
        let use_item = UseItem {
            codes: codes,
            source_info: source_info,
        };
        self.uses.insert(use_item);
    }

    pub fn add_static(&mut self, source_info: SourceInfo) {
        let codes = source_info.get_string();
        info!("Visiting static: {}", codes);
        let static_item = StaticItem {
            codes: codes,
            source_info: source_info,
        };
        self.statics.insert(static_item);
    }

    pub fn add_const(&mut self, source_info: SourceInfo) {
        let codes = source_info.get_string();
        info!("Visiting const: {}", codes);
        let const_item = ConstItem {
            codes: codes,
            source_info: source_info,
        };
        self.consts.insert(const_item);
    }

    pub fn add_fn(&mut self, fn_item: FnItem) {
        info!("Visiting fn: {}", fn_item.name);
        self.fns.insert(fn_item);
    }

    pub fn add_macro(&mut self, source_info: SourceInfo) {
        let codes = source_info.get_string();
        info!("Visiting macro: {}", codes);
        let macro_item = MacroItem {
            codes: codes,
            source_info: source_info,
        };
        self.macors.insert(macro_item);
    }

    pub fn add_ty_alias(&mut self, source_info: SourceInfo) {
        let codes = source_info.get_string();
        info!("Visiting type alias: {}", codes);
        let ty_alias_item = TyAliasItem {
            codes: codes,
            source_info: source_info,
        };
        self.ty_aliases.insert(ty_alias_item);
    }

    pub fn add_opaque_ty(&mut self, source_info: SourceInfo) {
        let codes = source_info.get_string();
        info!("Visiting opaque type: {}", codes);
        let opaque_ty_item = OpaqueTyItem {
            codes: codes,
            source_info: source_info,
        };
        self.opaque_tys.insert(opaque_ty_item);
    }

    pub fn add_enum(&mut self, enum_item: EnumItem) {
        info!("Visiting enum: {}", enum_item.name);
        self.enums.insert(enum_item);
    }

    pub fn add_struct(&mut self, struct_item: StructItem) {
        info!("Visiting struct: {}", struct_item.name);
        self.structs.insert(struct_item);
    }

    pub fn add_union(&mut self, union_item: UnionItem) {
        info!("Visiting union: {}", union_item.name);
        self.unions.insert(union_item);
    }

    pub fn add_trait(&mut self, trait_item: TraitItem) {
        info!("Visiting trait: {}", trait_item.name);
        self.traits.insert(trait_item);
    }

    pub fn add_trait_alias(&mut self, source_info: SourceInfo) {
        let codes = source_info.get_string();
        info!("Visiting trait alias: {}", codes);
        let trait_alias_item = TraitAliasItem {
            codes: codes,
            source_info: source_info,
        };
        self.trait_aliases.insert(trait_alias_item);
    }

    pub fn add_impl(&mut self, impl_item: ImplItem) {
        if let Some(trait_name) = &impl_item.trait_name {
            info!("Visiting impl: {}\t{}", impl_item.struct_name, trait_name);
        } else {
            info!("Visiting impl: {}", impl_item.struct_name);
        }
        self.impls.insert(impl_item);
    }

    pub fn add_application(&mut self, application: String) {
        self.applications.insert(application);
    }

    pub fn extend_application(&mut self, ty_strings: BTreeSet<String>) {
        self.applications.extend(ty_strings);
    }
}
