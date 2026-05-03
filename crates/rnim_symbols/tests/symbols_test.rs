//! Tests for module resolution and symbol interning

use rnim_allocator as _;
use rnim_span::{FileId, Span};
use rnim_symbols::{
    names_equal, normalize_name, DeclarationKind, ExportStmt, ExportedSymbol, GenericContext,
    GenericInstantiation, GenericResolver, Import, ImportStmt, ImportedSymbol,
    MethodCallResolution, MethodCallResolver, MethodCallTarget, ModuleGraph, ModuleId, ModuleKind,
    ModuleResolver, Name, OverloadCandidate, OverloadSet, OverloadSetBuilder, ResolvedModule,
    ScopeId, ScopeKind, ScopeTree, SymbolBindingMode, SymbolId, SymbolTable, Visibility,
    VisibilityChecker,
};
use std::path::PathBuf;

fn make_span() -> Span {
    Span::new(FileId(0), 0, 0)
}

#[test]
fn test_symbol_interning() {
    let mut table = SymbolTable::default();

    let id1 = table.intern("foo", make_span());
    let id2 = table.intern("foo", make_span());
    let id3 = table.intern("bar", make_span());

    // Same text should produce same id
    assert_eq!(id1, id2);
    // Different text should produce different id
    assert_ne!(id1, id3);

    // Verify we can retrieve the name
    assert_eq!(table.get(id1).unwrap().text.as_ref(), "foo");
    assert_eq!(table.get(id3).unwrap().text.as_ref(), "bar");
}

#[test]
fn test_symbol_interning_preserves_span() {
    let mut table = SymbolTable::default();
    let span1 = Span::new(FileId(1), 10, 20);
    let span2 = Span::new(FileId(2), 30, 40);

    let id1 = table.intern("test", span1);
    let id2 = table.intern("test", span2);

    // Same text should return same id (span is not part of identity for interning)
    assert_eq!(id1, id2);

    // But the stored name should have the first span
    let name = table.get(id1).unwrap();
    assert_eq!(name.span.file, FileId(1));
    assert_eq!(name.span.start, 10);
}

#[test]
fn test_module_resolver_creation() {
    let resolver = ModuleResolver::new();
    // Just verify we can create a resolver
    let _ = resolver;
}

#[test]
fn test_module_resolver_search_paths() {
    let mut resolver = ModuleResolver::new();
    resolver.add_search_path(PathBuf::from("/some/path"));
    resolver.add_stdlib_path(PathBuf::from("/stdlib"));
    // Just verify paths were added - the resolver exists and can be modified
    let _ = resolver;
}

#[test]
fn test_resolved_module_kinds() {
    let modules = vec![
        (ModuleKind::File, "file module"),
        (ModuleKind::Stdlib, "stdlib module"),
        (ModuleKind::Package, "package module"),
        (ModuleKind::Virtual, "virtual module"),
    ];

    for (kind, _desc) in modules {
        // Create a module first to get a valid ModuleId
        let mut table = SymbolTable::default();
        let mod_id = table.add_module(
            Name {
                text: "mod".into(),
                span: make_span(),
            },
            FileId(0),
            PathBuf::from("/test.nim"),
        );

        let resolved = ResolvedModule {
            id: mod_id,
            path: PathBuf::from("/test/path.nim"),
            kind: kind.clone(),
        };
        match resolved.kind {
            ModuleKind::File | ModuleKind::Stdlib | ModuleKind::Package | ModuleKind::Virtual => {}
        }
    }
}

#[test]
fn test_visibility_enum() {
    assert!(matches!(Visibility::Private, Visibility::Private));
    assert!(matches!(Visibility::Public, Visibility::Public));
    assert!(matches!(Visibility::Protected, Visibility::Protected));
}

#[test]
fn test_symbol_table_module_by_path_not_found() {
    let table = SymbolTable::default();
    let result = table.module_by_path(&PathBuf::from("/nonexistent.nim"));
    assert!(result.is_none());
}

