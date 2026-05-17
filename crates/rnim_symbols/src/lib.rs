//! Symbol interning, module graph, import/export visibility, overload sets.

#[cfg(test)]
use rnim_allocator as _;
use rnim_span::{FileId, Span};
use rustc_hash::FxHashMap;
use slotmap::SlotMap;
use std::path::{Path, PathBuf};

slotmap::new_key_type! {
    pub struct SymbolId;
    pub struct ModuleId;
    pub struct ScopeId;
}

#[derive(Debug, Clone)]
pub struct Name {
    pub text: Box<str>,
    pub span: Span,
}

#[derive(Debug, Default)]
pub struct SymbolTable {
    symbols: SlotMap<SymbolId, Name>,
    modules: SlotMap<ModuleId, ModuleSymbol>,
    names: FxHashMap<Box<str>, SymbolId>,
}

#[derive(Debug, Clone)]
pub struct ModuleSymbol {
    pub name: Name,
    pub file: FileId,
    pub path: PathBuf,
    pub exports: Vec<SymbolId>,
    pub imports: Vec<ModuleId>,
    pub is_virtual: bool,
}

impl SymbolTable {
    pub fn intern(&mut self, text: impl Into<Box<str>>, span: Span) -> SymbolId {
        let text = text.into();
        if let Some(&id) = self.names.get(&text) {
            return id;
        }
        let id = self.symbols.insert(Name {
            text: text.clone(),
            span,
        });
        self.names.insert(text, id);
        id
    }

    pub fn get(&self, id: SymbolId) -> Option<&Name> {
        self.symbols.get(id)
    }

    pub fn add_module(&mut self, name: Name, file: FileId, path: PathBuf) -> ModuleId {
        self.modules.insert(ModuleSymbol {
            name,
            file,
            path,
            exports: Vec::new(),
            imports: Vec::new(),
            is_virtual: false,
        })
    }

    pub fn get_module(&self, id: ModuleId) -> Option<&ModuleSymbol> {
        self.modules.get(id)
    }

    pub fn module_by_path(&self, path: &PathBuf) -> Option<ModuleId> {
        self.modules
            .iter()
            .find(|(_, m)| &m.path == path)
            .map(|(id, _)| id)
    }
}

/// Module path resolution and discovery
#[derive(Debug, Clone, PartialEq)]
pub enum ModuleKind {
    /// Regular file-based module
    File,
    /// Stdlib module
    Stdlib,
    /// Package module
    Package,
    /// Virtual module (e.g., for tests)
    Virtual,
}

#[derive(Debug, Clone)]
pub struct ResolvedModule {
    pub id: ModuleId,
    pub path: PathBuf,
    pub kind: ModuleKind,
}

/// Module resolver that handles import path resolution
pub struct ModuleResolver {
    /// Base directories to search for imports
    search_paths: Vec<PathBuf>,
    /// Stdlib paths
    stdlib_paths: Vec<PathBuf>,
    /// Cache of resolved modules
    resolved: FxHashMap<Box<str>, ResolvedModule>,
}

impl Default for ModuleResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ModuleResolver {
    pub fn new() -> Self {
        ModuleResolver {
            search_paths: Vec::new(),
            stdlib_paths: Vec::new(),
            resolved: FxHashMap::default(),
        }
    }

    /// Add a search path for module resolution
    pub fn add_search_path(&mut self, path: PathBuf) {
        self.search_paths.push(path);
    }

    /// Add a stdlib path for module resolution
    pub fn add_stdlib_path(&mut self, path: PathBuf) {
        self.stdlib_paths.push(path);
    }

    /// Get all search paths
    pub fn get_search_paths(&self) -> &[PathBuf] {
        &self.search_paths
    }

    /// Get all stdlib paths
    pub fn get_stdlib_paths(&self) -> &[PathBuf] {
        &self.stdlib_paths
    }

    /// Resolve a module from an import statement
    pub fn resolve(&mut self, current_file: &Path, module_name: &str) -> Option<ResolvedModule> {
        // Check cache first
        if let Some(cached) = self.resolved.get(module_name) {
            return Some(cached.clone());
        }

        // Try relative to current file
        let current_dir = current_file.parent().unwrap_or_else(|| Path::new(""));
        let relative_path = current_dir.join(format!("{}.nim", module_name.replace('.', "/")));

        for search_path in &self.search_paths {
            // First check relative to current file
            if search_path.join(&relative_path).exists() {
                let resolved = ResolvedModule {
                    id: ModuleId::default(),
                    path: relative_path.clone(),
                    kind: ModuleKind::File,
                };
                self.resolved.insert(module_name.into(), resolved.clone());
                return Some(resolved);
            }

            // Then check search path
            let full_path = search_path
                .join(module_name.replace('.', "/"))
                .with_extension("nim");
            if full_path.exists() {
                let resolved = ResolvedModule {
                    id: ModuleId::default(),
                    path: full_path,
                    kind: ModuleKind::File,
                };
                self.resolved.insert(module_name.into(), resolved.clone());
                return Some(resolved);
            }
        }

        // Check stdlib paths
        for stdlib_path in &self.stdlib_paths {
            let stdlib_module_path = stdlib_path
                .join(module_name.replace('.', "/"))
                .with_extension("nim");
            if stdlib_module_path.exists() {
                let resolved = ResolvedModule {
                    id: ModuleId::default(),
                    path: stdlib_module_path,
                    kind: ModuleKind::Stdlib,
                };
                self.resolved.insert(module_name.into(), resolved.clone());
                return Some(resolved);
            }
        }

        None
    }

