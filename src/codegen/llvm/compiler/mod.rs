use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::AtomicUsize;

use anyhow::{Result, anyhow};
use inkwell::builder::Builder;
use inkwell::context::Context as InkwellContext;
use inkwell::module::Module;
use inkwell::passes::{PassBuilderOptions, PassManager};
use inkwell::targets::TargetMachine;
use inkwell::types::{BasicType, BasicTypeEnum, PointerType};
use inkwell::values::{FunctionValue, PointerValue};

use crate::codegen::llvm::bridges::prepare_rust_bridges;
use crate::runtime::symbol_registry::SymbolRegistry;
use crate::typecheck::{EnumLayout, TypeInfo};
use ast::nodes::{Expr, Program, Statement};

pub mod expr;
pub mod stmt;
pub mod types;

use self::types::{FunctionContext, OtterType};

pub struct Compiler<'ctx> {
    pub(crate) context: &'ctx InkwellContext,
    pub(crate) builder: Builder<'ctx>,
    pub(crate) module: Module<'ctx>,
    #[allow(dead_code)]
    pub(crate) fpm: PassManager<FunctionValue<'ctx>>,
    pub(crate) symbol_registry: &'static SymbolRegistry,
    pub(crate) string_ptr_type: PointerType<'ctx>,
    pub(crate) declared_functions: HashMap<String, FunctionValue<'ctx>>,
    #[allow(dead_code)]
    pub(crate) expr_types: HashMap<usize, TypeInfo>,
    pub(crate) enum_layouts: HashMap<String, EnumLayout>,
    pub(crate) function_defaults: HashMap<String, Vec<Option<Expr>>>,
    #[allow(dead_code)]
    pub(crate) lambda_counter: AtomicUsize,
    pub cached_ir: Option<String>,
}

use crate::codegen::llvm::config::CodegenOptLevel;

impl<'ctx> Compiler<'ctx> {
    pub fn new(
        context: &'ctx InkwellContext,
        module: Module<'ctx>,
        builder: Builder<'ctx>,
        symbol_registry: &'static SymbolRegistry,
        expr_types: HashMap<usize, TypeInfo>,
        enum_layouts: HashMap<String, EnumLayout>,
    ) -> Self {
        let fpm = PassManager::create(&module);

        // fpm.add_instruction_combining_pass();
        // fpm.add_reassociate_pass();
        // fpm.add_gvn_pass();
        // fpm.add_cfg_simplification_pass();
        // fpm.add_basic_alias_analysis_pass();
        // fpm.add_promote_memory_to_register_pass();
        // fpm.initialize();

        let string_ptr_type = context.ptr_type(inkwell::AddressSpace::default());

        Self {
            context,
            builder,
            module,
            fpm,
            symbol_registry,
            string_ptr_type,
            declared_functions: HashMap::new(),
            expr_types,
            enum_layouts,
            function_defaults: HashMap::new(),
            lambda_counter: AtomicUsize::new(0),
            cached_ir: None,
        }
    }

    pub fn lower_program(&mut self, program: &Program, _require_main: bool) -> Result<()> {
        self.compile_module(program)
    }

    pub(crate) fn expr_type(&self, expr: &Expr) -> Option<&TypeInfo> {
        let id = expr as *const Expr as usize;
        self.expr_types.get(&id)
    }

    pub(crate) fn enum_layout(&self, name: &str) -> Option<&EnumLayout> {
        self.enum_layouts.get(name)
    }