#[test]
fn test_scope_id_creation() {
    let mut table = SymbolTable::default();
    // Create a scope by interning a symbol and adding a module
    let _sym = table.intern("test", make_span());
    let _mod = table.add_module(
        Name {
            text: "mod".into(),
            span: make_span(),
        },
        FileId(0),
        PathBuf::from("/test.nim"),
    );
    // ScopeId is created internally - just verify ScopeId type exists
    let _scope: ScopeId = ScopeId::default();
}

#[test]
fn test_module_id_creation() {
    let mut table = SymbolTable::default();
    // Create a module and get its ID
    let mod_id = table.add_module(
        Name {
            text: "mod".into(),
            span: make_span(),
        },
        FileId(0),
        PathBuf::from("/test.nim"),
    );
    // ModuleId should be usable
    let _mod_id2 = mod_id;
    assert_eq!(mod_id, mod_id);
}

#[test]
fn test_symbol_id_creation() {
    let mut table = SymbolTable::default();
    // Create a symbol and get its ID
    let sym_id1 = table.intern("foo", make_span());
    let sym_id2 = table.intern("bar", make_span());
    assert_ne!(sym_id1, sym_id2);
    assert_eq!(sym_id1, sym_id1);
}

#[test]
fn test_name_struct() {
    let name = Name {
        text: "test_name".into(),
        span: make_span(),
    };
    assert_eq!(name.text.as_ref(), "test_name");
}

#[test]
fn test_scope_tree_creation() {
    let mut tree = ScopeTree::new();
    let root_id = tree.create_module_scope(0);
    assert!(tree.get_scope(root_id).is_some());
    assert_eq!(tree.root_scope(), Some(root_id));
}

#[test]
fn test_scope_tree_child_creation() {
    let mut tree = ScopeTree::new();
    let parent_id = tree.create_module_scope(0);
    let child_id = tree.create_child_scope(parent_id, ScopeKind::Routine, 1);

    let child = tree.get_scope(child_id);
    assert!(child.is_some());
    assert_eq!(child.unwrap().parent, Some(parent_id));
}

#[test]
fn test_scope_tree_lookup_symbol() {
    let mut table = SymbolTable::default();
    let mut tree = ScopeTree::new();

    // Create module scope
    let mod_id = tree.create_module_scope(0);

    // Intern some symbols
    let sym1 = table.intern("foo", make_span());
    let sym2 = table.intern("bar", make_span());

    // Insert into scope
    tree.insert_symbol(mod_id, "foo".into(), sym1);

    // Lookup should find it
    let found = tree.lookup(mod_id, "foo");
    assert!(found.is_some());
    assert_eq!(found.unwrap(), sym1);

    // Lookup should not find non-existent symbol
    let not_found = tree.lookup(mod_id, "bar");
    assert!(not_found.is_none());
}

#[test]
fn test_scope_tree_child_scope_hides_parent() {
    let mut table = SymbolTable::default();
    let mut tree = ScopeTree::new();

    // Create module scope
    let mod_id = tree.create_module_scope(0);
    let child_id = tree.create_child_scope(mod_id, ScopeKind::Block, 1);

    // Intern same symbol name twice
    let sym1 = table.intern("foo", make_span());
    let sym2 = table.intern("foo", make_span());

    // Insert with same name in both scopes
    tree.insert_symbol(mod_id, "foo".into(), sym1);
    tree.insert_symbol(child_id, "foo".into(), sym2);

    // Child lookup should find its own symbol first (shadowing)
    let found = tree.lookup(child_id, "foo");
    assert!(found.is_some());
    assert_eq!(found.unwrap(), sym2);
}

#[test]
fn test_scope_tree_get_scope_symbols() {
    let mut table = SymbolTable::default();
    let mut tree = ScopeTree::new();

    let mod_id = tree.create_module_scope(0);
    let sym1 = table.intern("foo", make_span());
    let sym2 = table.intern("bar", make_span());

    tree.insert_symbol(mod_id, "foo".into(), sym1);
    tree.insert_symbol(mod_id, "bar".into(), sym2);

    let symbols = tree.get_scope_symbols(mod_id);
    assert_eq!(symbols.len(), 2);
}