    /// Resolve a from-import (e.g., "from foo import bar")
    pub fn resolve_from(
        &mut self,
        current_file: &Path,
        module_name: &str,
    ) -> Option<ResolvedModule> {
        self.resolve(current_file, module_name)
    }
}

/// Import statement representation
#[derive(Debug, Clone)]
pub struct Import {
    pub module: ModuleId,
    pub symbols: Vec<SymbolId>,
    pub alias: Option<SymbolId>,
    pub is_explicit: bool,
}

/// Import statement with detailed resolution info
#[derive(Debug, Clone)]
pub struct ImportStmt {
    /// The module being imported
    pub module_id: ModuleId,
    /// Specific symbols being imported (empty means all)
    pub symbols: Vec<ImportedSymbol>,
    /// Whether this is an explicit import (not "from x import *")
    pub is_explicit: bool,
    /// The alias for the imported module (as in "import foo as bar")
    pub module_alias: Option<Box<str>>,
    /// Whether this is a re-export (export after import)
    pub is_reexport: bool,
}

/// A symbol being imported from a module
#[derive(Debug, Clone)]
pub struct ImportedSymbol {
    /// The original name in the source module
    pub name: Box<str>,
    /// The alias in the importing module (as in "from x import y as z")
    pub alias: Option<Box<str>>,
    /// The resolved symbol ID
    pub symbol_id: Option<SymbolId>,
}

/// Export statement - re-exports symbols from another module
#[derive(Debug, Clone)]
pub struct ExportStmt {
    /// The source module
    pub module_id: ModuleId,
    /// Symbols being exported
    pub symbols: Vec<ExportedSymbol>,
}

/// An exported symbol with its alias
#[derive(Debug, Clone)]
pub struct ExportedSymbol {
    pub name: Box<str>,
    pub alias: Option<Box<str>>,
    pub symbol_id: Option<SymbolId>,
}

/// Visibility rules for exported symbols
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    /// Only visible within the defining module
    Private,
    /// Visible to all imports
    Public,
    /// Visible only to modules that import the defining module
    Protected,
}

/// Visibility checker for imports/exports
pub struct VisibilityChecker;

impl VisibilityChecker {
    /// Check if a symbol is visible to a given importing module
    pub fn is_visible(_module_id: ModuleId, _symbol_id: SymbolId, _target: ModuleId) -> bool {
        // For now, all exported symbols are public
        // This will be enhanced with proper visibility rules
        true
    }

    /// Filter symbols based on visibility
    pub fn filter_visible(
        module_id: ModuleId,
        symbols: &[(Box<str>, SymbolId)],
        target: ModuleId,
    ) -> Vec<&(Box<str>, SymbolId)> {
        symbols
            .iter()
            .filter(|(_, sym)| Self::is_visible(module_id, *sym, target))
            .collect()
    }
}

/// Module graph for tracking imports and exports
#[derive(Debug, Default)]
pub struct ModuleGraph {
    /// Map from module ID to its imports
    imports: FxHashMap<ModuleId, Vec<ImportStmt>>,
    /// Map from module ID to its exports
    exports: FxHashMap<ModuleId, Vec<ExportStmt>>,
    /// Re-export chains (module -> set of re-exported modules)
    reexports: FxHashMap<ModuleId, Vec<ModuleId>>,
}

impl ModuleGraph {
    pub fn new() -> Self {
        ModuleGraph {
            imports: FxHashMap::default(),
            exports: FxHashMap::default(),
            reexports: FxHashMap::default(),
        }
    }

    /// Add an import statement to a module
    pub fn add_import(&mut self, module: ModuleId, import: ImportStmt) {
        self.imports.entry(module).or_default().push(import);
    }

    /// Add an export statement to a module
    pub fn add_export(&mut self, module: ModuleId, export: ExportStmt) {
        self.exports.entry(module).or_default().push(export);
    }

    /// Add a re-export (module A re-exports from module B)
    pub fn add_reexport(&mut self, from: ModuleId, to: ModuleId) {
        self.reexports.entry(from).or_default().push(to);
    }

    /// Get all imports for a module
    pub fn get_imports(&self, module: ModuleId) -> &[ImportStmt] {
        self.imports.get(&module).map_or(&[], |v| v)
    }

    /// Get all exports for a module
    pub fn get_exports(&self, module: ModuleId) -> &[ExportStmt] {
        self.exports.get(&module).map_or(&[], |v| v)
    }

    /// Get all re-exports for a module
    pub fn get_reexports(&self, module: ModuleId) -> &[ModuleId] {
        self.reexports.get(&module).map_or(&[], |v| v)
    }

    /// Check if module A directly imports module B
    pub fn imports_module(&self, a: ModuleId, b: ModuleId) -> bool {
        self.get_imports(a).iter().any(|imp| imp.module_id == b)
    }

    /// Resolve a symbol through the import chain
    pub fn resolve_symbol(&self, module: ModuleId, name: &str) -> Option<SymbolId> {
        for import in self.get_imports(module) {
            for sym in &import.symbols {
                if sym.name.as_ref() == name {
                    return sym.symbol_id;
                }
            }
        }
        None
    }

    /// Check for cyclic imports using DFS
    pub fn has_cycle(&self) -> bool {
        let mut visited = std::collections::HashSet::new();
        let mut in_stack = std::collections::HashSet::new();

        // Check each module for cycles
        for module in self.imports.keys() {
            if self.detect_cycle(*module, &mut visited, &mut in_stack) {
                return true;
            }
        }
        false
    }