    pub fn compile_module(&mut self, program: &Program) -> Result<()> {
        // Prepare Rust bridges
        let _libraries = prepare_rust_bridges(program, self.symbol_registry)?;

        // First pass: register all functions and types
        for statement in &program.statements {
            match statement.as_ref() {
                Statement::Function(func) => {
                    self.register_function_prototype(func.as_ref())?;
                }
                Statement::Struct { name, fields, methods, .. } => {
                    // Create opaque struct type
                    let struct_type = self.context.opaque_struct_type(name);
                    
                    // Map field types
                    let mut field_types = Vec::new();
                    for (_, ty) in fields {
                        field_types.push(self.map_ast_type(ty.as_ref())?);
                    }
                    
                    // Set body
                    struct_type.set_body(&field_types, false);
                    
                    // Register methods
                    for method in methods {
                        let mut method_func = method.as_ref().clone();
                        method_func.name = format!("{}_{}", name, method_func.name);
                        
                        self.register_function_prototype(&method_func)?;
                    }
                }
                _ => {}
            }
        }

        // Second pass: compile function bodies
        for statement in &program.statements {
            if let Statement::Function(func) = statement.as_ref() {
                self.compile_function(func.as_ref())?;
            }
        }

        // Verify module
        if let Err(e) = self.module.verify() {
            self.module.print_to_stderr();
            return Err(anyhow!("Module verification failed: {}", e));
        }

        Ok(())
    }

    /// Declare an external function from the symbol registry
    fn declare_external_function(
        &mut self,
        name: &str,
        ffi_func: &crate::runtime::symbol_registry::FfiFunction,
    ) -> Result<FunctionValue<'ctx>> {
        use crate::runtime::symbol_registry::FfiType;

