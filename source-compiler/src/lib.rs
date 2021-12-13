use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

use wasmgen;

use backend_wasm;

use projstd;

// #[wasm_bindgen]
// extern {
//     pub fn alert(s: &str);
// }

// #[wasm_bindgen]
// pub fn greet(name: &str) {
//     alert(&format!("Hello, {}!", name));
// }

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = sourcerorLogCallback)]
    pub fn compiler_log(
        context: i32,
        severity: i32,
        location_file: &str, /* main file gets empty string */
        location_start_line: i32,
        location_start_column: i32,
        location_end_line: i32,
        location_end_column: i32,
        message: String,
    );

    #[wasm_bindgen(js_name = sourcerorFetchDepCallback)]
    pub fn fetch_dep(context: i32, name: String) -> js_sys::Promise;
}

pub async fn fetch_dep_proxy(context: i32, name: String) -> Option<String> {
    JsFuture::from(fetch_dep(context, name))
        .await
        .ok()
        .and_then(|x| x.as_string())
}

#[derive(Copy, Clone)]
pub struct MainLogger {
    context: i32,
}

impl MainLogger {
    fn new(context: i32) -> Self {
        Self { context: context }
    }
}

impl projstd::log::Logger for MainLogger {
    fn log<L: projstd::log::Loggable>(&self, content: L) {
        let loc = content.location();
        compiler_log(
            self.context,
            content.severity().code(),
            loc.source.unwrap_or(""),
            loc.start.line,
            loc.start.column,
            loc.end.line,
            loc.end.column,
            content.message(),
        );
    }
}

/**
 * The entry function for compilation.
 * `context` is an opaque value so that the host code can associate our calls to compiler_log() with the correct call to compile().
 * `source_code`: ESTree JSON representation of validated program
 * `import_spec`: list of imports following the import file format
 */
#[wasm_bindgen]
pub async fn compile(context: i32, source_code: String) -> js_sys::Uint8Array {
    // nice console errors in debug mode
    #[cfg(all(debug_assertions, target_arch = "wasm32"))]
    console_error_panic_hook::set_once();

    (|| async {
        use wasmgen::WasmSerialize;

        //let ir_imports = frontend_estree::parse_imports(import_spec, MainLogger::new(context))?;
        let ir_program = frontend_estree::run_frontend(
            source_code,
            move |name| fetch_dep_proxy(context, name),
            MainLogger::new(context),
        )
        .await?;
        let ir_program_opt = ir::opt::optimize_all(ir_program);
        let wasm_module =
            backend_wasm::run_backend(&ir_program_opt, backend_wasm::Options{wasm_tail_call: true, ..backend_wasm::Options::default()});
        let mut receiver = std::vec::Vec::<u8>::new();
        wasm_module.wasm_serialize(&mut receiver);
        Ok(js_sys::Uint8Array::from(receiver.as_slice()))
    })()
    .await
    .unwrap_or_else(|_: ()| js_sys::Uint8Array::new_with_length(0))
}

#[cfg(test)]
mod tests {
    #[test]
    fn gen() -> std::io::Result<()> {
        use crate::wasmgen::*;
        use std::fs::File;
        use std::io::prelude::*;
        let mut module = WasmModule::new_builder().build();
        let functype = FuncType::new(Box::new([ValType::I32]), Box::new([ValType::I32]));
        let (_type_idx, func_idx) = module.register_func(&functype);
        let mut code_builder = CodeBuilder::new(functype);
        {
            let (locals_builder, expr_builder) = code_builder.split();
            expr_builder.local_get(locals_builder.param(0)); // put the 0th parameter onto the stack
            expr_builder.end(); // return
        }
        module.commit_func(func_idx, code_builder);
        module.export_func(func_idx, "test".to_string());
        let mut file = File::create("test.wasm")?;
        let mut receiver = std::vec::Vec::<u8>::new();
        module.wasm_serialize(&mut receiver);
        file.write_all(receiver.as_slice())?;
        Ok(())
    }
}