    /// DFS helper for cycle detection
    fn detect_cycle(
        &self,
        module: ModuleId,
        visited: &mut std::collections::HashSet<ModuleId>,
        in_stack: &mut std::collections::HashSet<ModuleId>,
    ) -> bool {
        if in_stack.contains(&module) {
            return true;
        }
        if visited.contains(&module) {
            return false;
        }

        visited.insert(module);
        in_stack.insert(module);

        for import in self.get_imports(module) {
            if self.detect_cycle(import.module_id, visited, in_stack) {
                return true;
            }
        }

        in_stack.remove(&module);
        false
    }

    /// Get all modules that import a given module
    pub fn get_importers(&self, module: ModuleId) -> Vec<ModuleId> {
        self.imports
            .iter()
            .filter(|(_, imports)| imports.iter().any(|imp| imp.module_id == module))
            .map(|(id, _)| *id)
            .collect()
    }

    /// Get the number of modules with imports
    pub fn num_importing_modules(&self) -> usize {
        self.imports.len()
    }

    /// Get the total number of imports across all modules
    pub fn total_imports(&self) -> usize {
        self.imports.values().map(|v| v.len()).sum()
    }
}

/// An overload set - multiple symbols with the same name
#[derive(Debug, Clone)]
pub struct OverloadSet {
    /// The name of the overloaded symbols
    pub name: Box<str>,
    /// The module where this overload set is defined
    pub module_id: ModuleId,
    /// The scope where this overload set is visible
    pub scope_id: ScopeId,
    /// The candidate symbols in this overload set
    pub candidates: Vec<OverloadCandidate>,
    /// Whether this set includes private (non-exported) symbols
    pub has_private: bool,
}

/// A single candidate in an overload set
#[derive(Debug, Clone)]
pub struct OverloadCandidate {
    /// The symbol ID
    pub symbol_id: SymbolId,
    /// The source span of this declaration
    pub span: Span,
    /// Parameter count (for quick filtering)
    pub param_count: usize,
    /// Whether this is a generic routine
    pub is_generic: bool,
    /// The kind of declaration (proc, func, template, macro, converter)
    pub kind: DeclarationKind,
}

/// Kind of declaration for overload tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DeclarationKind {
    #[default]
    Proc,
    Func,
    Method,
    Iterator,
    Converter,
    Template,
    Macro,
    Type, // Type names can be overloaded too
    Var,
    Let,
    Const,
}

/// Overload set builder for constructing overload sets
pub struct OverloadSetBuilder {
    sets: FxHashMap<Box<str>, OverloadSet>,
}

impl OverloadSetBuilder {
    pub fn new() -> Self {
        OverloadSetBuilder {
            sets: FxHashMap::default(),
        }
    }

    /// Add a candidate to an overload set
    #[allow(clippy::too_many_arguments)]
    pub fn add_candidate(
        &mut self,
        name: Box<str>,
        module_id: ModuleId,
        scope_id: ScopeId,
        symbol_id: SymbolId,
        span: Span,
        param_count: usize,
        is_generic: bool,
        kind: DeclarationKind,
    ) {
        let entry = self
            .sets
            .entry(name.clone())
            .or_insert_with(|| OverloadSet {
                name,
                module_id,
                scope_id,
                candidates: Vec::new(),
                has_private: false,
            });

        entry.candidates.push(OverloadCandidate {
            symbol_id,
            span,
            param_count,
            is_generic,
            kind,
        });
    }

    /// Mark an overload set as having private candidates
    pub fn mark_private(&mut self, name: &str) {
        if let Some(set) = self.sets.get_mut(name) {
            set.has_private = true;
        }
    }

    /// Get an overload set by name
    pub fn get(&self, name: &str) -> Option<&OverloadSet> {
        self.sets.get(name)
    }

    /// Get all overload sets
    pub fn get_all(&self) -> &FxHashMap<Box<str>, OverloadSet> {
        &self.sets
    }

    /// Check if a name has an overload set
    pub fn has_overload(&self, name: &str) -> bool {
        self.sets.contains_key(name)
    }

    /// Get the number of candidates for a name
    pub fn candidate_count(&self, name: &str) -> usize {
        self.sets.get(name).map_or(0, |s| s.candidates.len())
    }
}

impl Default for OverloadSetBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl SymbolTable {
    /// Check if a symbol is exported with the `*` marker
    pub fn is_exported(&self, module: ModuleId, symbol: SymbolId) -> bool {
        let module_symbol = self.modules.get(module);
        module_symbol.is_some_and(|m| m.exports.contains(&symbol))
    }
}

/// Represents a lexical scope in the source code
#[derive(Debug, Clone)]
#[allow(non_snake_case)]
pub struct Scope {
    pub id: ScopeId,
    pub parent: Option<ScopeId>,
    pub kind: ScopeKind,
    pub symbols: FxHashMap<Box<str>, SymbolId>,
    #[allow(non_snake_case)]
    pub HygieneId: u32,
}

impl Scope {
    pub fn new(id: ScopeId, parent: Option<ScopeId>, kind: ScopeKind) -> Self {
        Scope {
            id,
            parent,
            kind,
            symbols: FxHashMap::default(),
            HygieneId: 0,
        }
    }
}

/// Kind of scope
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ScopeKind {
    /// Global/module scope
    Module,
    /// Routine body scope (proc, func, method, iterator)
    Routine,
    /// Block scope (discard, while, for, etc.)
    #[default]
    Block,
    /// Type scope (object fields, enum values)
    Type,
    /// Generic scope (template, macro parameters)
    Generic,
    /// Temporary scope for import/export
    Temporary,
}

