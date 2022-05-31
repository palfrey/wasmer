use crate::emscripten_target;
use crate::EmEnv;
use wasmer::ContextMut;

///emscripten: _llvm_bswap_i64
pub fn _llvm_bswap_i64(ctx: ContextMut<'_, EmEnv>, _low: i32, high: i32) -> i32 {
    debug!("emscripten::_llvm_bswap_i64");
    emscripten_target::setTempRet0(ctx, _low.swap_bytes());
    high.swap_bytes()
}