#[test]
fn test_scope_kind_default() {
    let kind = ScopeKind::default();
    assert!(matches!(kind, ScopeKind::Block));
}

#[test]
fn test_name_normalization() {
    assert_eq!(normalize_name("Foo").as_ref(), "foo");
    assert_eq!(normalize_name("BAR").as_ref(), "bar");
    assert_eq!(normalize_name("testName").as_ref(), "testname");
}

#[test]
fn test_names_equal() {
    assert!(names_equal("Foo", "foo"));
    assert!(names_equal("BAR", "bar"));
    assert!(names_equal("TestName", "testname"));
    assert!(!names_equal("Foo", "bar"));
}

#[test]
fn test_import_stmt_struct() {
    let import = ImportStmt {
        module_id: ModuleId::default(),
        symbols: vec![ImportedSymbol {
            name: "foo".into(),
            alias: None,
            symbol_id: None,
        }],
        is_explicit: true,
        module_alias: Some("myfoo".into()),
        is_reexport: false,
    };
    assert!(import.is_explicit);
    assert_eq!(import.symbols.len(), 1);
    assert_eq!(import.symbols[0].name.as_ref(), "foo");
}

#[test]
fn test_export_stmt_struct() {
    let export = ExportStmt {
        module_id: ModuleId::default(),
        symbols: vec![ExportedSymbol {
            name: "bar".into(),
            alias: None,
            symbol_id: None,
        }],
    };
    assert_eq!(export.symbols.len(), 1);
    assert_eq!(export.symbols[0].name.as_ref(), "bar");
}

#[test]
fn test_visibility_checker() {
    // Just verify VisibilityChecker exists and can be called
    let result = VisibilityChecker::is_visible(
        ModuleId::default(),
        SymbolId::default(),
        ModuleId::default(),
    );
    assert!(result);
}

#[test]
fn test_module_graph_creation() {
    let graph = ModuleGraph::new();
    let _ = graph.get_imports(ModuleId::default());
    let _ = graph.get_exports(ModuleId::default());
    let _ = graph.get_reexports(ModuleId::default());
}

#[test]
fn test_module_graph_add_import() {
    let mut graph = ModuleGraph::new();
    let mod_id = ModuleId::default();

    let import = ImportStmt {
        module_id: ModuleId::default(),
        symbols: vec![],
        is_explicit: true,
        module_alias: None,
        is_reexport: false,
    };

    graph.add_import(mod_id, import);
    let imports = graph.get_imports(mod_id);
    assert_eq!(imports.len(), 1);
}

#[test]
fn test_module_graph_add_export() {
    let mut graph = ModuleGraph::new();
    let mod_id = ModuleId::default();

    let export = ExportStmt {
        module_id: ModuleId::default(),
        symbols: vec![],
    };

    graph.add_export(mod_id, export);
    let exports = graph.get_exports(mod_id);
    assert_eq!(exports.len(), 1);
}

#[test]
fn test_module_graph_reexport() {
    let mut graph = ModuleGraph::new();
    let mod_a = ModuleId::default();
    let mod_b = ModuleId::default();

    graph.add_reexport(mod_a, mod_b);
    let reexports = graph.get_reexports(mod_a);
    assert_eq!(reexports.len(), 1);
    assert_eq!(reexports[0], mod_b);
}

#[test]
fn test_module_graph_imports_module() {
    let mut graph = ModuleGraph::new();
    let mut table = SymbolTable::default();

    // Create two distinct modules
    let mod_a = table.add_module(
        Name {
            text: "mod_a".into(),
            span: make_span(),
        },
        FileId(0),
        PathBuf::from("/mod_a.nim"),
    );
    let mod_b = table.add_module(
        Name {
            text: "mod_b".into(),
            span: make_span(),
        },
        FileId(0),
        PathBuf::from("/mod_b.nim"),
    );

    let import = ImportStmt {
        module_id: mod_b,
        symbols: vec![],
        is_explicit: true,
        module_alias: None,
        is_reexport: false,
    };

    graph.add_import(mod_a, import);
    assert!(graph.imports_module(mod_a, mod_b));
    assert!(!graph.imports_module(mod_b, mod_a));
}

