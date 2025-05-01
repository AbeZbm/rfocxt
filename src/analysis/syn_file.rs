use super::mod_context::ModContext;
use std::cmp::Ordering;
use syn::{
    parse_str, ImplItemConst, ImplItemFn, ImplItemType, ItemConst, ItemEnum, ItemFn, ItemMacro,
    ItemStatic, ItemStruct, ItemTrait, ItemTraitAlias, ItemType, ItemUnion, ItemUse,
    TraitItemConst, TraitItemFn, TraitItemType,
};

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

pub struct UseSynItem {
    pub item: ItemUse,
}

pub struct StaticSynItem {
    pub item: ItemStatic,
}

pub struct ConstSynItem {
    pub item: ItemConst,
}

pub struct FnSynItem {
    pub name: String,
    pub item: ItemFn,
}

impl_eq_cmp_unique!(FnSynItem);

pub struct MarcroSynItem {
    pub item: ItemMacro,
}

pub struct TyAliasSynItem {
    pub item: ItemType,
}

pub struct EnumSynItem {
    pub name: String,
    pub item: ItemEnum,
}

impl_eq_cmp_unique!(EnumSynItem);

pub struct StructSynItem {
    pub name: String,
    pub item: ItemStruct,
}

impl_eq_cmp_unique!(StructSynItem);

pub struct UnionSynItem {
    pub name: String,
    pub item: ItemUnion,
}

impl_eq_cmp_unique!(UnionSynItem);

pub struct TraitTypeSynItem {
    pub item: TraitItemType,
}

pub struct TraitConstSynItem {
    pub item: TraitItemConst,
}

pub struct TraitFnSynItem {
    pub name: String,
    pub item: TraitItemFn,
}

impl_eq_cmp_unique!(TraitFnSynItem);

pub struct TraitSynItem {
    pub name: String,
    pub item: ItemTrait,
    pub types: Vec<TraitTypeSynItem>,
    pub consts: Vec<TraitConstSynItem>,
    pub fns: Vec<TraitFnSynItem>,
}

impl_eq_cmp_unique!(TraitSynItem);

pub struct TraitAliasSynItem {
    pub item: ItemTraitAlias,
}

pub struct ImplTypeSynItem {
    pub item: ImplItemType,
}

pub struct ImplConstSynItem {
    pub item: ImplItemConst,
}

pub struct ImplFnSynItem {
    pub name: String,
    pub item: ImplItemFn,
}

impl_eq_cmp_unique!(ImplFnSynItem);

pub struct ImplSynItem {
    pub struct_name: String,
    pub trait_name: Option<String>,
    pub types: Vec<ImplTypeSynItem>,
    pub consts: Vec<ImplConstSynItem>,
    pub fns: Vec<ImplFnSynItem>,
}

impl PartialEq for ImplSynItem {
    fn eq(&self, other: &ImplSynItem) -> bool {
        self.struct_name == other.struct_name && self.trait_name == other.trait_name
    }
}

impl PartialOrd for ImplSynItem {
    fn partial_cmp(&self, other: &ImplSynItem) -> Option<Ordering> {
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

impl Eq for ImplSynItem {}

impl Ord for ImplSynItem {
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

pub struct SynFile {
    pub name: String,
    pub uses: Vec<UseSynItem>,
    pub statics: Vec<StaticSynItem>,
    pub consts: Vec<ConstSynItem>,
    pub fns: Vec<FnSynItem>,
    pub macors: Vec<MarcroSynItem>,
    pub ty_aliases: Vec<TyAliasSynItem>,
    pub enums: Vec<EnumSynItem>,
    pub structs: Vec<StructSynItem>,
    pub unions: Vec<UnionSynItem>,
    pub traits: Vec<TraitSynItem>,
    pub trait_aliases: Vec<TraitAliasSynItem>,
    pub impls: Vec<ImplSynItem>,
}

impl SynFile {
    pub fn new(mod_context: &ModContext) -> Self {
        let name = mod_context.name.clone();
        let mut uses: Vec<UseSynItem> = Vec::new();
        for use_item in mod_context.uses.iter() {
            let item_use = parse_str::<ItemUse>(&use_item.codes);
            if let Ok(item_use) = item_use {
                let use_syn_item = UseSynItem { item: item_use };
                uses.push(use_syn_item);
            }
        }
        let mut statics: Vec<StaticSynItem> = Vec::new();
        for static_item in mod_context.statics.iter() {
            let item_static = parse_str::<ItemStatic>(&static_item.codes);
            if let Ok(item_static) = item_static {
                let static_syn_item = StaticSynItem { item: item_static };
                statics.push(static_syn_item);
            }
        }
        let mut consts: Vec<ConstSynItem> = Vec::new();
        for const_item in mod_context.consts.iter() {
            let item_const = parse_str::<ItemConst>(&const_item.codes);
            if let Ok(item_const) = item_const {
                let const_syn_item = ConstSynItem { item: item_const };
                consts.push(const_syn_item);
            }
        }
        let mut macros: Vec<MarcroSynItem> = Vec::new();
        for marco_item in mod_context.macors.iter() {
            let item_macro = parse_str::<ItemMacro>(&marco_item.codes);
            if let Ok(item_macro) = item_macro {
                let macro_syn_item = MarcroSynItem { item: item_macro };
                macros.push(macro_syn_item);
            }
        }
        let mut ty_aliases: Vec<TyAliasSynItem> = Vec::new();
        for ty_alias in mod_context.ty_aliases.iter() {
            let item_type = parse_str::<ItemType>(&ty_alias.codes);
            if let Ok(item_type) = item_type {
                let ty_alias_syn_item = TyAliasSynItem { item: item_type };
                ty_aliases.push(ty_alias_syn_item);
            }
        }
        for opque_ty in mod_context.opaque_tys.iter() {
            let item_type = parse_str::<ItemType>(&opque_ty.codes);
            if let Ok(item_type) = item_type {
                let ty_alias_syn_item = TyAliasSynItem { item: item_type };
                ty_aliases.push(ty_alias_syn_item);
            }
        }
        let mut trait_aliases: Vec<TraitAliasSynItem> = Vec::new();
        for trait_alias in mod_context.trait_aliases.iter() {
            let item_trait_alias = parse_str::<ItemTraitAlias>(&trait_alias.codes);
            if let Ok(item_trait_alias) = item_trait_alias {
                let trait_alias_syn_item = TraitAliasSynItem {
                    item: item_trait_alias,
                };
                trait_aliases.push(trait_alias_syn_item);
            }
        }

        SynFile {
            name: name,
            uses: uses,
            statics: statics,
            consts: consts,
            fns: Vec::new(),
            macors: macros,
            ty_aliases: ty_aliases,
            enums: Vec::new(),
            structs: Vec::new(),
            unions: Vec::new(),
            traits: Vec::new(),
            trait_aliases: trait_aliases,
            impls: Vec::new(),
        }
    }
}