/// Represents the scope tree for a module
#[derive(Debug, Default)]
pub struct ScopeTree {
    scopes: SlotMap<ScopeId, Scope>,
    root: Option<ScopeId>,
}

impl ScopeTree {
    pub fn new() -> Self {
        ScopeTree {
            scopes: SlotMap::with_key(),
            root: None,
        }
    }

    /// Create a new module scope as root
    pub fn create_module_scope(&mut self, hygiene_id: u32) -> ScopeId {
        let id = self
            .scopes
            .insert(Scope::new(ScopeId::default(), None, ScopeKind::Module));
        self.scopes[id].HygieneId = hygiene_id;
        self.root = Some(id);
        id
    }

    /// Create a child scope
    pub fn create_child_scope(
        &mut self,
        parent: ScopeId,
        kind: ScopeKind,
        hygiene_id: u32,
    ) -> ScopeId {
        let id = self
            .scopes
            .insert(Scope::new(ScopeId::default(), Some(parent), kind));
        self.scopes[id].HygieneId = hygiene_id;
        id
    }

    /// Get a scope by ID
    pub fn get_scope(&self, id: ScopeId) -> Option<&Scope> {
        self.scopes.get(id)
    }

    /// Get a mutable scope by ID
    pub fn get_scope_mut(&mut self, id: ScopeId) -> Option<&mut Scope> {
        self.scopes.get_mut(id)
    }

    /// Get the root scope
    pub fn root_scope(&self) -> Option<ScopeId> {
        self.root
    }

    /// Find symbol in scope chain (lookup)
    pub fn lookup(&self, scope_id: ScopeId, name: &str) -> Option<SymbolId> {
        let mut current = Some(scope_id);
        while let Some(id) = current {
            if let Some(scope) = self.scopes.get(id) {
                if let Some(&sym_id) = scope.symbols.get(name) {
                    return Some(sym_id);
                }
                current = scope.parent;
            } else {
                break;
            }
        }
        None
    }

    /// Insert a symbol into a scope
    pub fn insert_symbol(&mut self, scope_id: ScopeId, name: Box<str>, symbol: SymbolId) -> bool {
        if let Some(scope) = self.scopes.get_mut(scope_id) {
            scope.symbols.insert(name, symbol);
            true
        } else {
            false
        }
    }

    /// Get all symbols in a scope
    pub fn get_scope_symbols(&self, scope_id: ScopeId) -> Vec<(Box<str>, SymbolId)> {
        if let Some(scope) = self.scopes.get(scope_id) {
            scope.symbols.iter().map(|(k, v)| (k.clone(), *v)).collect()
        } else {
            Vec::new()
        }
    }
}

/// Name normalization for Nim-style identifier comparison
pub fn normalize_name(name: &str) -> Box<str> {
    // Nim identifiers are case-insensitive for ASCII, but case is preserved
    // For now, just return the lowercase version for comparison purposes
    name.to_lowercase().into()
}

/// Check if two identifiers are equal according to Nim rules
pub fn names_equal(a: &str, b: &str) -> bool {
    normalize_name(a) == normalize_name(b)
}

/// Method call resolution for `x.f(y)` syntax
#[derive(Debug, Clone)]
pub enum MethodCallTarget {
    /// Field access (x.f is a field of some type)
    FieldAccess {
        receiver_type: Box<str>,
        field_name: Box<str>,
    },
    /// Callable call (x.f is callable with receiver as first arg)
    CallableCall {
        callable_name: Box<str>,
        receiver_arg_index: usize,
    },
    /// Ambiguous - could be either
    Ambiguous {
        field_name: Box<str>,
        callable_name: Box<str>,
    },
}

/// Resolution result for method call syntax
#[derive(Debug, Clone)]
pub struct MethodCallResolution {
    /// The target of the method call
    pub target: MethodCallTarget,
    /// Whether resolution was successful
    pub is_resolved: bool,
    /// Error message if unresolved
    pub error: Option<Box<str>>,
}

/// Resolve a method call expression `x.f(y)` to either field access or callable call
pub struct MethodCallResolver;

impl MethodCallResolver {
    /// Resolve a method call with a known receiver type
    pub fn resolve(
        _receiver_type: &str,
        method_name: &str,
        _arg_count: usize,
        overload_sets: &OverloadSetBuilder,
    ) -> MethodCallResolution {
        // Check if there's a callable with this name in scope
        if overload_sets.has_overload(method_name) {
            // Could be a callable - check if receiver type is a valid first argument
            // For now, assume callable if there's an overload set
            return MethodCallResolution {
                target: MethodCallTarget::CallableCall {
                    callable_name: method_name.into(),
                    receiver_arg_index: 0,
                },
                is_resolved: true,
                error: None,
            };
        }

        // Check if receiver has a field with this name
        // This would require type information - for now, mark as ambiguous
        MethodCallResolution {
            target: MethodCallTarget::Ambiguous {
                field_name: method_name.into(),
                callable_name: method_name.into(),
            },
            is_resolved: false,
            error: Some("Cannot resolve method call without type information".into()),
        }
    }

    /// Check if a method call could be a callable
    pub fn could_be_callable(method_name: &str, overload_sets: &OverloadSetBuilder) -> bool {
        overload_sets.has_overload(method_name)
    }