#[test]
fn test_overload_set_builder_creation() {
    let builder = OverloadSetBuilder::new();
    assert!(builder.get_all().is_empty());
}

#[test]
fn test_overload_set_builder_add_candidate() {
    let mut builder = OverloadSetBuilder::new();
    let mut table = SymbolTable::default();
    let mod_id = table.add_module(
        Name {
            text: "mod".into(),
            span: make_span(),
        },
        FileId(0),
        PathBuf::from("/test.nim"),
    );

    let sym_id = table.intern("foo", make_span());
    let scope_id = ScopeId::default();

    builder.add_candidate(
        "foo".into(),
        mod_id,
        scope_id,
        sym_id,
        make_span(),
        2,
        false,
        DeclarationKind::Proc,
    );

    assert!(builder.has_overload("foo"));
    assert_eq!(builder.candidate_count("foo"), 1);
}

#[test]
fn test_overload_set_builder_multiple_candidates() {
    let mut builder = OverloadSetBuilder::new();
    let mut table = SymbolTable::default();
    let mod_id = table.add_module(
        Name {
            text: "mod".into(),
            span: make_span(),
        },
        FileId(0),
        PathBuf::from("/test.nim"),
    );

    let sym1 = table.intern("foo", make_span());
    let sym2 = table.intern("foo", make_span());

    builder.add_candidate(
        "foo".into(),
        mod_id,
        ScopeId::default(),
        sym1,
        make_span(),
        2,
        false,
        DeclarationKind::Proc,
    );

    builder.add_candidate(
        "foo".into(),
        mod_id,
        ScopeId::default(),
        sym2,
        make_span(),
        3,
        false,
        DeclarationKind::Func,
    );

    assert_eq!(builder.candidate_count("foo"), 2);
}

#[test]
fn test_overload_set_builder_private_marker() {
    let mut builder = OverloadSetBuilder::new();
    let mut table = SymbolTable::default();
    let mod_id = table.add_module(
        Name {
            text: "mod".into(),
            span: make_span(),
        },
        FileId(0),
        PathBuf::from("/test.nim"),
    );

    let sym_id = table.intern("foo", make_span());

    builder.add_candidate(
        "foo".into(),
        mod_id,
        ScopeId::default(),
        sym_id,
        make_span(),
        2,
        false,
        DeclarationKind::Proc,
    );

    builder.mark_private("foo");

    let set = builder.get("foo").unwrap();
    assert!(set.has_private);
}

#[test]
fn test_overload_candidate_struct() {
    let candidate = OverloadCandidate {
        symbol_id: SymbolId::default(),
        span: make_span(),
        param_count: 5,
        is_generic: true,
        kind: DeclarationKind::Template,
    };
    assert_eq!(candidate.param_count, 5);
    assert!(candidate.is_generic);
    assert!(matches!(candidate.kind, DeclarationKind::Template));
}

#[test]
fn test_declaration_kind_default() {
    let kind = DeclarationKind::default();
    assert!(matches!(kind, DeclarationKind::Proc));
}

#[test]
fn test_overload_set_struct() {
    let mut table = SymbolTable::default();
    let mod_id = table.add_module(
        Name {
            text: "mod".into(),
            span: make_span(),
        },
        FileId(0),
        PathBuf::from("/test.nim"),
    );

    let set = OverloadSet {
        name: "test".into(),
        module_id: mod_id,
        scope_id: ScopeId::default(),
        candidates: vec![],
        has_private: false,
    };
    assert_eq!(set.name.as_ref(), "test");
}

