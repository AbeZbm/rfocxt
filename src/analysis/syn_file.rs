use super::mod_context::{ImplItem, ModContext, TraitItem};
use prettyplease::unparse;
use quote::quote;
use std::cmp::Ordering;
use syn::{
    parse2, parse_str, ImplItemConst, ImplItemFn, ImplItemType, Item, ItemConst, ItemEnum, ItemFn,
    ItemImpl, ItemMacro, ItemStatic, ItemStruct, ItemTrait, ItemTraitAlias, ItemType, ItemUnion,
    ItemUse, ReturnType, TraitItemConst, TraitItemFn, TraitItemType,
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

#[derive(Debug, Clone)]

pub struct UseSynItem {
    pub item: ItemUse,
}

impl UseSynItem {
    fn to_item(&self) -> Item {
        Item::Use(self.item.clone())
    }
}

#[derive(Debug, Clone)]

pub struct StaticSynItem {
    pub item: ItemStatic,
}

impl StaticSynItem {
    fn to_item(&self) -> Item {
        Item::Static(self.item.clone())
    }
}

#[derive(Debug, Clone)]

pub struct ConstSynItem {
    pub item: ItemConst,
}

impl ConstSynItem {
    fn to_item(&self) -> Item {
        Item::Const(self.item.clone())
    }
}

#[derive(Debug, Clone)]
pub struct FnSynItem {
    pub name: String,
    pub item: ItemFn,
}

impl FnSynItem {
    fn to_item(&self) -> Item {
        Item::Fn(self.item.clone())
    }
}

impl_eq_cmp_unique!(FnSynItem);

#[derive(Debug, Clone)]

pub struct MacroSynItem {
    pub item: ItemMacro,
}

impl MacroSynItem {
    fn to_item(&self) -> Item {
        Item::Macro(self.item.clone())
    }
}

#[derive(Debug, Clone)]

pub struct TyAliasSynItem {
    pub item: ItemType,
}

impl TyAliasSynItem {
    fn to_item(&self) -> Item {
        Item::Type(self.item.clone())
    }
}

#[derive(Debug, Clone)]

pub struct EnumSynItem {
    pub name: String,
    pub item: ItemEnum,
}

impl EnumSynItem {
    fn to_item(&self) -> Item {
        Item::Enum(self.item.clone())
    }
}

impl_eq_cmp_unique!(EnumSynItem);

#[derive(Debug, Clone)]

pub struct StructSynItem {
    pub name: String,
    pub item: ItemStruct,
}

impl StructSynItem {
    fn to_item(&self) -> Item {
        Item::Struct(self.item.clone())
    }
}

impl_eq_cmp_unique!(StructSynItem);

#[derive(Debug, Clone)]

pub struct UnionSynItem {
    pub name: String,
    pub item: ItemUnion,
}

impl UnionSynItem {
    fn to_item(&self) -> Item {
        Item::Union(self.item.clone())
    }
}

impl_eq_cmp_unique!(UnionSynItem);

#[derive(Debug, Clone)]

pub struct TraitTypeSynItem {
    pub item: TraitItemType,
}

#[derive(Debug, Clone)]

pub struct TraitConstSynItem {
    pub item: TraitItemConst,
}

#[derive(Debug, Clone)]

pub struct TraitFnSynItem {
    pub name: String,
    pub item: TraitItemFn,
}

impl_eq_cmp_unique!(TraitFnSynItem);

#[derive(Debug, Clone)]

pub struct TraitSynItem {
    pub name: String,
    pub item: ItemTrait,
    pub types: Vec<TraitTypeSynItem>,
    pub consts: Vec<TraitConstSynItem>,
    pub fns: Vec<TraitFnSynItem>,
}

impl TraitSynItem {
    pub fn from_trait_item(trait_item: &TraitItem) -> Option<Self> {
        let item_trait = parse_str::<ItemTrait>(&trait_item.codes);
        if let Ok(mut item_trait) = item_trait {
            item_trait.items.clear();

            let trait_name = trait_item.name.clone();
            let mut types: Vec<TraitTypeSynItem> = Vec::new();
            let mut consts: Vec<TraitConstSynItem> = Vec::new();
            let mut fns: Vec<TraitFnSynItem> = Vec::new();

            for trait_type in trait_item.types.iter() {
                let item_trait_type = parse_str::<TraitItemType>(&trait_type.codes);
                if let Ok(item_trait_type) = item_trait_type {
                    let trait_type_syn_item = TraitTypeSynItem {
                        item: item_trait_type,
                    };
                    types.push(trait_type_syn_item);
                }
            }
            for trait_const in trait_item.consts.iter() {
                let item_trait_const = parse_str::<TraitItemConst>(&trait_const.codes);
                if let Ok(item_trait_const) = item_trait_const {
                    let trait_const_syn_item = TraitConstSynItem {
                        item: item_trait_const,
                    };
                    consts.push(trait_const_syn_item);
                }
            }
            for trait_fn in trait_item.fns.iter() {
                let item_trait_fn = parse_str::<TraitItemFn>(&trait_fn.codes);
                if let Ok(mut item_trait_fn) = item_trait_fn {
                    if let Some(default) = &mut item_trait_fn.default {
                        default.stmts.clear();
                    }
                    let trait_fn_syn_item = TraitFnSynItem {
                        name: trait_fn.name.clone(),
                        item: item_trait_fn,
                    };
                    fns.push(trait_fn_syn_item);
                }
            }
            return Some(TraitSynItem {
                name: trait_name,
                item: item_trait,
                types: types,
                consts: consts,
                fns: fns,
            });
        }
        None
    }

    pub fn add_direct_application_trait(&mut self, other: &Self) {
        for other_trait_fn in other.fns.iter() {
            if let Some(default) = &other_trait_fn.item.default {
                if default.stmts.is_empty() {
                    continue;
                }
                for trait_fn in self.fns.iter_mut() {
                    if trait_fn.name == other_trait_fn.name {
                        trait_fn.item = other_trait_fn.item.clone();
                        break;
                    }
                }
            }
        }
    }

    fn to_item(&self) -> Item {
        let mut item = self.item.clone();
        for trait_type in self.types.iter() {
            item.items
                .push(syn::TraitItem::Type(trait_type.item.clone()));
        }
        for trait_const in self.consts.iter() {
            item.items
                .push(syn::TraitItem::Const(trait_const.item.clone()));
        }
        for trait_fn in self.fns.iter() {
            item.items.push(syn::TraitItem::Fn(trait_fn.item.clone()));
        }
        Item::Trait(item)
    }
}

impl_eq_cmp_unique!(TraitSynItem);

#[derive(Debug, Clone)]

pub struct TraitAliasSynItem {
    pub item: ItemTraitAlias,
}

impl TraitAliasSynItem {
    fn to_item(&self) -> Item {
        Item::TraitAlias(self.item.clone())
    }
}

#[derive(Debug, Clone)]

pub struct ImplTypeSynItem {
    pub item: ImplItemType,
}

#[derive(Debug, Clone)]

pub struct ImplConstSynItem {
    pub item: ImplItemConst,
}

#[derive(Debug, Clone)]

pub struct ImplFnSynItem {
    pub name: String,
    pub item: ImplItemFn,
}

impl_eq_cmp_unique!(ImplFnSynItem);

#[derive(Debug, Clone)]

pub struct ImplSynItem {
    pub name: String,
    pub struct_name: String,
    pub trait_name: Option<String>,
    pub item: ItemImpl,
    pub types: Vec<ImplTypeSynItem>,
    pub consts: Vec<ImplConstSynItem>,
    pub fns: Vec<ImplFnSynItem>,
}

impl ImplSynItem {
    pub fn from_impl_item(impl_item: &ImplItem) -> Option<Self> {
        let item_impl = parse_str::<ItemImpl>(&impl_item.codes);
        if let Ok(mut item_impl) = item_impl {
            item_impl.items.clear();

            let impl_name = impl_item.name.clone();
            let struct_name = impl_item.struct_name.clone();
            let trait_name = impl_item.trait_name.clone();
            let mut types: Vec<ImplTypeSynItem> = Vec::new();
            let mut consts: Vec<ImplConstSynItem> = Vec::new();
            let mut fns: Vec<ImplFnSynItem> = Vec::new();

            for impl_type in impl_item.types.iter() {
                let item_impl_type = parse_str::<ImplItemType>(&impl_type.codes);
                if let Ok(item_impl_type) = item_impl_type {
                    let impl_type_syn_item = ImplTypeSynItem {
                        item: item_impl_type,
                    };
                    types.push(impl_type_syn_item);
                }
            }
            for impl_const in impl_item.consts.iter() {
                let item_impl_const = parse_str::<ImplItemConst>(&impl_const.codes);
                if let Ok(item_impl_const) = item_impl_const {
                    let impl_const_syn_item = ImplConstSynItem {
                        item: item_impl_const,
                    };
                    consts.push(impl_const_syn_item);
                }
            }
            for impl_fn in impl_item.fns.iter() {
                let item_impl_fn = parse_str::<ImplItemFn>(&impl_fn.codes);
                if let Ok(mut item_impl_fn) = item_impl_fn {
                    let output = item_impl_fn.sig.output.clone();
                    if let ReturnType::Type(_, output) = output {
                        let output = format!("{:#?}", *output);
                        if !(output.contains("Self")
                            || output.contains(struct_name.split("::").last().unwrap()))
                        {
                            item_impl_fn.block.stmts.clear();
                        }
                    }
                    let impl_fn_syn_item = ImplFnSynItem {
                        name: impl_fn.name.clone(),
                        item: item_impl_fn,
                    };
                    fns.push(impl_fn_syn_item);
                }
            }
            return Some(ImplSynItem {
                name: impl_name,
                struct_name: struct_name,
                trait_name: trait_name,
                item: item_impl,
                types: types,
                consts: consts,
                fns: fns,
            });
        }
        None
    }

    pub fn add_direct_application_impl(&mut self, other: &Self) {
        for other_fn_item in other.fns.iter() {
            if other_fn_item.item.block.stmts.is_empty() {
                continue;
            }
            for fn_item in self.fns.iter_mut() {
                if fn_item.name == other_fn_item.name {
                    fn_item.item = other_fn_item.item.clone();
                    break;
                }
            }
        }
    }

    fn to_item(&self) -> Item {
        let mut item = self.item.clone();
        for impl_type in self.types.iter() {
            item.items.push(syn::ImplItem::Type(impl_type.item.clone()));
        }
        for impl_const in self.consts.iter() {
            item.items
                .push(syn::ImplItem::Const(impl_const.item.clone()));
        }
        for impl_fn in self.fns.iter() {
            item.items.push(syn::ImplItem::Fn(impl_fn.item.clone()));
        }
        Item::Impl(item)
    }
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

#[derive(Debug, Clone)]

pub struct SynFile {
    pub name: String,
    pub uses: Vec<UseSynItem>,
    pub statics: Vec<StaticSynItem>,
    pub consts: Vec<ConstSynItem>,
    pub fns: Vec<FnSynItem>,
    pub macros: Vec<MacroSynItem>,
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
        let mut macros: Vec<MacroSynItem> = Vec::new();
        for marco_item in mod_context.macors.iter() {
            let item_macro = parse_str::<ItemMacro>(&marco_item.codes);
            if let Ok(item_macro) = item_macro {
                let macro_syn_item = MacroSynItem { item: item_macro };
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
            macros: macros,
            ty_aliases: ty_aliases,
            enums: Vec::new(),
            structs: Vec::new(),
            unions: Vec::new(),
            traits: Vec::new(),
            trait_aliases: trait_aliases,
            impls: Vec::new(),
        }
    }

    pub fn to_string(&self) -> String {
        // pub name: String,
        // pub uses: Vec<UseSynItem>,
        // pub statics: Vec<StaticSynItem>,
        // pub consts: Vec<ConstSynItem>,
        // pub fns: Vec<FnSynItem>,
        // pub macors: Vec<MacroSynItem>,
        // pub ty_aliases: Vec<TyAliasSynItem>,
        // pub enums: Vec<EnumSynItem>,
        // pub structs: Vec<StructSynItem>,
        // pub unions: Vec<UnionSynItem>,
        // pub traits: Vec<TraitSynItem>,
        // pub trait_aliases: Vec<TraitAliasSynItem>,
        // pub impls: Vec<ImplSynItem>,
        let mut items: Vec<Item> = Vec::new();
        items.extend(self.uses.iter().map(|use_item| use_item.to_item()));
        items.extend(self.statics.iter().map(|static_item| static_item.to_item()));
        items.extend(self.consts.iter().map(|const_item| const_item.to_item()));
        items.extend(self.fns.iter().map(|fn_item| fn_item.to_item()));
        items.extend(self.macros.iter().map(|macro_item| macro_item.to_item()));
        items.extend(
            self.ty_aliases
                .iter()
                .map(|ty_alias_item| ty_alias_item.to_item()),
        );
        items.extend(self.enums.iter().map(|enum_item| enum_item.to_item()));
        items.extend(self.structs.iter().map(|struct_item| struct_item.to_item()));
        items.extend(self.unions.iter().map(|union_item| union_item.to_item()));
        items.extend(self.traits.iter().map(|trait_item| trait_item.to_item()));
        items.extend(
            self.trait_aliases
                .iter()
                .map(|trait_alias_item| trait_alias_item.to_item()),
        );
        items.extend(self.impls.iter().map(|impl_item| impl_item.to_item()));

        let tokens = quote! {#(#items)*};
        let syntax: syn::File = parse2(tokens).unwrap();
        unparse(&syntax)
    }
}

#[derive(Debug, Clone)]

pub enum SynApplication {
    Fn(FnSynItem),
    Enum(EnumSynItem),
    Struct(StructSynItem),
    Union(UnionSynItem),
    Trait(TraitSynItem),
    Impl(ImplSynItem),
}