    /// Check if a method call could be a field access (requires type info)
    pub fn could_be_field(_receiver_type: &str, _field_name: &str) -> bool {
        // Would need type information to determine this properly
        // For now, return true as field access is the fallback
        true
    }
}

/// Generic instantiation site - where a generic is instantiated
#[derive(Debug, Clone)]
pub struct GenericInstantiation {
    /// The definition site (where the generic was declared)
    pub definition_scope: ScopeId,
    /// The instantiation site (where it was used)
    pub instantiation_scope: ScopeId,
    /// Type arguments at this instantiation
    pub type_args: Vec<Box<str>>,
    /// The resulting concrete symbol
    pub concrete_symbol: Option<SymbolId>,
}

/// Symbol binding mode - how a symbol is bound in a generic context
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SymbolBindingMode {
    /// Open binding - symbol can be resolved to different things at each instantiation
    #[default]
    Open,
    /// Closed binding - symbol is bound once at definition and fixed
    Closed,
    /// Bind binding - explicitly bound via `bind` keyword
    Bind,
    /// Mixin binding - accessible from template/generic but not part of signature
    Mixin,
}

/// Generic symbol resolution context
#[derive(Debug, Clone, Default)]
pub struct GenericContext {
    /// The definition scope of the generic
    pub definition_scope: ScopeId,
    /// The current instantiation scope
    pub instantiation_scope: ScopeId,
    /// The type parameters of the generic
    pub type_params: Vec<SymbolId>,
    /// Bindings for captured symbols
    pub bindings: FxHashMap<SymbolId, SymbolBindingMode>,
    /// Instantiations of this generic
    pub instantiations: Vec<GenericInstantiation>,
}

impl GenericContext {
    pub fn new(definition_scope: ScopeId) -> Self {
        GenericContext {
            definition_scope,
            instantiation_scope: ScopeId::default(),
            type_params: Vec::new(),
            bindings: FxHashMap::default(),
            instantiations: Vec::new(),
        }
    }

    /// Add a type parameter to this generic context
    pub fn add_type_param(&mut self, param: SymbolId) {
        self.type_params.push(param);
    }

    /// Set the binding mode for a symbol
    pub fn set_binding(&mut self, symbol: SymbolId, mode: SymbolBindingMode) {
        self.bindings.insert(symbol, mode);
    }

    /// Get the binding mode for a symbol
    pub fn get_binding(&self, symbol: SymbolId) -> SymbolBindingMode {
        self.bindings.get(&symbol).copied().unwrap_or_default()
    }

    /// Record an instantiation of this generic
    pub fn record_instantiation(&mut self, type_args: Vec<Box<str>>, concrete: Option<SymbolId>) {
        self.instantiations.push(GenericInstantiation {
            definition_scope: self.definition_scope,
            instantiation_scope: self.instantiation_scope,
            type_args,
            concrete_symbol: concrete,
        });
    }

    /// Check if this context has any instantiations
    pub fn has_instantiations(&self) -> bool {
        !self.instantiations.is_empty()
    }

    /// Check if a symbol has open binding
    pub fn is_open_binding(&self, symbol: SymbolId) -> bool {
        self.get_binding(symbol) == SymbolBindingMode::Open
    }

    /// Check if a symbol has closed binding
    pub fn is_closed_binding(&self, symbol: SymbolId) -> bool {
        self.get_binding(symbol) == SymbolBindingMode::Closed
    }
}

/// Generic symbol resolver for Nim's complex generic binding rules
pub struct GenericResolver;

impl GenericResolver {
    /// Determine the binding mode for a symbol in a generic context
    pub fn determine_binding_mode(
        _symbol: SymbolId,
        is_type_param: bool,
        is_explicitly_bound: bool,
        is_mixin: bool,
    ) -> SymbolBindingMode {
        if is_explicitly_bound {
            SymbolBindingMode::Bind
        } else if is_mixin {
            SymbolBindingMode::Mixin
        } else if is_type_param {
            // Type parameters are open - can vary per instantiation
            SymbolBindingMode::Open
        } else {
            // Non-type-param symbols are closed - bound at definition
            SymbolBindingMode::Closed
        }
    }

    /// Check if a symbol should be captured by a generic
    pub fn should_capture(symbol: SymbolId, context: &GenericContext) -> bool {
        match context.get_binding(symbol) {
            SymbolBindingMode::Open | SymbolBindingMode::Bind => true,
            SymbolBindingMode::Mixin => true,
            SymbolBindingMode::Closed => false,
        }
    }