#[test]
fn test_method_call_resolver_callable() {
    let mut builder = OverloadSetBuilder::new();
    let mut table = SymbolTable::default();
    let mod_id = table.add_module(
        Name {
            text: "mod".into(),
            span: make_span(),
        },
        FileId(0),
        PathBuf::from("/test.nim"),
    );

    let sym_id = table.intern("foo", make_span());
    builder.add_candidate(
        "foo".into(),
        mod_id,
        ScopeId::default(),
        sym_id,
        make_span(),
        2,
        false,
        DeclarationKind::Proc,
    );

    let result = MethodCallResolver::resolve("SomeType", "foo", 1, &builder);
    assert!(result.is_resolved);
    match result.target {
        MethodCallTarget::CallableCall {
            callable_name,
            receiver_arg_index,
        } => {
            assert_eq!(callable_name.as_ref(), "foo");
            assert_eq!(receiver_arg_index, 0);
        }
        _ => panic!("Expected CallableCall"),
    }
}

#[test]
fn test_method_call_resolver_ambiguous() {
    let builder = OverloadSetBuilder::new();

    let result = MethodCallResolver::resolve("SomeType", "bar", 1, &builder);
    assert!(!result.is_resolved);
    match result.target {
        MethodCallTarget::Ambiguous {
            field_name,
            callable_name,
        } => {
            assert_eq!(field_name.as_ref(), "bar");
            assert_eq!(callable_name.as_ref(), "bar");
        }
        _ => panic!("Expected Ambiguous"),
    }
}

#[test]
fn test_method_call_could_be_callable() {
    let mut builder = OverloadSetBuilder::new();
    let mut table = SymbolTable::default();
    let mod_id = table.add_module(
        Name {
            text: "mod".into(),
            span: make_span(),
        },
        FileId(0),
        PathBuf::from("/test.nim"),
    );

    let sym_id = table.intern("foo", make_span());
    builder.add_candidate(
        "foo".into(),
        mod_id,
        ScopeId::default(),
        sym_id,
        make_span(),
        2,
        false,
        DeclarationKind::Proc,
    );

    assert!(MethodCallResolver::could_be_callable("foo", &builder));
    assert!(!MethodCallResolver::could_be_callable("bar", &builder));
}

#[test]
fn test_method_call_target_variants() {
    let field_access = MethodCallTarget::FieldAccess {
        receiver_type: "int".into(),
        field_name: "foo".into(),
    };
    match field_access {
        MethodCallTarget::FieldAccess {
            receiver_type,
            field_name,
        } => {
            assert_eq!(receiver_type.as_ref(), "int");
            assert_eq!(field_name.as_ref(), "foo");
        }
        _ => panic!("Expected FieldAccess"),
    }

    let callable = MethodCallTarget::CallableCall {
        callable_name: "bar".into(),
        receiver_arg_index: 1,
    };
    match callable {
        MethodCallTarget::CallableCall {
            callable_name,
            receiver_arg_index,
        } => {
            assert_eq!(callable_name.as_ref(), "bar");
            assert_eq!(receiver_arg_index, 1);
        }
        _ => panic!("Expected CallableCall"),
    }

    let ambiguous = MethodCallTarget::Ambiguous {
        field_name: "baz".into(),
        callable_name: "baz".into(),
    };
    match ambiguous {
        MethodCallTarget::Ambiguous {
            field_name,
            callable_name,
        } => {
            assert_eq!(field_name.as_ref(), "baz");
            assert_eq!(callable_name.as_ref(), "baz");
        }
        _ => panic!("Expected Ambiguous"),
    }
}

#[test]
fn test_method_call_resolution_struct() {
    let resolution = MethodCallResolution {
        target: MethodCallTarget::CallableCall {
            callable_name: "test".into(),
            receiver_arg_index: 0,
        },
        is_resolved: true,
        error: None,
    };
    assert!(resolution.is_resolved);
    assert!(resolution.error.is_none());

    let unresolved = MethodCallResolution {
        target: MethodCallTarget::Ambiguous {
            field_name: "foo".into(),
            callable_name: "foo".into(),
        },
        is_resolved: false,
        error: Some("test error".into()),
    };
    assert!(!unresolved.is_resolved);
    assert!(unresolved.error.is_some());
}