        // Map FFI types to LLVM types
        let map_type = |ffi_ty: &FfiType| -> BasicTypeEnum<'ctx> {
            match ffi_ty {
                FfiType::Unit => self.context.i8_type().into(), // Unit as i8
                FfiType::Bool => self.context.bool_type().into(),
                FfiType::I32 => self.context.i32_type().into(),
                FfiType::I64 => self.context.i64_type().into(),
                FfiType::F64 => self.context.f64_type().into(),
                FfiType::Str => self.string_ptr_type.into(),
                FfiType::Opaque | FfiType::List | FfiType::Map => self.context.i64_type().into(), // Handles as i64
            }
        };

        // Map parameter types
        let param_types: Vec<inkwell::types::BasicMetadataTypeEnum> = ffi_func
            .signature
            .params
            .iter()
            .map(|ty| map_type(ty).into())
            .collect();

        // Create function type
        let fn_type = match &ffi_func.signature.result {
            FfiType::Unit => self.context.void_type().fn_type(&param_types, false),
            result_ty => {
                let ret_type = map_type(result_ty);
                ret_type.fn_type(&param_types, false)
            }
        };

        // Declare the function using the symbol name (not the user-facing name)
        let function = self.module.add_function(&ffi_func.symbol, fn_type, None);

        // Cache it under the user-facing name
        self.declared_functions.insert(name.to_string(), function);

        Ok(function)
    }

    fn get_or_declare_ffi_function(&mut self, name: &str) -> Result<FunctionValue<'ctx>> {
        if let Some(function) = self.declared_functions.get(name) {
            return Ok(*function);
        }

        let ffi_func = self
            .symbol_registry
            .resolve(name)
            .ok_or_else(|| anyhow!("{name} not found in registry"))?;
        self.declare_external_function(name, &ffi_func)
    }

    /// Map AST type to LLVM type
    fn map_ast_type(&self, ty: &ast::nodes::Type) -> Result<BasicTypeEnum<'ctx>> {
        match ty {
            ast::nodes::Type::Simple(name) => match name.as_str() {
                "int" | "i64" => Ok(self.context.i64_type().into()),
                "float" | "f64" => Ok(self.context.f64_type().into()),
                "bool" => Ok(self.context.bool_type().into()),
                "string" | "str" => Ok(self.string_ptr_type.into()),
                "void" | "unit" => Ok(self.context.i8_type().into()), // Unit as i8 (or void for return)
                "list" | "List" => Ok(self.context.i64_type().into()), // Opaque handle
                "map" | "Map" => Ok(self.context.i64_type().into()),  // Opaque handle
                _ => Ok(self.context.i64_type().into()), // Default to i64/opaque for unknown types
            },
            ast::nodes::Type::Generic { .. } => Ok(self.context.i64_type().into()), // Treat generics as opaque handles
        }
    }

    fn otter_type_from_basic_type(&self, ty: BasicTypeEnum<'ctx>) -> OtterType {
        match ty {
            BasicTypeEnum::IntType(int_type) if int_type == self.context.bool_type() => {
                OtterType::Bool
            }
            BasicTypeEnum::IntType(int_type) if int_type == self.context.i32_type() => {
                OtterType::I32
            }
            BasicTypeEnum::IntType(int_type) if int_type == self.context.i64_type() => {
                OtterType::I64
            }
            BasicTypeEnum::IntType(int_type) if int_type == self.context.i8_type() => {
                OtterType::Unit
            }
            BasicTypeEnum::FloatType(float_type) if float_type == self.context.f64_type() => {
                OtterType::F64
            }
            BasicTypeEnum::PointerType(ptr_type) if ptr_type == self.string_ptr_type => {
                OtterType::Str
            }
            _ => OtterType::Opaque,
        }
    }

    fn register_function_prototype(&mut self, func: &ast::nodes::Function) -> Result<()> {
        let ret_type: Option<BasicTypeEnum> = if let Some(ret_ty) = &func.ret_ty {
            let mapped_ty = self.map_ast_type(ret_ty.as_ref())?;
            // Check if it's effectively unit/void
            if let ast::nodes::Type::Simple(name) = ret_ty.as_ref() {
                if name == "void" || name == "unit" {
                    None
                } else {
                    Some(mapped_ty)
                }
            } else {
                Some(mapped_ty)
            }
        } else {
            None
        };

        let mut param_types = Vec::new();
        for param in &func.params {
            if let Some(ty) = &param.as_ref().ty {
                param_types.push(self.map_ast_type(ty.as_ref())?.into());
            } else {
                // Default to i64 if no type specified
                param_types.push(self.context.i64_type().into());
            }
        }

        let fn_type = if let Some(rt) = ret_type {
            rt.fn_type(&param_types, false)
        } else {
            self.context.void_type().fn_type(&param_types, false)
        };

        let function = self.module.add_function(&func.name, fn_type, None);
        self.declared_functions.insert(func.name.clone(), function);

        // Store default values
        let defaults: Vec<Option<Expr>> = func
            .params
            .iter()
            .map(|p| p.as_ref().default.as_ref().map(|e| e.as_ref().clone()))
            .collect();
        self.function_defaults.insert(func.name.clone(), defaults);

        Ok(())
    }

    fn compile_function(&mut self, func: &ast::nodes::Function) -> Result<()> {
        let function = self
            .declared_functions
            .get(&func.name)
            .ok_or_else(|| anyhow!("Function {} not found", func.name))?;

        let entry = self.context.append_basic_block(*function, "entry");
        self.builder.position_at_end(entry);

        let mut ctx = FunctionContext::new();

        // Bind arguments
        for (i, param) in func.params.iter().enumerate() {
            let arg_val = function.get_nth_param(i as u32).unwrap();
            let param_name = &param.as_ref().name;

            // Determine type from AST or default to I64
            let llvm_type = if let Some(ty) = &param.as_ref().ty {
                self.map_ast_type(ty.as_ref())?
            } else {
                self.context.i64_type().into()
            };

            let otter_type = self.otter_type_from_basic_type(llvm_type);

            // Allocate stack space for parameter
            let alloca = self.create_entry_block_alloca(
                *function,
                param_name.as_ref().as_str(),
                otter_type,
            )?;
            self.builder.build_store(alloca, arg_val)?;

            // Add to context
            ctx.insert(
                param_name.as_ref().to_string(),
                crate::codegen::llvm::compiler::types::Variable {
                    ptr: alloca,
                    ty: otter_type,
                },
            );
        }

        // Compile body
        self.lower_block(func.body.as_ref(), *function, &mut ctx)?;

        // Add implicit return if needed
        if self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_terminator())
            .is_none()
        {
            if func.ret_ty.is_none() {
                self.builder.build_return(None)?;
            } else {
                let ret_ty = func.ret_ty.as_ref().unwrap();
                let llvm_ty = self.map_ast_type(ret_ty.as_ref())?;
                
                let default_val: inkwell::values::BasicValueEnum = match llvm_ty {
                    BasicTypeEnum::IntType(t) => t.const_zero().into(),
                    BasicTypeEnum::FloatType(t) => t.const_zero().into(),
                    BasicTypeEnum::PointerType(t) => t.const_null().into(),
                    BasicTypeEnum::StructType(t) => t.const_zero().into(), // For unit/void which might be mapped to struct or i8
                    BasicTypeEnum::ArrayType(t) => t.const_zero().into(),
                    BasicTypeEnum::VectorType(t) => t.const_zero().into(),
                    _ => unimplemented!("Unsupported return type for default value generation"),
                };
                
                self.builder.build_return(Some(&default_val))?;
            }
        }

        Ok(())
    }

    /// Creates a new stack allocation instruction in the entry block of the function.
    pub(super) fn create_entry_block_alloca(
        &self,
        function: FunctionValue<'ctx>,
        name: &str,
        otter_type: OtterType,
    ) -> Result<PointerValue<'ctx>> {
        let builder = self.context.create_builder();
        let entry_block = function.get_first_basic_block().unwrap();

        // If the entry block has terminators, insert before the first one.
        // Otherwise, insert at the end of the block.
        match entry_block.get_first_instruction() {
            Some(first_instr) => builder.position_before(&first_instr),
            None => builder.position_at_end(entry_block),
        }

        let llvm_type: BasicTypeEnum = match otter_type {
            OtterType::I64 => self.context.i64_type().into(),
            OtterType::F64 => self.context.f64_type().into(),
            OtterType::Bool => self.context.bool_type().into(),
            OtterType::Str => self.string_ptr_type.into(),
            OtterType::Unit => self.context.i8_type().into(), // Unit as i8
            OtterType::List => self.context.i64_type().into(), // Opaque handle
            OtterType::Map => self.context.i64_type().into(), // Opaque handle
            OtterType::Opaque => self.context.i64_type().into(), // Opaque handle
            _ => self.context.i64_type().into(),              // Default to i64
        };

        Ok(builder.build_alloca(llvm_type, name)?)
    }

    pub(super) fn run_default_passes(
        &self,
        level: CodegenOptLevel,
        _enable_pgo: bool,
        _pgo_profile_file: Option<&Path>,
        _inline_threshold: Option<u32>,
        target_machine: &TargetMachine,
    ) {
        if matches!(level, CodegenOptLevel::None) {
            return;
        }

        // Simplified pass running for now
        let pass_options = PassBuilderOptions::create();
        pass_options.set_loop_interleaving(true);
        pass_options.set_loop_vectorization(true);

        let _ = self
            .module
            .run_passes("default<O2>", target_machine, pass_options);
    }

    /// Build a heap allocation using the GC
    pub fn build_heap_alloc(&mut self, size: u64) -> Result<PointerValue<'ctx>> {
        let alloc_func = self.get_or_declare_ffi_function("gc.alloc")?;

        let size_val = self.context.i64_type().const_int(size, false);
        let call = self
            .builder
            .build_call(alloc_func, &[size_val.into()], "alloc")?;

        Ok(call
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value())
    }

    /// Register a root with the GC
    pub fn build_gc_add_root(&mut self, ptr: PointerValue<'ctx>) -> Result<()> {
        let add_root_func = self.get_or_declare_ffi_function("gc.add_root")?;

        let ptr_as_int = self
            .builder
            .build_ptr_to_int(ptr, self.context.i64_type(), "ptr_int")?;
        self.builder
            .build_call(add_root_func, &[ptr_as_int.into()], "")?;

        Ok(())
    }

    /// Remove a root from the GC
    pub fn build_gc_remove_root(&mut self, ptr: PointerValue<'ctx>) -> Result<()> {
        let remove_root_func = self.get_or_declare_ffi_function("gc.remove_root")?;

        let ptr_as_int = self
            .builder
            .build_ptr_to_int(ptr, self.context.i64_type(), "ptr_int")?;
        self.builder
            .build_call(remove_root_func, &[ptr_as_int.into()], "")?;

        Ok(())
    }
}