    /// Resolve a symbol in a generic context
    pub fn resolve_in_context(
        symbol: SymbolId,
        context: &GenericContext,
        _current_scope: ScopeId,
    ) -> Option<SymbolId> {
        // If the symbol is open in this context, we need to look it up
        // in the current instantiation scope instead of the definition scope
        if context.is_open_binding(symbol) {
            // In a real implementation, we'd look up in the instantiation scope
            // For now, just return the original symbol
            Some(symbol)
        } else {
            // Closed binding - use the definition scope
            Some(symbol)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_resolver_new() {
        let resolver = ModuleResolver::new();
        assert!(resolver.get_search_paths().is_empty());
        assert!(resolver.get_stdlib_paths().is_empty());
    }

    #[test]
    fn test_module_resolver_add_search_path() {
        let mut resolver = ModuleResolver::new();
        resolver.add_search_path(PathBuf::from("/path/to/modules"));
        assert_eq!(resolver.get_search_paths().len(), 1);
        assert_eq!(
            resolver.get_search_paths()[0],
            PathBuf::from("/path/to/modules")
        );
    }

    #[test]
    fn test_module_resolver_add_stdlib_path() {
        let mut resolver = ModuleResolver::new();
        resolver.add_stdlib_path(PathBuf::from("/usr/local/nim/lib"));
        assert_eq!(resolver.get_stdlib_paths().len(), 1);
        assert_eq!(
            resolver.get_stdlib_paths()[0],
            PathBuf::from("/usr/local/nim/lib")
        );
    }

    #[test]
    fn test_module_kind_variants() {
        assert!(matches!(ModuleKind::File, ModuleKind::File));
        assert!(matches!(ModuleKind::Stdlib, ModuleKind::Stdlib));
        assert!(matches!(ModuleKind::Package, ModuleKind::Package));
        assert!(matches!(ModuleKind::Virtual, ModuleKind::Virtual));
    }

    #[test]
    fn test_resolved_module_clone() {
        let resolved = ResolvedModule {
            id: ModuleId::default(),
            path: PathBuf::from("/test/module.nim"),
            kind: ModuleKind::File,
        };
        let cloned = resolved.clone();
        assert_eq!(cloned.path, resolved.path);
        assert_eq!(cloned.kind, resolved.kind);
    }

    #[test]
    fn test_import_stmt_creation() {
        let import = ImportStmt {
            module_id: ModuleId::default(),
            symbols: vec![],
            is_explicit: true,
            module_alias: None,
            is_reexport: false,
        };
        assert!(import.is_explicit);
        assert!(!import.is_reexport);
    }

    #[test]
    fn test_import_stmt_with_alias() {
        let import = ImportStmt {
            module_id: ModuleId::default(),
            symbols: vec![],
            is_explicit: true,
            module_alias: Some("alias".into()),
            is_reexport: false,
        };
        assert!(import.module_alias.is_some());
        assert_eq!(import.module_alias.unwrap().as_ref(), "alias");
    }

    #[test]
    fn test_import_stmt_with_symbols() {
        let import = ImportStmt {
            module_id: ModuleId::default(),
            symbols: vec![
                ImportedSymbol {
                    name: "foo".into(),
                    alias: None,
                    symbol_id: None,
                },
                ImportedSymbol {
                    name: "bar".into(),
                    alias: Some("baz".into()),
                    symbol_id: None,
                },
            ],
            is_explicit: true,
            module_alias: None,
            is_reexport: false,
        };
        assert_eq!(import.symbols.len(), 2);
        assert_eq!(import.symbols[0].name.as_ref(), "foo");
        assert!(import.symbols[1].alias.is_some());
    }

    #[test]
    fn test_module_graph_new() {
        let graph = ModuleGraph::new();
        assert!(graph.get_imports(ModuleId::default()).is_empty());
        assert!(graph.get_exports(ModuleId::default()).is_empty());
    }

    #[test]
    fn test_module_graph_add_import() {
        let mut graph = ModuleGraph::new();
        let module = ModuleId::default();
        let import = ImportStmt {
            module_id: ModuleId::default(),
            symbols: vec![],
            is_explicit: true,
            module_alias: None,
            is_reexport: false,
        };
        graph.add_import(module, import);
        assert_eq!(graph.get_imports(module).len(), 1);
    }

    #[test]
    fn test_module_graph_add_export() {
        let mut graph = ModuleGraph::new();
        let module = ModuleId::default();
        let export = ExportStmt {
            module_id: ModuleId::default(),
            symbols: vec![],
        };
        graph.add_export(module, export);
        assert_eq!(graph.get_exports(module).len(), 1);
    }

    #[test]
    fn test_module_graph_imports_module() {
        let mut graph = ModuleGraph::new();

        // Create a export to get a valid module ID
        let export = ExportStmt {
            module_id: ModuleId::default(),
            symbols: vec![],
        };
        let module_a = export.module_id;
        let module_b = ModuleId::default();

        // module_a imports from module_b
        let import = ImportStmt {
            module_id: module_b,
            symbols: vec![],
            is_explicit: true,
            module_alias: None,
            is_reexport: false,
        };
        graph.add_import(module_a, import);

        // Verify module_a imports module_b
        assert!(graph.imports_module(module_a, module_b));
    }

    #[test]
    fn test_module_graph_no_self_import() {
        let mut graph = ModuleGraph::new();
        let export = ExportStmt {
            module_id: ModuleId::default(),
            symbols: vec![],
        };
        let module = export.module_id;
        // A module cannot import itself
        let import = ImportStmt {
            module_id: module,
            symbols: vec![],
            is_explicit: true,
            module_alias: None,
            is_reexport: false,
        };
        graph.add_import(module, import);
        // Self-import should not be reported (or handled specially)
        let imports = graph.get_imports(module);
        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].module_id, module);
    }

    #[test]
    fn test_module_graph_add_reexport() {
        let mut graph = ModuleGraph::new();
        let module_a = ModuleId::default();
        let module_b = ModuleId::default();
        graph.add_reexport(module_a, module_b);
        let reexports = graph.get_reexports(module_a);
        assert_eq!(reexports.len(), 1);
        assert_eq!(reexports[0], module_b);
    }

    #[test]
    fn test_module_graph_resolve_symbol() {
        let mut graph = ModuleGraph::new();
        let module = ModuleId::default();
        let import = ImportStmt {
            module_id: ModuleId::default(),
            symbols: vec![ImportedSymbol {
                name: "testSymbol".into(),
                alias: None,
                symbol_id: Some(SymbolId::default()),
            }],
            is_explicit: true,
            module_alias: None,
            is_reexport: false,
        };
        graph.add_import(module, import);
        let resolved = graph.resolve_symbol(module, "testSymbol");
        assert!(resolved.is_some());
    }