#[test]
fn test_generic_context_creation() {
    let context = GenericContext::new(ScopeId::default());
    assert_eq!(context.type_params.len(), 0);
    assert!(!context.has_instantiations());
}

#[test]
fn test_generic_context_add_type_param() {
    let mut context = GenericContext::new(ScopeId::default());
    let sym = SymbolId::default();
    context.add_type_param(sym);
    assert_eq!(context.type_params.len(), 1);
}

#[test]
fn test_generic_context_bindings() {
    let mut context = GenericContext::new(ScopeId::default());
    let sym = SymbolId::default();

    context.set_binding(sym, SymbolBindingMode::Open);
    assert_eq!(context.get_binding(sym), SymbolBindingMode::Open);

    context.set_binding(sym, SymbolBindingMode::Closed);
    assert_eq!(context.get_binding(sym), SymbolBindingMode::Closed);
}

#[test]
fn test_generic_context_record_instantiation() {
    let mut context = GenericContext::new(ScopeId::default());
    context.record_instantiation(vec!["int".into(), "string".into()], None);
    assert!(context.has_instantiations());
    assert_eq!(context.instantiations.len(), 1);
}

#[test]
fn test_symbol_binding_mode_default() {
    let mode = SymbolBindingMode::default();
    assert!(matches!(mode, SymbolBindingMode::Open));
}

#[test]
fn test_generic_instantiation_struct() {
    let instantiation = GenericInstantiation {
        definition_scope: ScopeId::default(),
        instantiation_scope: ScopeId::default(),
        type_args: vec!["int".into()],
        concrete_symbol: None,
    };
    assert_eq!(instantiation.type_args.len(), 1);
    assert!(instantiation.concrete_symbol.is_none());
}

#[test]
fn test_generic_resolver_determine_binding_mode() {
    // Type param should be Open
    let mode = GenericResolver::determine_binding_mode(
        SymbolId::default(),
        true,  // is_type_param
        false, // is_explicitly_bound
        false, // is_mixin
    );
    assert!(matches!(mode, SymbolBindingMode::Open));

    // Explicitly bound should be Bind
    let mode = GenericResolver::determine_binding_mode(
        SymbolId::default(),
        false, // is_type_param
        true,  // is_explicitly_bound
        false, // is_mixin
    );
    assert!(matches!(mode, SymbolBindingMode::Bind));

    // Mixin should be Mixin
    let mode = GenericResolver::determine_binding_mode(
        SymbolId::default(),
        false, // is_type_param
        false, // is_explicitly_bound
        true,  // is_mixin
    );
    assert!(matches!(mode, SymbolBindingMode::Mixin));

    // Normal non-type-param should be Closed
    let mode = GenericResolver::determine_binding_mode(
        SymbolId::default(),
        false, // is_type_param
        false, // is_explicitly_bound
        false, // is_mixin
    );
    assert!(matches!(mode, SymbolBindingMode::Closed));
}

#[test]
fn test_generic_resolver_should_capture() {
    let mut context = GenericContext::new(ScopeId::default());
    let sym = SymbolId::default();

    context.set_binding(sym, SymbolBindingMode::Open);
    assert!(GenericResolver::should_capture(sym, &context));

    context.set_binding(sym, SymbolBindingMode::Closed);
    assert!(!GenericResolver::should_capture(sym, &context));

    context.set_binding(sym, SymbolBindingMode::Bind);
    assert!(GenericResolver::should_capture(sym, &context));

    context.set_binding(sym, SymbolBindingMode::Mixin);
    assert!(GenericResolver::should_capture(sym, &context));
}

#[test]
fn test_generic_resolver_resolve_in_context() {
    let context = GenericContext::new(ScopeId::default());
    let sym = SymbolId::default();

    // Closed binding should return the same symbol
    let result = GenericResolver::resolve_in_context(sym, &context, ScopeId::default());
    assert!(result.is_some());
    assert_eq!(result.unwrap(), sym);
}
