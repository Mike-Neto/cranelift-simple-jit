use cranelift::{
    codegen::{
        entity::EntityRef,
        ir::types::I64,
        ir::{AbiParam, InstBuilder},
        settings, verifier,
    },
    frontend::{FunctionBuilder, FunctionBuilderContext, Variable},
    jit::{JITBuilder, JITModule},
    module::{Linkage, Module},
    object::{ObjectBuilder, ObjectModule},
};

pub fn build_and_run_jit(operands: Vec<i64>) -> Result<(), String> {
    let flag_builder = settings::builder();
    let isa_builder = cranelift::native::builder()
        .map_err(|msg| format!("failed to create isa_builder: {msg}"))?;
    let isa = isa_builder
        .finish(settings::Flags::new(flag_builder))
        .map_err(|err| format!("failed to create isa: {err}"))?;
    let builder = JITBuilder::with_isa(isa, cranelift::module::default_libcall_names());
    let mut module = JITModule::new(builder);
    let mut codegen_ctx = module.make_context();
    codegen_ctx.func.signature.params.push(AbiParam::new(I64));
    codegen_ctx.func.signature.returns.push(AbiParam::new(I64));

    let mut builder_ctx = FunctionBuilderContext::new();
    let mut builder = FunctionBuilder::new(&mut codegen_ctx.func, &mut builder_ctx);

    let x = Variable::new(0);
    let y = Variable::new(1);
    let a = Variable::new(2);
    builder.declare_var(x, I64);
    builder.declare_var(y, I64);
    builder.declare_var(a, I64);

    let block0 = builder.create_block();
    builder.append_block_params_for_function_params(block0);
    builder.switch_to_block(block0);
    builder.seal_block(block0);

    let tmp = builder.block_params(block0)[0];
    let tmp = builder
        .ins()
        .load(I64, cranelift::codegen::ir::MemFlags::trusted(), tmp, 0);
    builder.def_var(x, tmp);

    // First value used in the JIT logic
    let tmp = builder.ins().iconst(I64, operands[0]);
    builder.def_var(y, tmp);

    let arg1 = builder.use_var(x);
    let arg2 = builder.use_var(y);
    let tmp = builder.ins().iadd(arg1, arg2);
    builder.def_var(a, tmp);
    builder.ins().return_(&[tmp]);
    builder.finalize();

    let flags = settings::Flags::new(settings::builder());
    verifier::verify_function(&codegen_ctx.func, &flags)
        .map_err(|err| format!("verifications failed: {err}"))?;
    println!("{}", codegen_ctx.func.display());

    let func = module
        .declare_function("adder", Linkage::Export, &codegen_ctx.func.signature)
        .map_err(|e| format!("failed to declare function: {e}"))?;
    module
        .define_function(func, &mut codegen_ctx)
        .map_err(|e| format!("failed to define function: {e}"))?;
    module.clear_context(&mut codegen_ctx);
    module.finalize_definitions().unwrap();
    let code = module.get_finalized_function(func);
    let func = unsafe { core::mem::transmute::<*const u8, for<'a> fn(&'a i64) -> i64>(code) };

    // Second value used as passing as a argument to the JITed function
    println!("result: {}", func(&operands[1]));

    Ok(())
}

pub fn build_native(operands: Vec<i64>) -> Result<(), String> {
    let flag_builder = settings::builder();
    let isa_builder = cranelift::native::builder()
        .map_err(|msg| format!("failed to create isa_builder: {msg}"))?;
    let isa = isa_builder
        .finish(settings::Flags::new(flag_builder))
        .map_err(|err| format!("failed to create isa: {err}"))?;
    let builder = ObjectBuilder::new(isa, "example", cranelift::module::default_libcall_names())
        .map_err(|err| format!("failed to create builder: {err}"))?;
    let mut module = ObjectModule::new(builder);
    let mut codegen_ctx = module.make_context();
    codegen_ctx.func.signature.params.push(AbiParam::new(I64));
    codegen_ctx.func.signature.returns.push(AbiParam::new(I64));

    let mut builder_ctx = FunctionBuilderContext::new();
    let mut builder = FunctionBuilder::new(&mut codegen_ctx.func, &mut builder_ctx);

    let x = Variable::new(0);
    let y = Variable::new(1);
    let a = Variable::new(2);
    builder.declare_var(x, I64);
    builder.declare_var(y, I64);
    builder.declare_var(a, I64);

    let block0 = builder.create_block();
    builder.append_block_params_for_function_params(block0);
    builder.switch_to_block(block0);
    builder.seal_block(block0);

    let tmp = builder.block_params(block0)[0];
    let tmp = builder
        .ins()
        .load(I64, cranelift::codegen::ir::MemFlags::trusted(), tmp, 0);
    builder.def_var(x, tmp);

    let tmp = builder.ins().iconst(I64, operands[0]);
    builder.def_var(y, tmp);

    let arg1 = builder.use_var(x);
    let arg2 = builder.use_var(y);
    let tmp = builder.ins().iadd(arg1, arg2);
    builder.def_var(a, tmp);
    builder.ins().return_(&[tmp]);
    builder.finalize();

    let flags = settings::Flags::new(settings::builder());
    verifier::verify_function(&codegen_ctx.func, &flags)
        .map_err(|err| format!("verifications failed: {err}"))?;
    println!("{}", codegen_ctx.func.display());

    let func = module
        .declare_function("main", Linkage::Export, &codegen_ctx.func.signature)
        .map_err(|e| format!("failed to declare function: {e}"))?;
    module
        .define_function(func, &mut codegen_ctx)
        .map_err(|e| format!("failed to define function: {e}"))?;
    module.clear_context(&mut codegen_ctx);
    let object = module.finish();

    use std::io::Write;
    let mut file = std::fs::File::create("example.o").unwrap();
    file.write_all(&object.emit().unwrap()).unwrap();

    println!("example.o was written");

    // How can I do this cross platform? I should be calling clang, msvc or gcc right?
    let result = std::process::Command::new("gcc")
        .args(["example.o"])
        .output();

    println!("compilation result: {result:#?}");

    Ok(())
}