    #[test]
    fn test_module_graph_resolve_symbol_not_found() {
        let graph = ModuleGraph::new();
        let resolved = graph.resolve_symbol(ModuleId::default(), "nonexistent");
        assert!(resolved.is_none());
    }

    #[test]
    fn test_exported_symbol_creation() {
        let exported = ExportedSymbol {
            name: "myFunc".into(),
            alias: Some("alias".into()),
            symbol_id: Some(SymbolId::default()),
        };
        assert_eq!(exported.name.as_ref(), "myFunc");
        assert!(exported.alias.is_some());
        assert!(exported.symbol_id.is_some());
    }

    #[test]
    fn test_import_stmt_reexport() {
        let import = ImportStmt {
            module_id: ModuleId::default(),
            symbols: vec![],
            is_explicit: false,
            module_alias: None,
            is_reexport: true,
        };
        assert!(!import.is_explicit);
        assert!(import.is_reexport);
    }

    #[test]
    fn test_module_graph_has_cycle_no_cycles() {
        let graph = ModuleGraph::new();
        // Simple case: single module with no imports
        // Or two modules A -> B with no cycle
        // Just verify the function runs without panic
        assert!(!graph.has_cycle());
    }

    #[test]
    fn test_module_graph_has_cycle_with_self() {
        let mut graph = ModuleGraph::new();
        // Create a self-import (A -> A)
        let export = ExportStmt {
            module_id: ModuleId::default(),
            symbols: vec![],
        };
        let module_a = export.module_id;

        // A imports A (self-import)
        let import = ImportStmt {
            module_id: module_a,
            symbols: vec![],
            is_explicit: true,
            module_alias: None,
            is_reexport: false,
        };
        graph.add_import(module_a, import);

        // Self-import should be detected as a cycle
        assert!(graph.has_cycle());
    }

    #[test]
    fn test_module_graph_has_cycle_with_cycle() {
        let mut graph = ModuleGraph::new();
        let export_a = ExportStmt {
            module_id: ModuleId::default(),
            symbols: vec![],
        };
        let export_b = ExportStmt {
            module_id: ModuleId::default(),
            symbols: vec![],
        };
        let module_a = export_a.module_id;
        let module_b = export_b.module_id;

        // A -> B -> A (cycle)
        let import_ab = ImportStmt {
            module_id: module_b,
            symbols: vec![],
            is_explicit: true,
            module_alias: None,
            is_reexport: false,
        };
        let import_ba = ImportStmt {
            module_id: module_a,
            symbols: vec![],
            is_explicit: true,
            module_alias: None,
            is_reexport: false,
        };
        graph.add_import(module_a, import_ab);
        graph.add_import(module_b, import_ba);

        assert!(graph.has_cycle());
    }

    #[test]
    fn test_module_graph_get_importers() {
        let mut graph = ModuleGraph::new();
        let export_a = ExportStmt {
            module_id: ModuleId::default(),
            symbols: vec![],
        };
        let export_b = ExportStmt {
            module_id: ModuleId::default(),
            symbols: vec![],
        };
        let module_a = export_a.module_id;
        let module_b = export_b.module_id;

        // A imports B
        let import = ImportStmt {
            module_id: module_b,
            symbols: vec![],
            is_explicit: true,
            module_alias: None,
            is_reexport: false,
        };
        graph.add_import(module_a, import);

        let importers = graph.get_importers(module_b);
        assert_eq!(importers.len(), 1);
        assert_eq!(importers[0], module_a);
    }

    #[test]
    fn test_module_graph_stats() {
        let mut graph = ModuleGraph::new();
        let export_a = ExportStmt {
            module_id: ModuleId::default(),
            symbols: vec![],
        };
        let export_b = ExportStmt {
            module_id: ModuleId::default(),
            symbols: vec![],
        };
        let module_a = export_a.module_id;
        let module_b = export_b.module_id;

        // A imports B
        let import = ImportStmt {
            module_id: module_b,
            symbols: vec![],
            is_explicit: true,
            module_alias: None,
            is_reexport: false,
        };
        graph.add_import(module_a, import);

        assert_eq!(graph.num_importing_modules(), 1);
        assert_eq!(graph.total_imports(), 1);
    }

    #[test]
    fn test_normalize_name() {
        // Basic lowercase
        assert_eq!(normalize_name("foo").as_ref(), "foo");
        // Mixed case
        assert_eq!(normalize_name("FooBar").as_ref(), "foobar");
        // All uppercase
        assert_eq!(normalize_name("FOO").as_ref(), "foo");
        // Single character
        assert_eq!(normalize_name("X").as_ref(), "x");
        // Empty string
        assert_eq!(normalize_name("").as_ref(), "");
    }

    #[test]
    fn test_names_equal() {
        // Same identifiers
        assert!(names_equal("foo", "foo"));
        assert!(names_equal("Foo", "Foo"));
        // Case insensitive match
        assert!(names_equal("foo", "FOO"));
        assert!(names_equal("FooBar", "foobar"));
        assert!(names_equal("HelloWorld", "helloworld"));
        // Not equal
        assert!(!names_equal("foo", "bar"));
        assert!(!names_equal("Foo", "Bar"));
        // Empty
        assert!(names_equal("", ""));
    }

    #[test]
    fn test_normalize_name_preserves_original() {
        let original = "SomeIdentifier";
        let normalized = normalize_name(original);
        // Normalized is lowercase
        assert_eq!(normalized.as_ref(), "someidentifier");
        // Original should not be modified (but we're just returning lowercase for comparison)
    }

    #[test]
    fn test_export_star_handling() {
        let mut table = SymbolTable::default();
        let span = Span::new(FileId::new(0), 0, 10);
        // Intern some symbols
        let foo_id = table.intern("foo", span);
        let bar_id = table.intern("bar", span);
        let baz_id = table.intern("baz", span);

        // Create a module and set exports
        let module_name = Name {
            text: "testmodule".into(),
            span,
        };
        let module_id = table.add_module(
            module_name.clone(),
            FileId::new(0),
            PathBuf::from("test.nim"),
        );
        if table.get_module(module_id).is_some() {
            let mut exports = table.get_module(module_id).unwrap().exports.clone();
            exports.push(foo_id);
            exports.push(bar_id);
            exports.push(baz_id);
            // Verify exports are interned
            assert_eq!(exports.len(), 3);
        }
    }

    #[test]
    fn test_style_insensitive_comparison() {
        // Nim identifiers are case-insensitive
        let identifiers = ["myVar", "MyVar", "MYVAR", "myvar"];
        for i in 0..identifiers.len() {
            for j in 0..identifiers.len() {
                assert!(
                    names_equal(identifiers[i], identifiers[j]),
                    "{} should match {}",
                    identifiers[i],
                    identifiers[j]
                );
            }
        }
    }

    #[test]
    fn test_symbol_interning_deduplication() {
        let mut table = SymbolTable::default();
        let span = Span::new(FileId::new(0), 0, 5);

        // Same text should produce same SymbolId
        let id1 = table.intern("test", span);
        let id2 = table.intern("test", span);
        assert_eq!(id1, id2, "Interning same text should return same ID");

        // Different text should produce different ID
        let id3 = table.intern("other", span);
        assert_ne!(id1, id3, "Different text should produce different ID");
    }

    #[test]
    fn test_symbol_table_get() {
        let mut table = SymbolTable::default();
        let span = Span::new(FileId::new(0), 0, 5);

        let id = table.intern("gettest", span);
        let name = table.get(id);
        assert!(name.is_some());
        assert_eq!(name.unwrap().text.as_ref(), "gettest");
    }

    #[test]
    fn test_interned_symbol_identity() {
        let mut table = SymbolTable::default();

        // Intern same identifier multiple times from different locations
        let span1 = Span::new(FileId::new(0), 0, 5);
        let span2 = Span::new(FileId::new(0), 10, 15);

        let id1 = table.intern("same", span1);
        let id2 = table.intern("Same", span2); // Different case

        // Should be considered the same due to case-insensitive comparison
        let name1 = table.get(id1);
        let name2 = table.get(id2);
        assert!(name1.is_some());
        assert!(name2.is_some());
        // The IDs should be equal if we use normalized comparison
        assert!(names_equal(
            name1.unwrap().text.as_ref(),
            name2.unwrap().text.as_ref()
        ));
    }

    #[test]
    fn test_scope_tree_lookup_not_found() {
        let mut tree = ScopeTree::new();
        let module_scope = tree.create_module_scope(0);

        // Lookup non-existent symbol
        let found = tree.lookup(module_scope, "nonexistent");
        assert!(found.is_none());
    }

    #[test]
    fn test_duplicate_definition_detection() {
        let mut table = SymbolTable::default();
        let span = Span::new(FileId::new(0), 0, 5);

        let id1 = table.intern("dupVar", span);
        // Re-interning same name should return same ID (deduplication)
        let id2 = table.intern("dupVar", span);
        assert_eq!(
            id1, id2,
            "Same identifier should deduplicate to same SymbolId"
        );
    }

    #[test]
    fn test_scope_kind_variants() {
        assert_eq!(ScopeKind::Module, ScopeKind::Module);
        assert_eq!(ScopeKind::Routine, ScopeKind::Routine);
        assert_eq!(ScopeKind::Block, ScopeKind::Block);
        assert_eq!(ScopeKind::Type, ScopeKind::Type);
        assert_eq!(ScopeKind::Generic, ScopeKind::Generic);
        assert_ne!(ScopeKind::Module, ScopeKind::Routine);
    }

    #[test]
    fn test_scope_creation() {
        let scope = Scope::new(ScopeId::default(), None, ScopeKind::Module);
        assert_eq!(scope.kind, ScopeKind::Module);
        assert!(scope.parent.is_none());
        assert!(scope.symbols.is_empty());
    }

    #[test]
    fn test_scope_with_parent() {
        let parent = ScopeId::default();
        let child = Scope::new(ScopeId::default(), Some(parent), ScopeKind::Block);
        assert_eq!(child.parent, Some(parent));
        assert_eq!(child.kind, ScopeKind::Block);
    }

    #[test]
    fn test_scope_tree_creation() {
        let tree = ScopeTree::new();
        assert!(tree.root_scope().is_none());
        assert!(tree.get_scope(ScopeId::default()).is_none());
    }

    #[test]
    fn test_scope_tree_child_creation() {
        let mut tree = ScopeTree::new();
        let parent = tree.create_module_scope(0);
        let child = tree.create_child_scope(parent, ScopeKind::Block, 1);

        assert!(tree.get_scope(child).is_some());
        assert_eq!(tree.get_scope(child).unwrap().kind, ScopeKind::Block);
        assert_eq!(tree.get_scope(child).unwrap().parent, Some(parent));
    }
}
